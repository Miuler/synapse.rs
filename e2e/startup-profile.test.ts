// Perfil del arranque con una bóveda real (solo lectura desde la prueba). Se activa con
// SYNAPSE_REAL_VAULT y conviene usar un build de medición:
//
//   VITE_SYNAPSE_IPC_LOG=1 bun run tauri build --no-bundle
//   E2E_APP=src-tauri/target/release/synapsers SYNAPSE_REAL_VAULT=/ruta/boveda \
//     bun test ./e2e/startup-profile.test.ts
//
// Junta tres fuentes en una línea de tiempo por tramos de 500 ms: CPU por proceso (de /proc),
// bloqueos del hilo de JS del webview (`__synapseLagLog`) y llamadas IPC (`__synapseIpcLog`). Al
// final lista los hilos que más CPU usaron, para saber qué parte del backend o del webview trabaja.
import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { readdirSync, readFileSync } from 'fs';
import { startAppOnVault, type Session } from './harness';

const VAULT = process.env.SYNAPSE_REAL_VAULT;
const CLK_TCK = 100;
const BUCKET_MS = 500;
const PROFILE_MS = 12_000;

interface ProcInfo {
  ppid: number;
  name: string;
  cmd: string;
}

function readProcs(): Map<number, ProcInfo> {
  const info = new Map<number, ProcInfo>();
  for (const pid of readdirSync('/proc').filter((d) => /^\d+$/.test(d))) {
    try {
      const status = readFileSync(`/proc/${pid}/status`, 'utf8');
      const cmd = readFileSync(`/proc/${pid}/cmdline`, 'utf8').replace(/\0/g, ' ');
      const ppid = Number(/PPid:\s+(\d+)/.exec(status)?.[1] ?? 0);
      const name = /Name:\s+(\S+)/.exec(status)?.[1] ?? '?';
      info.set(Number(pid), { ppid, name, cmd });
    } catch {}
  }
  return info;
}

/** PIDs de la app (el proceso synapsers de esta bóveda y sus descendientes) con su nombre. */
function appTree(rootPattern: string): Map<number, string> {
  const info = readProcs();
  const root = [...info.entries()].find(([, p]) => p.cmd.includes(rootPattern) && p.name.startsWith('synapsers'));
  const tree = new Map<number, string>();
  if (!root) return tree;
  const visit = (pid: number) => {
    tree.set(pid, info.get(pid)!.name);
    for (const [child, c] of info) if (c.ppid === pid) visit(child);
  };
  visit(root[0]);
  return tree;
}

/** Ticks de CPU (utime + stime) de cada hilo de un proceso, por id de hilo. */
function threadTicks(pid: number): Map<number, { comm: string; ticks: number }> {
  const out = new Map<number, { comm: string; ticks: number }>();
  let tids: string[] = [];
  try {
    tids = readdirSync(`/proc/${pid}/task`);
  } catch {
    return out;
  }
  for (const tid of tids) {
    try {
      const stat = readFileSync(`/proc/${pid}/task/${tid}/stat`, 'utf8');
      const comm = stat.slice(stat.indexOf('(') + 1, stat.lastIndexOf(')'));
      const fields = stat.slice(stat.lastIndexOf(')') + 2).split(' ');
      out.set(Number(tid), { comm, ticks: Number(fields[11]) + Number(fields[12]) });
    } catch {}
  }
  return out;
}

interface Sample {
  t: number;
  /** Ticks acumulados por proceso (nombre) en el instante t. */
  byProc: Map<string, number>;
}

