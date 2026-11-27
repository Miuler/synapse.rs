import { expect, it } from 'bun:test';
import { completionNotice } from './completion-notice';

it('incluye la consulta en una línea y recortada', () => {
  expect(completionNotice('Claude', '#2', ' revisa\n  el  índice ', false)).toEqual({
    title: 'Claude · #2',
    body: 'Terminó la consulta: "revisa el índice"',
  });
  expect(completionNotice('Claude', '#2', 'a'.repeat(300), false).body).toBe(`Terminó la consulta: "${'a'.repeat(120)}…"`);
  expect(completionNotice('Claude', '#2', 'x', true).body).toBe('Falló la consulta: "x"');
});

it('con la respuesta final la muestra como texto corrido, y sin ella o si falló, la consulta', () => {
  const answer = '## Hecho\n\n- **Revisé** `mi_archivo.ts` y [la guía](docs/guia.md)\n\n```bash\nls *.md\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n- [x] *listo*';
  expect(completionNotice('Claude', '#2', 'revisa', false, answer).body).toBe('Hecho Revisé mi_archivo.ts y la guía ls *.md a · b 1 · 2 ✅ listo');
  expect(completionNotice('Claude', '#2', 'revisa', false, 'a'.repeat(300)).body).toBe(`${'a'.repeat(240)}…`);
  expect(completionNotice('Claude', '#2', 'revisa', false, '  ').body).toBe('Terminó la consulta: "revisa"');
  expect(completionNotice('Claude', '#2', 'revisa', true, 'a medias').body).toBe('Falló la consulta: "revisa"');
});
