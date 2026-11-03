// Rendimiento real del IPC de Tauri en el webview: cuánto tarda el front en recibir y parsear
// las dos listas de la bóveda (explorador y fachada de Obsidian) con una bóveda grande.
// No necesita plugins: llama a los comandos desde la página y mide con performance.now().
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
}, 300_000);

afterAll(async () => {
  await s?.close();
});

async function measure(command: string): Promise<{ medianMs: number; items: number; kb: number }> {
  return (await s.browser.executeAsync((cmd: string, done: (r: unknown) => void) => {
    const invoke = (window as any).__TAURI_INTERNALS__.invoke as (c: string) => Promise<unknown[]>;
    (async () => {
      const times: number[] = [];
      let items = 0;
      let kb = 0;
      for (let i = 0; i < 7; i++) {
        const t = performance.now();
        const res = await invoke(cmd);
        times.push(performance.now() - t);
        items = res.length;
        if (i === 0) kb = JSON.stringify(res).length / 1024;
      }
      times.sort((a, b) => a - b);
      done({ medianMs: times[3], items, kb });
    })();
  }, command)) as { medianMs: number; items: number; kb: number };
}

it('mide la carga de las dos listas por IPC', async () => {
  const notes = await measure('get_vault_notes');
  const files = await measure('get_vault_files');
  console.log(`get_vault_notes  (explorador): ${notes.items} elementos, ${notes.kb.toFixed(0)} KB, ${notes.medianMs.toFixed(1)} ms`);
  console.log(`get_vault_files  (fachada):    ${files.items} elementos, ${files.kb.toFixed(0)} KB, ${files.medianMs.toFixed(1)} ms`);
  expect(notes.items).toBe(NOTES);
  expect(files.items).toBe(NOTES);
}, 120_000);
