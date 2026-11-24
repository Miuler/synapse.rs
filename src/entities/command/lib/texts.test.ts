import { expect, it } from 'bun:test';
import { readFileSync } from 'node:fs';
import { computeCommandScore } from '../../../../node_modules/bits-ui/dist/bits/command/compute-command-score.js';
import { commands } from './en';
import { commandAliases, commandCategory, commandName, type CommandCategory, type CommandId, type CommandLocale } from './texts';

const page = readFileSync(new URL('../../../pages/workspace/ui/WorkspacePage.svelte', import.meta.url), 'utf8');
const registered = [...page.matchAll(/id: "(cmd-[\w-]+)",\s*category: "(\w+)"/g)].map((m) => ({
  id: m[1] as CommandId,
  category: m[2] as CommandCategory,
}));

/** Primer resultado de la paleta en ese idioma, con el mismo cálculo que usa. */
const first = (query: string, locale: CommandLocale) =>
  registered
    .map((c) => ({
      id: c.id,
      score: computeCommandScore(
        `${commandName(c.id, locale)} ${commandCategory(c.category, locale)} `,
        query,
        commandAliases(c.id, c.category, locale),
      ) as number,
    }))
    .sort((a, b) => b.score - a.score)[0].id;

it('cada comando registrado tiene textos y no sobra ninguno', () => {
  expect(registered.map((c) => c.id as string).sort()).toEqual(Object.keys(commands).sort());
});

const CASES: [string, CommandId][] = [
  ['update', 'cmd-app-check-updates'],
  ['upgrade', 'cmd-app-check-updates'],
  ['check for updates', 'cmd-app-check-updates'],
  ['buscar actualizaciones', 'cmd-app-check-updates'],
  ['close tab', 'cmd-close-tab'],
  ['cerrar pestaña', 'cmd-close-tab'],
  ['dark mode', 'cmd-theme-dark'],
  ['modo oscuro', 'cmd-theme-dark'],
  ['save', 'cmd-save-file'],
  ['guardar', 'cmd-save-file'],
  ['reload vault', 'cmd-reload-vault'],
  ['unstage', 'cmd-git-restore-staged-active'],
];
for (const locale of ['en', 'es'] as const) {
  it.each(CASES)(`en «${locale}», «%s» encuentra %s`, (query, id) => {
    expect(first(query, locale)).toBe(id);
  });
}
