import { expect, it } from 'bun:test';
import { matchesShortcut } from './shortcuts.svelte';

const ev = (key: string, mods: Partial<KeyboardEvent> = {}) =>
  ({ key, code: '', ctrlKey: true, metaKey: false, altKey: false, shiftKey: false, ...mods }) as KeyboardEvent;

it('reconoce la tecla + con y sin Shift, y varias combinaciones', () => {
  const spec = 'Ctrl++, Ctrl+=';
  expect(matchesShortcut(ev('+'), spec)).toBe(true);
  expect(matchesShortcut(ev('+', { shiftKey: true }), spec)).toBe(true);
  expect(matchesShortcut(ev('='), spec)).toBe(true);
  expect(matchesShortcut(ev('-'), spec)).toBe(false);
  expect(matchesShortcut(ev('+', { ctrlKey: false }), spec)).toBe(false);
});
