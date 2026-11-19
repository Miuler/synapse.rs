export interface GitFileStatus {
  index?: string | null;
  worktree?: string | null;
  is_stashed?: boolean;
}

export type GitFileStatusKind = GitFileStatus;

/** Repositorio Git de la bóveda: `path` vacío es la raíz; si no, una subcarpeta. */
export interface GitRepoInfo {
  path: string;
  branch?: string | null;
  /** Archivos con cambios sin confirmar. */
  changes: number;
}

export interface VaultGitStatus {
  is_repo: boolean;
  branch?: string | null;
  statuses: Record<string, GitFileStatus>;
  /** Todos los repositorios: la raíz y los de subcarpetas. */
  repos?: GitRepoInfo[];
}

export interface GitDiffResponse {
  path: string;
  is_staged: boolean;
  old_file_name: string;
  new_file_name: string;
  old_content: string;
  new_content: string;
  diff: string;
}

export interface GitFileRevisionResult {
  path: string;
  revision: string;
  content: string | null;
  data_url: string | null;
  is_binary: boolean;
  exists: boolean;
}

export interface GitImageDiffResult {
  path: string;
  is_staged: boolean;
  diff_data_url: string | null;
  diff_count: number;
  width: number;
  height: number;
  total_pixels: number;
  diff_percentage: number;
  has_differences: boolean;
}

export interface GitBranchItem {
  name: string;
  ref_name: string;
  is_current: boolean;
  is_remote: boolean;
  upstream?: string | null;
  last_commit_date?: string | null;
  last_commit_message?: string | null;
}

export interface GitBranchesResult {
  current_branch?: string | null;
  local_branches: GitBranchItem[];
  remote_branches: GitBranchItem[];
}

export interface GitCommitLogItem {
  commit_hash: string;
  short_hash: string;
  author_name: string;
  author_email: string;
  /** Fecha ISO-8601 del autor (el backend la envía como `date`). */
  date: string;
  timestamp: number;
  message: string;
  /** Tags que apuntan a este commit. */
  tags?: string[];
  /** Padres dentro del historial del archivo (para el grafo de ramas). */
  parents?: string[];
  /** Qué le pasó al archivo: `A` agregado, `M`, `D` borrado, `R` renombrado; nada en una fusión. */
  change?: string | null;
  /** Con `R`: la ruta que tenía antes. */
  previous_path?: string | null;
}

export interface GitCommitFileItem {
  path: string;
  statusKind?: 'modified' | 'deleted' | 'untracked' | 'added' | 'staged';
  statusLabel?: string;
  isDeleted?: boolean;
}

export interface GitFileLogResult {
  path: string;
  commits: GitCommitLogItem[];
}

/** Rama o tag que apunta a un commit del log del repositorio. */
export interface GitCommitRef {
  name: string;
  kind: 'local' | 'remote' | 'tag';
  is_head: boolean;
}

/** Commit del log de todas las ramas (`git log --all`). */
export interface GitRepoCommit {
  commit_hash: string;
  short_hash: string;
  parents: string[];
  author_name: string;
  author_email: string;
  date: string;
  timestamp: number;
  message: string;
  refs: GitCommitRef[];
}

export interface GitRepoLogResult {
  head: string | null;
  commits: GitRepoCommit[];
}

export interface GitCommitFileChange {
  /** Relativa al repositorio. */
  path: string;
  change: string;
  previous_path: string | null;
}

export interface GitCommitDetails {
  commit_hash: string;
  parents: string[];
  author_name: string;
  author_email: string;
  committer_name: string;
  date: string;
  message: string;
  files: GitCommitFileChange[];
}
