import { afterAll, beforeAll, expect, it } from 'bun:test';
import { startApp, type Session } from './harness';

let s: Session;

beforeAll(async () => {
  s = await startApp({
    'bienvenida.md': '# Bienvenida\nHola desde la bóveda de prueba.\n',
    'docs/arquitectura.md': '---\ntags: [rust]\n---\n# Arquitectura\nCapas.\n',
    'docs/notas/idea.md': '# Idea\nUna idea.\n',
  });
}, 120_000);

afterAll(async () => {
  await s?.close();
});

it('arranca la app con la bóveda de prueba', async () => {
  await s.browser.pause(4000);
  await s.browser.saveScreenshot(process.env.E2E_SHOTS ? `${process.env.E2E_SHOTS}/smoke.png` : '/tmp/synapse-e2e-smoke.png');
  const title = await s.browser.getTitle();
  console.log('título de la ventana:', title);
  expect(typeof title).toBe('string');
}, 60_000);
