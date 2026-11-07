import { describe, expect, test } from 'bun:test';
import { hasHeadingFilter } from './heading-filter';

describe('hasHeadingFilter', () => {
  test('detecta el filtro al inicio de un término', () => {
    expect(hasHeadingFilter('hs:prompts')).toBe(true);
    expect(hasHeadingFilter('headings:prompts')).toBe(true);
    expect(hasHeadingFilter('nota hs:"prompts para"')).toBe(true);
    expect(hasHeadingFilter('-hs:borrador')).toBe(true);
  });

  test('ignora el resto de las consultas', () => {
    expect(hasHeadingFilter('hs:')).toBe(false);
    expect(hasHeadingFilter('ths:prompts')).toBe(false);
    expect(hasHeadingFilter('kind:pdf informe')).toBe(false);
  });
});
