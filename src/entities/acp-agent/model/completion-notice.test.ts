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
