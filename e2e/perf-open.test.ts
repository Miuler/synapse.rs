// Comandos IPC que el front llama al abrir una bóveda grande, con su duración real.
// En modo dev, invokeTauri (shared/api/tauri-client.ts) anota cada llamada y su duración en
// window.__synapseIpcLog.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

const NOTES = 6000;
let s: Session;

beforeAll(async () => {
  const files: Record<string, string> = {};
  for (let i = 0; i < NOTES; i++) {
    const dir = `Proyectos/Cliente-${i % 25}/Repositorio-${i % 40}/documentacion/arquitectura`;
    files[`${dir}/nota-${i}-con-un-nombre-bastante-largo-para-parecerse-a-la-realidad.md`] = `# Nota ${i}\nContenido.\n`;
  }
  s = await startApp(files);
  await s.browser.$('.file-tree-item').waitForExist({ timeout: 60_000 });
  await s.browser.pause(3000); // dejar terminar la carga de la fachada de Obsidian
}, 300_000);

afterAll(async () => {
  await s?.close();
});

it('lista los comandos IPC pesados de la apertura', async () => {
  // invokeTauri anota cada llamada en window.__synapseIpcLog cuando la app corre en modo dev
  const entries = (await s.browser.execute(
    () => ((window as any).__synapseIpcLog ?? []) as { cmd: string; ms: number }[]
  )) as { cmd: string; ms: number }[];
  const byName = new Map<string, { count: number; ms: number }>();
  for (const e of entries) {
    const acc = byName.get(e.cmd) ?? { count: 0, ms: 0 };
    acc.count++;
    acc.ms += e.ms;
    byName.set(e.cmd, acc);
  }
  const top = [...byName.entries()].sort((a, b) => b[1].ms - a[1].ms).slice(0, 20);
  for (const [name, { count, ms }] of top) console.log(`${name.padEnd(32)} x${count}  ${ms.toFixed(1)} ms`);
  const total = entries.reduce((a, e) => a + e.ms, 0);
  console.log(`total IPC en la apertura: ${total.toFixed(1)} ms en ${entries.length} llamadas`);
  expect(entries.length).toBeGreaterThan(0);
}, 60_000);
