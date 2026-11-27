import { describe, expect, it } from 'bun:test';
import { placeWindow } from './aside-window';

const viewport = { w: 1200, h: 800 };

describe('ventana de las preguntas aparte', () => {
  it('sin posición guardada se pone a la izquierda del chat, alineada abajo', () => {
    expect(placeWindow(null, { left: 800, bottom: 780 }, viewport)).toEqual({ x: 368, y: 308, w: 420, h: 460 });
  });

  it('no se pierde fuera de la ventana', () => {
    expect(placeWindow({ x: 5000, y: -30, w: 400, h: 2000 }, null, viewport)).toEqual({ x: 1080, y: 0, w: 400, h: 800 });
    expect(placeWindow(null, { left: 100, bottom: 200 }, viewport)).toMatchObject({ x: 0, y: 0 });
  });
});
