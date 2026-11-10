// Árbol del explorador a partir del estado de Git: con el filtro de Git se ve todo lo que entraría
// en un commit (sin seguimiento, modificado, añadido, borrado), sea o no un tipo de archivo que el
// explorador muestra normalmente; y sin filtro, los archivos borrados que Git aún tiene se siguen viendo.

export interface GitTreeNode {
  name: string;
  relativePath: string;
  isFolder: boolean;
  children: GitTreeNode[];
  /** Nodo que ya no existe en disco: un archivo borrado o una carpeta que solo tiene borrados. */
  deleted?: boolean;
}

export interface GitStatusLike {
  worktree?: string | null;
  index?: string | null;
  is_stashed?: boolean;
}

const LEADING_EMOJI = /^\p{Extended_Pictographic}/u;

/** Orden del explorador: carpetas con emoji, luego carpetas y archivos, alfabético en cada grupo. */
export function sortSiblings<T extends { name: string; isFolder: boolean }>(nodes: T[]): T[] {
  const rank = (node: T) => {
    if (node.isFolder && LEADING_EMOJI.test(node.name)) return 0;
    return node.isFolder ? 1 : 2;
  };
  return nodes.sort((a, b) => {
    const byRank = rank(a) - rank(b);
    if (byRank !== 0) return byRank;
    return a.name.localeCompare(b.name, undefined, { sensitivity: 'base', numeric: true });
  });
}

export function cleanGitPath(path: string): string {
  return path.replace(/^\.\//, '').replace(/\\/g, '/');
}

export function isGitRelevant(info: GitStatusLike | null | undefined): boolean {
  return !!info && !!(info.worktree || info.index || info.is_stashed);
}

export function isGitDeleted(info: GitStatusLike | null | undefined): boolean {
  return !!info && (info.worktree === 'D' || info.index === 'D');
}

function insertPath(roots: GitTreeNode[], path: string, deleted: boolean) {
  const parts = path.split('/').filter(Boolean);
  let level = roots;
  let current = '';
  parts.forEach((part, i) => {
    current = current ? `${current}/${part}` : part;
    const isFolder = i < parts.length - 1;
    let node = level.find((n) => n.name === part && n.isFolder === isFolder);
    if (!node) {
      node = { name: part, relativePath: current, isFolder, children: [], deleted: isFolder ? true : deleted };
      level.push(node);
    }
    // Una carpeta deja de contar como borrada en cuanto tiene algo que sigue en disco
    if (isFolder && !deleted) node.deleted = false;
    level = node.children;
  });
}

function sortTree(nodes: GitTreeNode[]) {
  sortSiblings(nodes);
  for (const node of nodes) if (node.isFolder) sortTree(node.children);
}

/** Árbol con todos los archivos que Git marca, de cualquier tipo. */
export function buildGitTree(
  statuses: Record<string, unknown>,
  parse: (raw: unknown) => GitStatusLike | null,
): GitTreeNode[] {
  const roots: GitTreeNode[] = [];
  for (const [raw, status] of Object.entries(statuses)) {
    const info = parse(status);
    if (!isGitRelevant(info)) continue;
    const path = cleanGitPath(raw);
    if (path) insertPath(roots, path, isGitDeleted(info));
  }
  sortTree(roots);
  return roots;
}

/** Árbol solo con los archivos borrados que Git aún conoce (filtrados por `include`). */
export function buildDeletedTree(
  statuses: Record<string, unknown>,
  parse: (raw: unknown) => GitStatusLike | null,
  include: (path: string) => boolean = () => true,
): GitTreeNode[] {
  const roots: GitTreeNode[] = [];
  for (const [raw, status] of Object.entries(statuses)) {
    const path = cleanGitPath(raw);
    if (path && isGitDeleted(parse(status)) && include(path)) insertPath(roots, path, true);
  }
  return roots;
}

/** Hijos de `parentPath` dentro de un árbol (raíz con ''). */
export function childrenAt(roots: GitTreeNode[], parentPath: string): GitTreeNode[] {
  if (!parentPath) return roots;
  let level = roots;
  for (const part of parentPath.split('/').filter(Boolean)) {
    const folder = level.find((n) => n.isFolder && n.name === part);
    if (!folder) return [];
    level = folder.children;
  }
  return level;
}

/**
 * Suma a los hijos reales de una carpeta los borrados de Git que ya no están en disco.
 * Una carpeta que ya existe en la lista no se duplica.
 */
export function mergeDeleted<T extends { name: string; isFolder: boolean }>(
  existing: T[],
  deleted: GitTreeNode[],
  toNode: (n: GitTreeNode) => T,
): T[] {
  if (deleted.length === 0) return existing;
  const names = new Set(existing.map((n) => `${n.isFolder ? 'd' : 'f'}:${n.name}`));
  const extra = deleted.filter((n) => !names.has(`${n.isFolder ? 'd' : 'f'}:${n.name}`)).map(toNode);
  if (extra.length === 0) return existing;
  return sortSiblings([...existing, ...extra]);
}

/** Todas las rutas de carpeta de un árbol. */
export function collectFolderPaths(nodes: GitTreeNode[], out: string[] = []): string[] {
  for (const node of nodes) {
    if (node.isFolder) {
      out.push(node.relativePath);
      collectFolderPaths(node.children, out);
    }
  }
  return out;
}
