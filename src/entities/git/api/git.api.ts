import { invokeTauri } from '@shared/api';
import type {
  VaultGitStatus,
  GitDiffResponse,
  GitFileRevisionResult,
  GitImageDiffResult,
  GitFileLogResult,
  GitBranchesResult,
  GitRepoLogResult,
  GitCommitDetails,
} from '../model/types';

export interface GitSquashPreview {
  branch: string;
  commits: number;
  message: string;
  /** Alguno ya está en una rama remota */
  published: boolean;
}

export interface GitUndoPoint {
  branch: string;
  before: string;
  after: string;
  stash: string | null;
  files: boolean;
}

export type GitHistoryAction =
  | { kind: 'squash'; from: string; message: string }
  | { kind: 'revert'; commit: string }
  | { kind: 'reset'; commit: string; hard: boolean }
  | { kind: 'undo'; point: GitUndoPoint };

export const gitApi = {
  async getGitStatus(folderPath?: string): Promise<VaultGitStatus | null> {
    return await invokeTauri<VaultGitStatus | null>('get_vault_git_status', { folderPath });
  },

  async gitAdd(paths: string[]): Promise<void> {
    await invokeTauri('git_add_paths', { paths });
  },

  async gitCommit(paths: string[], message: string): Promise<string> {
    return await invokeTauri<string>('git_commit_paths', { paths, message });
  },

  async gitRestore(paths: string[]): Promise<void> {
    await invokeTauri('git_restore_paths', { paths });
  },

  async gitRestoreStaged(paths: string[]): Promise<void> {
    await invokeTauri('git_restore_staged_paths', { paths });
  },

  async add(paths: string[]): Promise<void> {
    return this.gitAdd(paths);
  },

  async restore(paths: string[]): Promise<void> {
    return this.gitRestore(paths);
  },

  async restoreStaged(paths: string[]): Promise<void> {
    return this.gitRestoreStaged(paths);
  },

  async getGitFileDiff(path: string, staged: boolean, revision?: string): Promise<GitDiffResponse> {
    return await invokeTauri<GitDiffResponse>('get_git_file_diff', { path, staged, revision });
  },

  async getGitFileRevision(path: string, staged: boolean, revision?: string): Promise<GitFileRevisionResult> {
    return await invokeTauri<GitFileRevisionResult>('get_git_file_revision', { path, staged, revision });
  },

  async getGitImageDiff(path: string, staged: boolean, revision?: string): Promise<GitImageDiffResult> {
    return await invokeTauri<GitImageDiffResult>('get_git_image_diff', { path, staged, revision });
  },

  async getGitFileLog(path: string, maxCount = 50): Promise<GitFileLogResult> {
    return await invokeTauri<GitFileLogResult>('get_git_file_log', { path, maxCount });
  },

  async getGitBranches(repo = ''): Promise<GitBranchesResult> {
    return await invokeTauri<GitBranchesResult>('get_git_branches', { repo });
  },

  /** Log de todas las ramas del repositorio (`repo` relativo a la bóveda; `''` = la raíz). */
  async getGitRepoLog(repo = '', maxCount = 1000): Promise<GitRepoLogResult> {
    return await invokeTauri<GitRepoLogResult>('get_git_repo_log', { repo, maxCount });
  },

  async getGitCommitDetails(commit: string, repo = ''): Promise<GitCommitDetails> {
    return await invokeTauri<GitCommitDetails>('get_git_commit_details', { repo, commit });
  },

  async gitCreateTag(params: { name: string; commit: string; message?: string; repo?: string }): Promise<string> {
    return await invokeTauri<string>('git_create_tag', {
      repo: params.repo ?? '',
      name: params.name,
      commit: params.commit,
      message: params.message || null,
    });
  },

  /** Avanza la rama local `branch` hasta `target` (commit o rama) solo si es fast-forward; `dryRun` solo comprueba. */
  async gitFastForward(branch: string, target: string, repo = '', dryRun = false): Promise<string> {
    return await invokeTauri<string>('git_fast_forward', { repo, branch, target, dryRun });
  },

  /** Qué uniría un squash desde `from` hasta el último commit de la rama actual. */
  async gitSquashPreview(from: string, repo = ''): Promise<GitSquashPreview> {
    return await invokeTauri<GitSquashPreview>('git_squash_preview', { repo, from });
  },

  /** Squash, revert o reset de la rama actual; devuelve cómo deshacerlo (`null` al deshacer). */
  async gitChangeHistory(action: GitHistoryAction, repo = ''): Promise<GitUndoPoint | null> {
    return await invokeTauri<GitUndoPoint | null>('git_change_history', { repo, action });
  },

  /** Borra un tag o una rama local (`force` = `git branch -D`). */
  async gitDeleteRef(kind: 'tag' | 'branch', name: string, repo = '', force = false): Promise<string> {
    return await invokeTauri<string>('git_delete_ref', { repo, kind, name, force });
  },

  async gitCheckout(branchName: string, repo = ''): Promise<string> {
    return await invokeTauri<string>('git_checkout_branch', { branchName, repo });
  },

  async gitCreateBranch(params: {
    newBranch: string;
    baseBranch: string;
    checkout: boolean;
    repo?: string;
  }): Promise<string> {
    return await invokeTauri<string>('git_create_branch', {
      newBranch: params.newBranch,
      baseBranch: params.baseBranch,
      checkout: params.checkout,
      repo: params.repo ?? '',
    });
  },

  async gitPush(repo = ''): Promise<string> {
    return await invokeTauri<string>('git_push_vault', { repo });
  },
};
