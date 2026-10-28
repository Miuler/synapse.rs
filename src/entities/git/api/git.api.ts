import { invokeTauri } from '@shared/api';
import type {
  VaultGitStatus,
  GitDiffResponse,
  GitFileRevisionResult,
  GitImageDiffResult,
  GitFileLogResult,
  GitBranchesResult,
} from '../model/types';

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

  async getGitBranches(): Promise<GitBranchesResult> {
    return await invokeTauri<GitBranchesResult>('get_git_branches');
  },

  async gitCheckout(branchName: string): Promise<string> {
    return await invokeTauri<string>('git_checkout_branch', { branchName });
  },

  async gitCreateBranch(params: {
    newBranch: string;
    baseBranch: string;
    checkout: boolean;
  }): Promise<string> {
    return await invokeTauri<string>('git_create_branch', {
      newBranch: params.newBranch,
      baseBranch: params.baseBranch,
      checkout: params.checkout,
    });
  },

  async gitPush(): Promise<string> {
    return await invokeTauri<string>('git_push_vault');
  },
};
