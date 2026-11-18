import { describe, expect, it } from 'bun:test';
import { unscaledTextEditorStyle } from './sharp-text-editor';

const base = { left: '100px', top: '50px', width: '100px', height: '20px', fontSize: '20px', maxWidth: '', maxHeight: '300px' };

describe('unscaledTextEditorStyle', () => {
  it('sin escala no cambia nada', () => {
    expect(unscaledTextEditorStyle({ ...base, transform: 'translate(0px) scale(1) rotate(0deg)' })).toBeNull();
  });

  it('pasa la escala a los tamaños y deja el cuadro en el mismo sitio de la pantalla', () => {
    // Excalidraw compensa el origen central con translate((z-1)·w/2, (z-1)·h/2): el cuadro empieza en left/top
    const out = unscaledTextEditorStyle({ ...base, transform: 'translate(100px, 20px) scale(3) rotate(0deg)' })!;
    expect(out).toEqual({
      transform: 'rotate(0deg)',
      width: '300px',
      height: '60px',
      left: '100px',
      top: '50px',
      fontSize: '60px',
      maxHeight: '900px',
    });
  });

  it('conserva la rotación', () => {
    const out = unscaledTextEditorStyle({ ...base, transform: 'translate(50px, 10px) scale(2) rotate(0.5rad)' })!;
    expect(out.transform).toBe('rotate(0.5rad)');
    expect(out.fontSize).toBe('40px');
  });
});