describe.skipIf(!VAULT)('perfil del arranque', () => {
  let s: Session;
  const samples: Sample[] = [];
  /** Ticks finales por hilo, con el nombre del proceso y del hilo. */
  const threads = new Map<string, number>();
  let t0 = 0;
  let explorerMs = 0;

  beforeAll(async () => {
    t0 = performance.now();
    const pattern = `--vault ${VAULT}`;
    const sampler = setInterval(() => {
      const t = performance.now() - t0;
      const byProc = new Map<string, number>();
      for (const [pid, name] of appTree(pattern)) {
        for (const [tid, th] of threadTicks(pid)) {
          byProc.set(name, (byProc.get(name) ?? 0) + th.ticks);
          threads.set(`${name}/${th.comm}#${tid}`, th.ticks);
        }
      }
      if (byProc.size) samples.push({ t, byProc });
    }, 100);
    s = await startAppOnVault(VAULT!);
    await s.browser.$('.file-tree-item').waitForExist({ timeout: 120_000 });
    explorerMs = performance.now() - t0;
    await Bun.sleep(Math.max(0, PROFILE_MS - explorerMs));
    clearInterval(sampler);
  }, 300_000);

  afterAll(async () => {
    await s?.close();
  });

  it('línea de tiempo del arranque', async () => {
    const { ipc, lag, now } = (await s.browser.execute(() => ({
      ipc: (window as any).__synapseIpcLog ?? [],
      lag: (window as any).__synapseLagLog ?? [],
      now: performance.now(),
    }))) as { ipc: { cmd: string; start: number; ms: number }[]; lag: { t: number; ms: number }[]; now: number };
    // El reloj del webview empieza al cargar la página: se alinea con el de la prueba usando el
    // instante actual de ambos.
    const offset = performance.now() - t0 - now;

    console.log(`explorador visible a los ${(explorerMs / 1000).toFixed(1)} s (reloj de la prueba)`);
    console.log(`página cargada a los ${(offset / 1000).toFixed(1)} s`);
    const procNames = [...new Set(samples.flatMap((x) => [...x.byProc.keys()]))];
    console.log(
      `tramo     ${procNames.map((n) => n.slice(0, 15).padStart(16)).join('')}   bloqueo JS   IPC (salen / en curso)`,
    );
    const at = (t: number) => samples.findLast((x) => x.t <= t) ?? samples[0];
    for (let b = 0; b < PROFILE_MS; b += BUCKET_MS) {
      const a = at(b);
      const z = at(b + BUCKET_MS);
      if (!a || !z || z.t <= a.t) continue;
      const secs = (z.t - a.t) / 1000;
      const cpu = procNames.map((n) => {
        const pct = (((z.byProc.get(n) ?? 0) - (a.byProc.get(n) ?? 0)) / CLK_TCK / secs) * 100;
        return `${pct.toFixed(0)}%`.padStart(16);
      });
      const blocked = lag.filter((l) => l.t + offset >= b && l.t + offset < b + BUCKET_MS).reduce((acc, l) => acc + l.ms, 0);
      const started = ipc.filter((e) => e.start + offset >= b && e.start + offset < b + BUCKET_MS).length;
      const inFlight = ipc.filter((e) => e.start + offset < b + BUCKET_MS && e.start + e.ms + offset >= b).length;
      console.log(
        `${(b / 1000).toFixed(1).padStart(4)} s  ${cpu.join('')}   ${`${blocked.toFixed(0)} ms`.padStart(10)}   ${started} / ${inFlight}`,
      );
    }

    const spans = (await s.browser.execute(() => (window as any).__synapseSpanLog ?? [])) as { name: string; start: number; ms: number }[];
    console.log('tramos medidos en el front:');
    for (const sp of spans.sort((x, y) => x.start - y.start)) {
      console.log(`  ${((sp.start + offset) / 1000).toFixed(2)} s  ${`${sp.ms.toFixed(0)} ms`.padStart(8)}  ${sp.name}`);
    }

    const longest = [...lag].sort((x, y) => y.ms - x.ms).slice(0, 8);
    console.log('bloqueos más largos del hilo de JS:');
    for (const l of longest) {
      // El bloqueo empezó ~l.ms antes de que corriera el temporizador: las respuestas IPC que
      // llegaron justo antes son las candidatas a haberlo causado.
      const blockStart = l.t - l.ms;
      const before = ipc
        .filter((e) => e.start + e.ms <= blockStart + 5 && e.start + e.ms >= blockStart - 60)
        .map((e) => e.cmd);
      console.log(`  ${((blockStart + offset) / 1000).toFixed(2)} s  ${l.ms.toFixed(0)} ms  tras: ${[...new Set(before)].join(', ') || '—'}`);
    }

    console.log(`hilos con más CPU en los primeros ${PROFILE_MS / 1000} s:`);
    const byComm = new Map<string, number>();
    for (const [key, ticks] of threads) {
      const comm = key.replace(/#\d+$/, '');
      byComm.set(comm, (byComm.get(comm) ?? 0) + ticks);
    }
    for (const [comm, ticks] of [...byComm].sort((x, y) => y[1] - x[1]).slice(0, 15)) {
      console.log(`  ${comm.padEnd(45)} ${((ticks / CLK_TCK) * 1000).toFixed(0)} ms`);
    }
    expect(samples.length).toBeGreaterThan(0);
  });

  it('las mismas llamadas IPC del arranque, repetidas en reposo', async () => {
    // Si una llamada tarda poco en reposo, lo que costó al arrancar fue la espera por el hilo de JS
    // ocupado; si tarda mucho, el costo está en el backend.
    const res = (await s.browser.executeAsync((done: (r: unknown) => void) => {
      const invoke = (window as any).__TAURI_INTERNALS__.invoke;
      const log = ((window as any).__synapseIpcLog ?? []).slice() as { cmd: string; ms: number; args?: unknown }[];
      (async () => {
        const out: { cmd: string; startup: number; rest: number }[] = [];
        for (const e of log) {
          if (/^(save_|set_|record_|update_|write_|delete_|move_|rename_|create_)/.test(e.cmd)) continue;
          const t = performance.now();
          await invoke(e.cmd, e.args ?? {}).catch(() => null);
          out.push({ cmd: e.cmd, startup: e.ms, rest: performance.now() - t });
        }
        done(out);
      })();
    })) as { cmd: string; startup: number; rest: number }[];
    const byCmd = new Map<string, { n: number; startup: number; rest: number; max: number }>();
    for (const r of res) {
      const acc = byCmd.get(r.cmd) ?? { n: 0, startup: 0, rest: 0, max: 0 };
      acc.n++;
      acc.startup += r.startup;
      acc.rest += r.rest;
      acc.max = Math.max(acc.max, r.rest);
      byCmd.set(r.cmd, acc);
    }
    console.log('comando                          veces   al arrancar   en reposo (máx)');
    for (const [cmd, a] of [...byCmd].sort((x, y) => y[1].rest - x[1].rest)) {
      console.log(
        `  ${cmd.padEnd(32)} ${String(a.n).padStart(4)} ${`${a.startup.toFixed(0)} ms`.padStart(12)} ${`${a.rest.toFixed(0)} ms`.padStart(10)} (${a.max.toFixed(0)})`,
      );
    }
    expect(res.length).toBeGreaterThan(0);
  });
});
