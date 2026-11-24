import { describe, expect, it } from 'bun:test';
import { btwPrompt, conversationTitle, forkItems, readOnlyMode, rejectOption } from './side-question';
import type { ChatItem } from './types';

const user = (id: number, text: string): ChatItem => ({ type: 'user', id, text, attachments: [] });
const fork: ChatItem = { type: 'fork', id: 2, handle: '#1', title: 'hola' };

describe('/fork y /btw', () => {
  it('titula una bifurcación con su primera pregunta propia', () => {
    expect(conversationTitle([user(1, 'hola')])).toBe('hola');
    expect(conversationTitle([user(1, 'hola'), fork])).toBe('Bifurcada de hola');
    expect(conversationTitle([user(1, 'hola'), fork, user(3, 'otro\ncamino')])).toBe('otro camino');
    expect(conversationTitle([])).toBe('');
  });

  it('no hereda los permisos pendientes de la original', () => {
    const request = { sessionId: 's', requestId: 'r', title: 't', options: [] };
    const items: ChatItem[] = [
      user(1, 'hola'),
      { type: 'permission', id: 2, request, answered: null },
      { type: 'permission', id: 3, request, answered: 'ok' },
    ];
    expect(forkItems(items).map((i) => i.id)).toEqual([1, 3]);
  });

  it('rechaza una sola vez si puede y nunca permite', () => {
    const allow = { id: 'a', name: '', kind: 'allow_once' };
    const always = { id: 'ra', name: '', kind: 'reject_always' };
    const once = { id: 'ro', name: '', kind: 'reject_once' };
    expect(rejectOption([allow, always, once])).toBe('ro');
    expect(rejectOption([allow, always])).toBe('ra');
    expect(rejectOption([allow])).toBeNull();
  });

  it('elige un modo que no modifique', () => {
    expect(readOnlyMode([{ id: 'acceptEdits', name: '' }, { id: 'plan', name: '' }])).toBe('plan');
    expect(readOnlyMode([{ id: 'code', name: '' }])).toBeNull();
  });

  it('manda la pregunta al final', () => {
    expect(btwPrompt('¿y esto?').endsWith('¿y esto?')).toBe(true);
  });
});
