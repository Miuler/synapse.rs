import { describe, expect, it } from 'bun:test';
import { withAlpha } from './theme-colors';

describe('withAlpha', () => {
  it('convierte hex corto, largo y con alfa', () => {
    expect(withAlpha('#f03', 0.5)).toBe('rgba(255, 0, 51, 0.5)');
    expect(withAlpha('#ff0033', 1)).toBe('rgba(255, 0, 51, 1)');
    expect(withAlpha('#ff003380', 0.2)).toBe('rgba(255, 0, 51, 0.2)');
  });
  it('reemplaza la opacidad de rgb() y rgba()', () => {
    expect(withAlpha('rgb(1, 2, 3)', 0.3)).toBe('rgba(1, 2, 3, 0.3)');
    expect(withAlpha('rgba(10 20 30 / 0.9)', 0.1)).toBe('rgba(10, 20, 30, 0.1)');
  });
  it('deja igual un formato que no entiende', () => {
    expect(withAlpha('red', 0.5)).toBe('red');
  });
});
