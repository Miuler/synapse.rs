import { describe, expect, it } from 'bun:test';
import { buildCommitTree, type CommitTreeNode } from './commit-tree';

const show = (nodes: CommitTreeNode<{ path: string }>[], depth = 0): string[] =>
  nodes.flatMap((n) =>
    n.kind === 'file'
      ? [`${'  '.repeat(depth)}${n.name}`]
      : [`${'  '.repeat(depth)}${n.name}/ (${n.filePaths.length})`, ...show(n.children, depth + 1)],
  );

describe('buildCommitTree', () => {
  it('agrupa por carpetas, carpetas primero y compacta cadenas de un solo hijo', () => {
    const tree = buildCommitTree(
      ['AGENTS.md', '.agents/skills/a/SKILL.md', '.agents/skills/b/SKILL.md', 'src/components/d.drawio', 'src/software/f.excalidraw', 'openspec/specs/.gitkeep'].map(
        (path) => ({ path }),
      ),
    );
    expect(show(tree)).toEqual([
      '.agents/skills/ (2)',
      '  a/ (1)',
      '    SKILL.md',
      '  b/ (1)',
      '    SKILL.md',
      'openspec/specs/ (1)',
      '  .gitkeep',
      'src/ (2)',
      '  components/ (1)',
      '    d.drawio',
      '  software/ (1)',
      '    f.excalidraw',
      'AGENTS.md',
    ]);
  });

  it('la carpeta compactada conserva su ruta completa y los archivos que contiene', () => {
    const [folder] = buildCommitTree([{ path: 'a/b/c/x.md' }, { path: 'a/b/c/y.md' }]);
    expect(folder.kind).toBe('folder');
    if (folder.kind !== 'folder') return;
    expect(folder.path).toBe('a/b/c');
    expect(folder.name).toBe('a/b/c');
    expect(folder.filePaths).toEqual(['a/b/c/x.md', 'a/b/c/y.md']);
  });
});
