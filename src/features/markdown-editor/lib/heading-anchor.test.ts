import { describe, expect, it } from 'bun:test';
import { findHeadingLine, normalizeHeadingText, splitNavigationTarget } from './heading-anchor';

describe('heading-anchor', () => {
  it('separa la ruta del ancla y usa la última ancla', () => {
    expect(splitNavigationTarget('docs/Plan.md#Presupuesto anual')).toEqual({ path: 'docs/Plan.md', anchor: 'Presupuesto anual' });
    expect(splitNavigationTarget('Plan.md#Meta#Subtítulo')).toEqual({ path: 'Plan.md', anchor: 'Subtítulo' });
    expect(splitNavigationTarget('Plan.md')).toEqual({ path: 'Plan.md' });
    expect(splitNavigationTarget('#etiqueta')).toEqual({ path: '', anchor: 'etiqueta' });
  });

  it('normaliza el marcado de los títulos', () => {
    expect(normalizeHeadingText('Plan **anual** con `código`')).toBe('plan anual con código');
    expect(normalizeHeadingText('Ver [[Otra nota|la otra]] y [web](https://x.y)')).toBe('ver la otra y web');
    expect(normalizeHeadingText('  Muchos   espacios ')).toBe('muchos espacios');
  });

  it('encuentra el encabezado y no la mención en un párrafo', () => {
    const lines = ['Hablamos del presupuesto aquí.', '', '## Presupuesto', 'Cifras.', '### Gastos fijos ###'];
    expect(findHeadingLine(lines, 'presupuesto')).toBe(2);
    expect(findHeadingLine(lines, 'Gastos Fijos')).toBe(4);
    expect(findHeadingLine(lines, 'inexistente')).toBe(-1);
  });

  it('ignora frontmatter y bloques de código', () => {
    const lines = ['---', '# titulo: falso', '---', '```sh', '# Presupuesto', '```', '# Presupuesto'];
    expect(findHeadingLine(lines, 'Presupuesto')).toBe(6);
  });
});
