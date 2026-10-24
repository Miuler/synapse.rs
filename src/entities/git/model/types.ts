export interface GitFileStatus {
  index?: string | null;
  worktree?: string | null;
  is_stashed?: boolean;
}

export type GitFileStatusKind = GitFileStatus;

export interface VaultGitStatus {
  is_repo: boolean;
  branch?: string | null;
  statuses: Record<string, GitFileStatus>;
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
  date_iso: string;
  timestamp: number;
  message: string;
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
