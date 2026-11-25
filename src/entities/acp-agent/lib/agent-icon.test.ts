import { expect, test } from 'bun:test';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { agentIconId, isColoredIcon } from './agent-icon';

const DIR = join(import.meta.dir, '../../../shared/assets/agents');

test('los agentes que ofrece la app tienen icono', () => {
  for (const id of ['claude-acp', 'antigravity', 'github-copilot']) {
    expect(existsSync(join(DIR, `${agentIconId(id)}.svg`))).toBe(true);
  }
});

test('un id desconocido se queda como está', () => {
  expect(agentIconId('mi-agente')).toBe('mi-agente');
  expect(agentIconId('constructor')).toBe('constructor');
});

test('los SVG no traen scripts, y los monocromos siguen el color del texto', () => {
  for (const file of readdirSync(DIR)) {
    const svg = readFileSync(join(DIR, file), 'utf8');
    expect(svg).not.toMatch(/<script|\son\w+=|href=|<foreignObject/i);
    if (!isColoredIcon(file.replace('.svg', ''))) expect(svg).toContain('currentColor');
  }
});
