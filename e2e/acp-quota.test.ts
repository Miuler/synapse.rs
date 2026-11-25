// Cuota de la cuenta en el chat del agente: se pregunta a `agy` (uno falso en el PATH) o la adjunta el
// agente a la conversación, y se ve junto al contexto y en `/usage`.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { chmodSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { delimiter, join } from 'node:path';
import { startApp, type Session } from './harness';

let s: Session;
const bin = mkdtempSync(join(tmpdir(), 'synapse-e2e-agy-'));
const calls = join(bin, 'llamadas');
const path = process.env.PATH;

beforeAll(async () => {
  const soon = new Date(Date.now() + 2 * 3_600_000).toISOString().replace(/\.\d+Z/, 'Z');
  const usage = {
    command: {
      name: 'usage',
      data: {
        groups: [
          {
            name: 'Gemini Models',
            buckets: [
              { window: 'weekly', remaining_fraction: 0.5835, reset_time: soon },
              { window: '5h', remaining_fraction: 0.97, reset_time: '1970-01-01T00:00:00Z' },
            ],
          },
        ],
      },
    },
  };
  writeFileSync(join(bin, 'usage.json'), JSON.stringify(usage));
  writeFileSync(
    join(bin, 'agy'),
    `#!/bin/sh
if [ "$1" = acp ]; then exec '${process.execPath}' '${join(import.meta.dir, 'fixtures/fake-acp-agent.ts')}'; fi
echo "$*" >> '${calls}'
sleep 1
cat '${join(bin, 'usage.json')}'
`,
  );
  chmodSync(join(bin, 'agy'), 0o755);
  writeFileSync(calls, '');
  process.env.PATH = `${bin}${delimiter}${path}`;
  s = await startApp({ 'nota.md': '# Nota\n', 'flujo.mmd': 'graph TD\n  A --> B\n' }).finally(() => (process.env.PATH = path));
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.editor-header .agent-toggle').click();
  await s.browser.waitUntil(
    () => s.browser.execute(() => [...document.querySelectorAll('.acp-select option')].some((o) => (o as HTMLOptionElement).value === 'antigravity')) as Promise<boolean>,
    { timeout: 15_000, timeoutMsg: 'agy no aparece entre los agentes' },
  );
  await s.browser.execute(() => {
    const select = document.querySelector('.acp-select') as HTMLSelectElement;
    select.value = 'antigravity';
    select.dispatchEvent(new Event('change', { bubbles: true }));
  });
  await s.browser
    .waitUntil(async () => !(await s.browser.$('.acp-input textarea').getAttribute('disabled')), { timeout: 15_000 })
    .catch(async (e: unknown) => {
      throw new Error(`${e}: ${await text('.synapse-agent-panel')}`);
    });
}, 120_000);
afterAll(async () => {
  await s?.close();
  rmSync(bin, { recursive: true, force: true });
});

const text = (sel: string) => s.browser.execute((q) => document.querySelector(q)?.textContent?.replace(/\s+/g, ' ').trim() ?? '', sel) as Promise<string>;
const asked = () => readFileSync(calls, 'utf8').trim().split('\n').filter(Boolean);
async function say(message: string) {
  await s.browser.$('.acp-input textarea').setValue(message);
  await s.browser.keys('Escape');
  await s.browser.keys('Enter');
}

it('muestra lo que queda del límite más apretado', async () => {
  await s.browser.waitUntil(async () => (await text('.acp-quota')) === '58% sem', { timeout: 10_000, timeoutMsg: 'no salió la cuota' });
  const title = await s.browser.$('.acp-quota').getAttribute('title');
  expect(title).toContain('semanal: queda 58% · se reinicia en');
  // Sin fecha de reinicio (1970) no se inventa una
  expect(title).toMatch(/5 h: queda 97%(\n|$)/);
  expect(asked()).toEqual(['--print /usage --output-format json']);
  // Y su icono va antes del nombre del agente
  const icon = await s.browser.execute(() => {
    const img = document.querySelector('.acp-row-title .agent-icon') as HTMLImageElement | null;
    return [(img?.naturalWidth ?? 0) > 0, img?.nextElementSibling?.classList.contains('agent')];
  });
  expect(icon).toEqual([true, true]);
}, 30_000);

it('/usage añade la cuota al mismo mensaje y reutiliza la respuesta reciente', async () => {
  await say('/usage');
  await s.browser.waitUntil(async () => (await text('.acp-msg.info')).includes('queda 58%'), { timeout: 10_000, timeoutMsg: '/usage no mostró la cuota' });
  const report = await text('.acp-msg.info');
  expect(report).toContain('Uso de esta conversación');
  expect(report).not.toContain('Consultando');
  expect(await s.browser.execute(() => document.querySelectorAll('.acp-msg.info').length)).toBe(1);
  expect(asked()).toHaveLength(1);
}, 30_000);

it('un clic en el indicador vuelve a consultar, y los minutos se configuran', async () => {
  await s.browser.pause(10_500); // margen mínimo entre clics
  await s.browser.execute(() => (document.querySelector('.acp-quota') as HTMLElement).click());
  await s.browser.waitUntil(async () => asked().length === 2, { timeout: 5_000, timeoutMsg: 'el clic no consultó' });
  await s.browser.pause(1_500); // que termine: si no, su respuesta sería más nueva que la siguiente
  await s.browser.$('.synapse-agent-panel button[title="Ajustes de agentes"]').click();
  const minutes = s.browser.$('input[aria-label="Minutos entre consultas de la cuota"]');
  await minutes.waitForExist({ timeout: 5_000 });
  expect(await minutes.getValue()).toBe('3');
  const set = (v: string) =>
    s.browser.execute((value: string) => {
      const el = document.querySelector('input[aria-label="Minutos entre consultas de la cuota"]') as HTMLInputElement;
      el.value = value;
      el.dispatchEvent(new Event('change', { bubbles: true }));
      return `${el.value} ${localStorage.getItem('synapse.agent.quotaMinutes')}`;
    }, v);
  expect(await set('10')).toBe('10 10');
  // Fuera de rango o vacío no se queda tal cual
  expect(await set('0')).toBe('1 1');
  expect(await set('')).toBe('3 3');
  await s.browser.keys('Escape');
  await s.browser.$('.settings-panel').waitForExist({ reverse: true, timeout: 5_000 });
}, 40_000);

it('la cuota que adjunta el agente se muestra como lo que queda, y el contexto sigue aparte', async () => {
  await say('dime tus límites');
  await s.browser.waitUntil(async () => (await text('.acp-quota')) === '5% 5 h', { timeout: 10_000, timeoutMsg: 'no salió la cuota adjunta' });
  expect(await s.browser.$('.acp-quota.low').isExisting()).toBe(true);
  expect(await text('.acp-usage-text')).toBe('1%');
}, 30_000);

it('al reabrir la conversación se ve el contexto sin esperar a la primera respuesta', async () => {
  await s.browser.pause(1_200); // se guarda con un pequeño retraso
  await s.browser.execute(() => {
    (window as any).__antes = true;
    location.reload();
  });
  await s.browser.waitUntil(() => s.browser.execute(() => !(window as any).__antes && document.readyState === 'complete') as Promise<boolean>, { timeout: 15_000 });
  await s.browser.waitUntil(async () => (await text('.acp-usage-text')) === '1%', { timeout: 20_000, timeoutMsg: 'no se recuperó el contexto' });
}, 60_000);
