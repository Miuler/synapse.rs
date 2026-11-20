// Árbol de carpetas para la vista en árbol del diálogo de commit.
// Las cadenas de carpetas con un único hijo carpeta se compactan («src/components»), como en VS Code.

export interface CommitTreeFolder<T> {
  kind: 'folder';
  /** Ruta completa de la carpeta, sin barra final */
  path: string;
  /** Nombre mostrado; puede ser una cadena compactada («a/b/c») */
  name: string;
  children: CommitTreeNode<T>[];
  /** Rutas de todos los archivos que cuelgan de esta carpeta */
  filePaths: string[];
}

export interface CommitTreeFile<T> {
  kind: 'file';
  path: string;
  name: string;
  item: T;
}

export type CommitTreeNode<T> = CommitTreeFolder<T> | CommitTreeFile<T>;

export function buildCommitTree<T extends { path: string }>(items: T[]): CommitTreeNode<T>[] {
  const root: CommitTreeFolder<T> = { kind: 'folder', path: '', name: '', children: [], filePaths: [] };
  for (const item of items) {
    const parts = item.path.split('/').filter(Boolean);
    let folder = root;
    folder.filePaths.push(item.path);
    for (let i = 0; i < parts.length - 1; i++) {
      const path = parts.slice(0, i + 1).join('/');
      let next = folder.children.find((c): c is CommitTreeFolder<T> => c.kind === 'folder' && c.path === path);
      if (!next) {
        next = { kind: 'folder', path, name: parts[i], children: [], filePaths: [] };
        folder.children.push(next);
      }
      next.filePaths.push(item.path);
      folder = next;
    }
    folder.children.push({ kind: 'file', path: item.path, name: parts[parts.length - 1] ?? item.path, item });
  }
  return root.children.map(compact).sort(byKindThenName);
}

function compact<T>(node: CommitTreeNode<T>): CommitTreeNode<T> {
  if (node.kind === 'file') return node;
  let folder = node;
  let name = folder.name;
  while (folder.children.length === 1 && folder.children[0].kind === 'folder') {
    folder = folder.children[0];
    name = `${name}/${folder.name}`;
  }
  return { ...folder, name, children: folder.children.map(compact).sort(byKindThenName) };
}

function byKindThenName<T>(a: CommitTreeNode<T>, b: CommitTreeNode<T>): number {
  if (a.kind !== b.kind) return a.kind === 'folder' ? -1 : 1;
  return a.name.localeCompare(b.name);
}
