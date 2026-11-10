import { describe, expect, it } from 'bun:test';
import { buildDeletedTree, buildGitTree, childrenAt, collectFolderPaths, mergeDeleted } from './git-tree';

const parse = (raw: unknown) => raw as { worktree?: string; index?: string };
const statuses = {
  'notas/borrada.md': { worktree: 'D' },
  'notas/nueva.md': { worktree: '?' },
  'build/app.bin': { worktree: '?' },
  'viejo/sub/x.md': { index: 'D' },
  'limpia.md': {},
};

describe('git-tree', () => {
  it('el árbol de Git incluye todo tipo de archivo y marca los borrados', () => {
    const tree = buildGitTree(statuses, parse);
    expect(tree.map((n) => n.name)).toEqual(['build', 'notas', 'viejo']);
    const notas = childrenAt(tree, 'notas');
    expect(notas.find((n) => n.name === 'borrada.md')?.deleted).toBe(true);
    expect(notas.find((n) => n.name === 'nueva.md')?.deleted).toBe(false);
    expect(tree.find((n) => n.name === 'notas')?.deleted).toBe(false);
    expect(tree.find((n) => n.name === 'viejo')?.deleted).toBe(true);
    expect(collectFolderPaths(tree)).toEqual(['build', 'notas', 'viejo', 'viejo/sub']);
  });

  it('suma los borrados a los hijos reales sin duplicar carpetas', () => {
    const deleted = buildDeletedTree(statuses, parse, (p) => p.endsWith('.md'));
    const real = [{ name: 'notas', isFolder: true }, { name: 'a.md', isFolder: false }];
    const merged = mergeDeleted(real, deleted, (n) => ({ name: n.name, isFolder: n.isFolder }));
    expect(merged.map((n) => n.name)).toEqual(['notas', 'viejo', 'a.md']);
    expect(childrenAt(deleted, 'notas').map((n) => n.name)).toEqual(['borrada.md']);
  });
});
