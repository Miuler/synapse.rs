import { describe, expect, it } from 'bun:test';
import { modeBadge, shortModelLabels } from './model-label';

describe('shortModelLabels', () => {
  it('quita el fabricante y abrevia el esfuerzo', () => {
    expect(shortModelLabels(['Gemini 4 Argon (Low)', 'Claude Opus 5.5 (High)', 'Gemini 4 Argon (Medium)'])).toEqual([
      'Argon·L',
      'Opus 5.5·H',
      'Argon·M',
    ]);
  });

  it('corta los nombres largos', () => {
    const [l] = shortModelLabels(['Gemini 4 Hydrogen Ultra Preview (Low)']);
    expect(l.endsWith('…·L')).toBe(true);
    expect(l.length).toBeLessThanOrEqual(12);
  });

  it('conserva la versión si dos quedan iguales', () => {
    expect(shortModelLabels(['Gemini 3 Argon', 'Gemini 4 Argon'])).toEqual(['3 Argon', '4 Argon']);
  });
});

describe('modeBadge', () => {
  it('pone icono según el modo', () => {
    expect(modeBadge('plan', 'Plan')).toBe('◐ Plan');
    expect(modeBadge('auto', 'Auto')).toBe('⚡ Auto');
  });
});

describe('modelos desconocidos', () => {
  it('muestran su nombre completo, sin cortar', () => {
    expect(shortModelLabels(['Qwen 3 Coder 480B', 'Química 3'])).toEqual(['Qwen 3 Coder 480B', 'Química 3']);
  });

  it('un fabricante conocido sigue abreviándose junto a uno desconocido', () => {
    expect(shortModelLabels(['Gemini 4 Argon (Low)', 'DeepSeek V4'])).toEqual(['Argon·L', 'DeepSeek V4']);
  });
});
