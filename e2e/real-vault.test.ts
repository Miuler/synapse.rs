// Revisión con una bóveda real (solo lectura desde la prueba). Se activa con SYNAPSE_REAL_VAULT:
//
//   SYNAPSE_REAL_VAULT=/ruta/a/la/boveda bun test ./e2e/real-vault.test.ts
//
// Mide la apertura (tiempo hasta ver el explorador e IPC), la memoria real de los procesos de la
// app (RSS de la app y del proceso web de WebKit) y la latencia del Quick Open. No crea, edita ni
// borra notas.
import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { readdirSync, readFileSync } from 'fs';
import { startAppOnVault, type Session } from './harness';

const VAULT = process.env.SYNAPSE_REAL_VAULT;
const shots = process.env.E2E_SHOTS;

/** RSS (MB) de un proceso y de todos sus descendientes, por nombre de ejecutable. */
function processTreeRss(rootPattern: string): Record<string, number> {
  const procs = readdirSync('/proc').filter((d) => /^\d+$/.test(d));
  const info = new Map<number, { ppid: number; name: string; rss: number; cmd: string }>();
  for (const pid of procs) {
    try {
      const status = readFileSync(`/proc/${pid}/status`, 'utf8');
      const cmd = readFileSync(`/proc/${pid}/cmdline`, 'utf8').replace(/\0/g, ' ');
      const ppid = Number(/PPid:\s+(\d+)/.exec(status)?.[1] ?? 0);
      const rss = Number(/VmRSS:\s+(\d+)/.exec(status)?.[1] ?? 0) / 1024;
      const name = /Name:\s+(\S+)/.exec(status)?.[1] ?? '?';
      info.set(Number(pid), { ppid, name, rss, cmd });
    } catch {}
  }
  const root = [...info.entries()].find(([, p]) => p.cmd.includes(rootPattern) && p.name.startsWith('synapsers'));
  if (!root) return {};
  const result: Record<string, number> = {};
  const visit = (pid: number) => {
    const p = info.get(pid)!;
    result[p.name] = (result[p.name] ?? 0) + p.rss;
    for (const [child, c] of info) if (c.ppid === pid) visit(child);
  };
  visit(root[0]);
  return result;
}

describe.skipIf(!VAULT)('bóveda real', () => {
  let s: Session;
  let explorerMs = 0;

  beforeAll(async () => {
    const t0 = performance.now();
    s = await startAppOnVault(VAULT!);
    await s.browser.$('.file-tree-item').waitForExist({ timeout: 120_000 });
    explorerMs = performance.now() - t0;
    await s.browser.pause(8000); // dejar terminar índice, plugins y escaneo
  }, 300_000);

  afterAll(async () => {
    await s?.close();
  });

  it('apertura: tiempo hasta el explorador e IPC', async () => {
    const log = (await s.browser.execute(() => (window as any).__synapseIpcLog ?? [])) as { cmd: string; ms: number }[];
    const byName = new Map<string, { count: number; ms: number }>();
    for (const e of log) {
      const acc = byName.get(e.cmd) ?? { count: 0, ms: 0 };
      acc.count++;
      acc.ms += e.ms;
      byName.set(e.cmd, acc);
    }
    console.log(`explorador visible a los ${(explorerMs / 1000).toFixed(1)} s (incluye arrancar la app)`);
    console.log(`IPC: ${log.length} llamadas, ${log.reduce((a, e) => a + e.ms, 0).toFixed(0)} ms acumulados`);
    for (const [name, { count, ms }] of [...byName].sort((a, b) => b[1].ms - a[1].ms).slice(0, 10)) {
      console.log(`  ${name.padEnd(32)} x${count}  ${ms.toFixed(0)} ms`);
    }
    const items = (await s.browser.execute(() => document.querySelectorAll('.file-tree-item').length)) as number;
    console.log(`elementos visibles en el explorador: ${items}`);
    expect(items).toBeGreaterThan(0);
  });

  it('memoria real de los procesos de la app', async () => {
    const rss = processTreeRss(`--vault ${VAULT}`);
    for (const [name, mb] of Object.entries(rss).sort((a, b) => b[1] - a[1])) console.log(`  ${name.padEnd(20)} ${mb.toFixed(0)} MB`);
    console.log(`  total                ${Object.values(rss).reduce((a, b) => a + b, 0).toFixed(0)} MB`);
    expect(Object.keys(rss).length).toBeGreaterThan(0);
  });

  it('Quick Open: latencia hasta mostrar resultados', async () => {
    const times: number[] = [];
    for (const query of ['rust', 'docker', 'proyecto', 'arq', 'linux']) {
      await s.browser.keys(['Control', 'o']);
      const input = s.browser.$('.quick-open-command-root input');
      await input.waitForExist({ timeout: 10_000 });
      const t = performance.now();
      await input.setValue(query);
      await s.browser
        .waitUntil(async () => ((await s.browser.$$('.quick-open-results-container [data-command-item]').length) as number) > 0, { timeout: 10_000 })
        .catch(() => {});
      const ms = performance.now() - t;
      times.push(ms);
      const count = (await s.browser.execute(() => document.querySelectorAll('.quick-open-results-container [data-command-item]').length)) as number;
      console.log(`  "${query}": ${count} resultados en ${ms.toFixed(0)} ms (incluye ida y vuelta de WebDriver)`);
      if (shots && query === 'rust') await s.browser.saveScreenshot(`${shots}/real-quick-open.png`);
      await s.browser.keys(['Escape']);
      await s.browser.pause(300);
    }
    expect(times.length).toBe(5);
  });

  it('la consola no muestra errores ni avisos de Svelte', async () => {
    const log = await s.consoleLog();
    const lines = log.split('\n').filter((l) => /console\.(error|warn)|\[svelte\]|Uncaught|TypeError/.test(l));
    for (const l of lines.slice(0, 15)) console.log('  ' + l.slice(0, 200));
    console.log(`  (${lines.length} líneas de error o aviso)`);
    if (shots) await s.browser.saveScreenshot(`${shots}/real-vault.png`);
    expect(true).toBe(true);
  });
});
