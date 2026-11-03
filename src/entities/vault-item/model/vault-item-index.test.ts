import { describe, expect, it } from 'bun:test';
import { VaultItemIndex } from './vault-item-index';
import type { VaultItem } from './types';

const item = (id: string, path: string, extra: Partial<VaultItem> = {}): VaultItem => ({
  id,
  title: path,
  relative_path: path,
  ...extra,
});

describe('VaultItemIndex', () => {
  const base = new VaultItemIndex([item('1', 'a.md'), item('2', 'docs/b.md', { view_mode: 'reading' })]);

  it('busca por ruta y cuenta', () => {
    expect(base.count).toBe(2);
    expect(base.get('docs/b.md')?.id).toBe('2');
    expect(base.get('no.md')).toBeUndefined();
    expect(base.has('a.md')).toBe(true);
  });

  it('patch devuelve una lista nueva y conserva el orden y los demás elementos', () => {
    const next = base.patch('docs/b.md', { view_mode: 'live' });
    expect(next).not.toBe(base);
    expect(next.get('docs/b.md')?.view_mode).toBe('live');
    expect(base.get('docs/b.md')?.view_mode).toBe('reading');
    expect(next.items.map((i) => i.relative_path)).toEqual(['a.md', 'docs/b.md']);
    expect(next.items[0]).toBe(base.items[0]);
  });

  it('patch de una ruta inexistente no cambia nada', () => {
    expect(base.patch('no.md', { title: 'x' })).toBe(base);
  });

  it('add agrega al final y queda buscable', () => {
    const next = base.add(item('3', 'c.md'));
    expect(next.count).toBe(3);
    expect(next.items[2].relative_path).toBe('c.md');
    expect(next.has('c.md')).toBe(true);
    expect(base.has('c.md')).toBe(false);
  });
});

describe('vaultItemStore', () => {
  it('expone el índice y notifica cambios reemplazándolo', async () => {
    const { vaultItemStore } = await import('./vault-item-store.svelte');
    vaultItemStore.set([item('1', 'a.md'), item('2', 'b.md')]);
    const before = vaultItemStore.items;
    vaultItemStore.patch('b.md', { title: 'B' });
    expect(vaultItemStore.items).not.toBe(before);
    expect(vaultItemStore.get('b.md')?.title).toBe('B');
    vaultItemStore.add(item('3', 'c.md'));
    expect(vaultItemStore.count).toBe(3);
    vaultItemStore.clear();
    expect(vaultItemStore.count).toBe(0);
  });
});
