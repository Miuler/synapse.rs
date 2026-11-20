import { describe, expect, it } from 'bun:test';
import { stepViewerZoom } from './index';

describe('stepViewerZoom', () => {
  it('sube y baja por niveles y se queda en los extremos', () => {
    expect(stepViewerZoom(1, 1)).toBe(1.1);
    expect(stepViewerZoom(1, -1)).toBe(0.9);
    expect(stepViewerZoom(2, 1)).toBe(2.25);
    expect(stepViewerZoom(3, 1)).toBe(3);
    expect(stepViewerZoom(0.5, -1)).toBe(0.5);
    // Un zoom fuera de los niveles (p. ej. «ajustar al ancho») va al nivel más cercano en esa dirección
    expect(stepViewerZoom(1.37, 1)).toBe(1.4);
    expect(stepViewerZoom(1.37, -1)).toBe(1.3);
  });
});
