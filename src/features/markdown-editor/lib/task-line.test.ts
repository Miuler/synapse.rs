import { describe, expect, it } from 'bun:test';
import { toggleTaskLine } from './task-line';

describe('toggleTaskLine', () => {
  it('marca y desmarca', () => {
    expect(toggleTaskLine('- [ ] Pendiente 00')).toBe('- [x] Pendiente 00');
    expect(toggleTaskLine('- [x] LISTO 02')).toBe('- [ ] LISTO 02');
    expect(toggleTaskLine('  * [X] hecho')).toBe('  * [ ] hecho');
    expect(toggleTaskLine('1. [ ] uno')).toBe('1. [x] uno');
  });

  it('añade la casilla a un elemento de lista o convierte la línea en tarea', () => {
    expect(toggleTaskLine('- comprar pan')).toBe('- [ ] comprar pan');
    expect(toggleTaskLine('    Pendiente 01')).toBe('    - [ ] Pendiente 01');
    expect(toggleTaskLine('')).toBe('- [ ] ');
  });
});
