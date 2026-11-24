import { expect, it } from 'bun:test';
import { medianRowPitch } from './row-pitch';

it('usa la distancia más habitual, con fracciones e ignorando filas más altas', () => {
  expect(medianRowPitch([0, 26.449, 52.898, 82, 108.449])).toBeCloseTo(26.449);
  expect(medianRowPitch([10])).toBe(0);
});
