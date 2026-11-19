/**
 * Estado reactivo compartido para la apertura y control de diálogos de Git.
 * Permite que cualquier componente (VaultExplorer, StatusBar, Menús, Atajos)
 * active diálogos de Git sin acoplamiento horizontal entre features.
 */

import type { GitRepoInfo } from './types';

/** Repositorio más cercano que contiene `path` (`''` = la raíz de la bóveda). */
export function repoOf(path: string | null | undefined, repos: GitRepoInfo[]): string {
  const clean = (path ?? '').replace(/\\/g, '/').replace(/^\.\//, '').replace(/\/+$/, '');
  let best = '';
  for (const r of repos) {
    if (r.path && (clean === r.path || clean.startsWith(`${r.path}/`)) && r.path.length > best.length) {
      best = r.path;
    }
  }
  return best;
}

/** Nombre corto de un repositorio para la interfaz. */
export function repoLabel(path: string): string {
  return path ? path.split('/').pop() || path : 'Bóveda';
}

class GitDialogState {
  /** Repositorios de la bóveda (los mantiene al día el refresco del estado de git). */
  repos = $state<GitRepoInfo[]>([]);
  /** Repositorio sobre el que actúan los diálogos de ramas y commit. */
  repo = $state('');
  isBranchDialogOpen = $state(false);
  isCommitDialogOpen = $state(false);
  isFileLogOpen = $state(false);
  isRepoLogOpen = $state(false);
  commitPaths = $state<string[]>([]);
  fileLogTarget = $state<string | null>(null);

  /** Si el repositorio no existe (p. ej. la bóveda no es un repositorio), el primero que haya. */
  private pick(repo: string): string {
    if (this.repos.length === 0 || this.repos.some((r) => r.path === repo)) return repo;
    return this.repos[0].path;
  }

  openBranchDialog(repo?: string) {
    this.repo = this.pick(repo ?? this.repo);
    this.isBranchDialogOpen = true;
  }

  closeBranchDialog() {
    this.isBranchDialogOpen = false;
  }

  openCommitDialog(paths: string[], repo?: string) {
    this.repo = this.pick(repo ?? repoOf(paths[0], this.repos));
    this.commitPaths = paths;
    this.isCommitDialogOpen = true;
  }

  closeCommitDialog() {
    this.isCommitDialogOpen = false;
    this.commitPaths = [];
  }

  /** Log de todas las ramas del repositorio (estilo Git Log de JetBrains). */
  openRepoLog(repo?: string) {
    this.repo = this.pick(repo ?? this.repo);
    this.isRepoLogOpen = true;
  }

  openFileLog(path: string) {
    this.fileLogTarget = path;
    this.isFileLogOpen = true;
  }

  closeFileLog() {
    this.isFileLogOpen = false;
    this.fileLogTarget = null;
  }
}

export const gitDialogState = new GitDialogState();
