import { expect, it } from 'bun:test';
import { i18n } from '@shared/lib/i18n';
import type { AgentQuota } from '../model/types';
import { duration, groupsForModel, percent, quotaLevel, quotaMarkdown, quotaTooltip, tightestWindow } from './quota';

const NOW = 1_000_000_000;
const Q: AgentQuota = {
  fetched_at: NOW - 3 * 60_000,
  groups: [
    {
      name: 'Gemini Models',
      models: ['Gemini 4 Neon', 'Gemini 4 Hydrogen'],
      windows: [
        { window: 'weekly', remaining: 0.5835, resets_at: NOW + (5 * 1440 + 22 * 60) * 60_000 },
        { window: '5h', remaining: 0.97, resets_at: null },
      ],
    },
    { name: 'Claude and GPT models', models: ['Claude Opus 5.5'], windows: [{ window: '5h', remaining: 0.05, resets_at: NOW - 1 }] },
  ],
};

it('el grupo es el del modelo elegido, o todos si no se sabe', () => {
  expect(groupsForModel(Q, 'Gemini 4 Hydrogen (Medium)').map((g) => g.name)).toEqual(['Gemini Models']);
  expect(groupsForModel(Q, 'Hydrogen').map((g) => g.name)).toEqual(['Gemini Models']);
  expect(groupsForModel(Q, 'claude-opus-5-5').map((g) => g.name)).toEqual(['Claude and GPT models']);
  expect(groupsForModel(Q, 'otro')).toHaveLength(2);
  expect(groupsForModel(Q, null)).toHaveLength(2);
});

it('enseña el límite más apretado', () => {
  expect(tightestWindow(groupsForModel(Q, 'Hydrogen'))?.window).toBe('weekly');
  expect(tightestWindow(Q.groups)?.remaining).toBe(0.05);
  expect(tightestWindow([])).toBeNull();
  expect([0.5, 0.25, 0.24, 0.1, 0.09].map(quotaLevel)).toEqual(['ok', 'ok', 'warn', 'warn', 'low']);
  expect([1, 0.5835, 0.29, 0.004].map(percent)).toEqual(['100%', '58%', '29%', '0%']);
});

it('textos', () => {
  expect([30_000, 4 * 60_000, 151 * 60_000, 8520 * 60_000].map(duration)).toEqual(['1min', '4min', '2h 31min', '5d 22h']);
  i18n.setLocale('es');
  expect(quotaTooltip(Q, 'agy', NOW)).toContain('semanal: queda 58% · se reinicia en 5d 22h');
  const md = quotaMarkdown(Q, 'agy', NOW);
  expect(md).toContain('hace 3min');
  // Un reinicio ya pasado no se anuncia
  expect(md).toContain('| Claude and GPT models · 5 h | queda 5% | — |');
});
