import { expect, it } from 'bun:test';
import { readFileSync } from 'node:fs';
import { computeCommandScore } from '../../../../node_modules/bits-ui/dist/bits/command/compute-command-score.js';
import { commandIdsWithKeywords, commandKeywords } from './keywords';

const page = readFileSync(new URL('../../../pages/workspace/ui/WorkspacePage.svelte', import.meta.url), 'utf8');
const commands = [...page.matchAll(/id: "(cmd-[\w-]+)",\s*name: (?:"([^"]+)"|t\('([^']+)'\)),\s*category: (?:"([^"]+)")?/g)].map((m) => ({
  id: m[1],
  value: `${m[2] ?? m[3]} ${m[4] ?? ''} `,
}));

/** Primer resultado de la paleta, con el mismo cálculo que usa. */
const first = (query: string) =>
  commands
    .map((c) => ({ id: c.id, score: computeCommandScore(c.value, query, commandKeywords(c.id)) as number }))
    .sort((a, b) => b.score - a.score)[0].id;

it('todos los comandos tienen sinónimos y no sobra ninguno', () => {
  expect(commandIdsWithKeywords.slice().sort()).toEqual(commands.map((c) => c.id).sort());
});

it.each([
  ['update', 'cmd-app-check-updates'],
  ['upgrade', 'cmd-app-check-updates'],
  ['check for updates', 'cmd-app-check-updates'],
  ['close tab', 'cmd-close-tab'],
  ['dark mode', 'cmd-theme-dark'],
  ['save', 'cmd-save-file'],
  ['reload vault', 'cmd-reload-vault'],
  ['unstage', 'cmd-git-restore-staged-active'],
  // En español sigue igual
  ['buscar actualizaciones', 'cmd-app-check-updates'],
  ['cerrar pestaña', 'cmd-close-tab'],
  ['guardar', 'cmd-save-file'],
])('«%s» encuentra %s', (query, id) => {
  expect(first(query)).toBe(id);
});
