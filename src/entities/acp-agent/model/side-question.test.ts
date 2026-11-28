import { afterEach, beforeEach, describe, expect, it } from 'bun:test';
import { i18n } from '@shared/lib/i18n';
import { asideBase, btwPrompt, conversationTitle, forkItems, promotedNote, readOnlyMode, rejectOption } from './side-question';
import type { ChatItem } from './types';

const initial = i18n.locale;
beforeEach(() => i18n.setLocale('es'));
afterEach(() => i18n.setLocale(initial));

const user = (id: number, text: string): ChatItem => ({ type: 'user', id, text, attachments: [] });
const fork: ChatItem = { type: 'fork', id: 2, handle: '#1', title: 'hola' };

describe('/fork y /btw', () => {
  it('titula una bifurcación con su primera pregunta propia', () => {
    expect(conversationTitle([user(1, 'hola')])).toBe('hola');
    expect(conversationTitle([user(1, 'hola'), fork])).toBe('Bifurcada de hola');
    i18n.setLocale('en');
    expect(conversationTitle([user(1, 'hola'), fork])).toBe('Forked from hola');
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

  it('reconstruye lo que heredó una pregunta aparte, sin lo posterior', () => {
    const items = [user(1, 'hola'), user(2, 'sigue'), user(7, 'después')];
    expect(asideBase(items, 2, '#3', 'hola')).toEqual([
      items[0],
      items[1],
      { type: 'fork', id: 3, handle: '#3', title: 'hola' },
    ]);
    expect(asideBase(items, 0, '#3', '')).toEqual([]);
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
    i18n.setLocale('en');
    expect(btwPrompt('what about this?')).toContain('[User side question');
    expect(promotedNote()).toContain('[Synapse] This side question is now a normal conversation');
  });
});
