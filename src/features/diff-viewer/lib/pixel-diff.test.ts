import { expect, it } from 'bun:test';
import { pixelDiff } from './pixel-diff';

const px = (...rgba: number[][]) => new Uint8ClampedArray(rgba.flat());

it('marca en magenta solo los píxeles distintos', () => {
  const before = px([255, 255, 255, 255], [0, 0, 0, 255], [10, 10, 10, 255]);
  const after = px([255, 255, 255, 255], [255, 255, 255, 255], [12, 10, 10, 255]);
  const { output, diffCount } = pixelDiff(before, after);
  expect(diffCount).toBe(1);
  expect([...output.slice(4, 8)]).toEqual([255, 0, 119, 255]);
  // Sin cambios: gris atenuado y opaco
  expect(output[0]).toBe(255);
  expect(output[8]).toBeGreaterThan(200);
  expect(output[11]).toBe(255);
});

it('un píxel transparente equivale a blanco', () => {
  const { diffCount } = pixelDiff(px([0, 0, 0, 0]), px([255, 255, 255, 255]));
  expect(diffCount).toBe(0);
});

it('rechaza tamaños distintos', () => {
  expect(() => pixelDiff(px([0, 0, 0, 255]), px())).toThrow();
});
