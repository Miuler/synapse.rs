import { describe, expect, it } from 'bun:test';
import { replayText } from './replay-text';
import type { ChatItem } from './types';

describe('replayText', () => {
  it('manda toda la conversación si nunca se compactó', () => {
    const items: ChatItem[] = [
      { type: 'user', id: 1, text: 'hola', attachments: [] },
      { type: 'agent', id: 2, text: 'qué tal' },
    ];
    const out = replayText(items);
    expect(out).toContain('Usuario: hola');
    expect(out).toContain('Agente: qué tal');
  });

  it('tras /compact manda solo el resumen y lo posterior', () => {
    const items: ChatItem[] = [
      { type: 'user', id: 1, text: 'tema viejo', attachments: [] },
      { type: 'agent', id: 2, text: 'respuesta vieja' },
      { type: 'compact', id: 3 },
      { type: 'user', id: 4, text: '/compact', attachments: [] },
      { type: 'agent', id: 5, text: 'Resumen: hablamos del tema viejo' },
      { type: 'user', id: 6, text: 'sigamos', attachments: [] },
    ];
    const out = replayText(items);
    expect(out).not.toContain('respuesta vieja');
    expect(out).not.toContain('Usuario: /compact');
    expect(out).toContain('Agente: Resumen: hablamos del tema viejo');
    expect(out).toContain('Usuario: sigamos');
  });
});
