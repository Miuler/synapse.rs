<script lang="ts">
  import GitBranchDialog from './GitBranchDialog.svelte';
  import GitCommitDialog from './GitCommitDialog.svelte';
  import GitFileLogDialog from './GitFileLogDialog.svelte';
  import GitRepoLogDialog from './GitRepoLogDialog.svelte';
  import GitMergeDialog from './GitMergeDialog.svelte';
  import { t } from '@shared/lib/i18n';
  import {
    gitDialogState,
    gitApi,
    repoOf,
    type GitCommitFileItem,
    type GitFileStatusKind,
  } from '@entities/git';

  interface Props {
    gitBranch?: string | null;
    gitStatuses?: Record<string, GitFileStatusKind>;
    allFiles?: GitCommitFileItem[];
    onRefreshGit?: () => Promise<void> | void;
    onViewRevision?: (filePath: string, commitHash: string) => void;
    onDiffRevision?: (filePath: string, commitHash: string) => void;
    /** Abre el parche del commit: el archivo en ese commit contra su commit padre. */
    onPatchRevision?: (filePath: string, commitHash: string) => void;
    /** Diff entre dos versiones del archivo (`from` la más antigua). */
    onCompareRevisions?: (filePath: string, from: string, to: string) => void;
    onAskAgent?: (filePath: string, from: string, to: string, message?: string) => void;
    onToast?: (message: string) => void;
  }

  let {
    gitBranch = null,
    gitStatuses = {},
    allFiles,
    onRefreshGit,
    onViewRevision,
    onDiffRevision,
    onPatchRevision,
    onCompareRevisions,
    onAskAgent,
    onToast,
  }: Props = $props();

  let isCommitting = $state(false);
  let commitErrorMessage = $state<string | null>(null);

  let computedAllFiles = $derived.by<GitCommitFileItem[]>(() => {
    if (allFiles) return allFiles;
    if (!gitStatuses) return [];
    const list: GitCommitFileItem[] = [];
    const repos = gitDialogState.repos;
    for (const [path, status] of Object.entries(gitStatuses)) {
      if (!status) continue;
      // Con varios repositorios, el commit es del repositorio elegido
      if (repos.length > 1 && repoOf(path, repos) !== gitDialogState.repo) continue;
      const index = typeof status.index === 'string' ? status.index : null;
      const worktree = typeof status.worktree === 'string' ? status.worktree : null;

      let statusKind: GitCommitFileItem['statusKind'] = 'modified';
      let statusLabel = t('gitUi.status.modified');
      let isDeleted = false;

      if (worktree === 'D' || index === 'D') {
        statusKind = 'deleted';
        statusLabel = t('gitUi.status.deleted');
        isDeleted = true;
      } else if (worktree === '?' || index === '?') {
        statusKind = 'untracked';
        statusLabel = t('gitUi.status.untracked');
      } else if (index === 'A') {
        statusKind = 'added';
        statusLabel = t('gitUi.status.added');
      } else if (index && !worktree) {
        statusKind = 'staged';
        statusLabel = t('gitUi.status.staged');
      }

      list.push({ path, statusKind, statusLabel, isDeleted });
    }
    return list;
  });

  async function handleConfirmCommit(message: string, selectedPaths?: string[]) {
    const targets =
      selectedPaths && selectedPaths.length > 0
        ? selectedPaths
        : [...gitDialogState.commitPaths];
    if (targets.length === 0 || isCommitting) return;
    isCommitting = true;
    commitErrorMessage = null;
    try {
      await gitApi.gitCommit(targets, message);
      if (onToast) {
        onToast(
          targets.length > 1
            ? t('gitUi.commitToast.multiple', { count: targets.length })
            : t('gitUi.commitToast.single')
        );
      }
      if (onRefreshGit) {
        await onRefreshGit();
      }
      gitDialogState.closeCommitDialog();
    } catch (err: unknown) {
      console.error('Error al realizar git commit:', err);
      const msg = err instanceof Error ? err.message : String(err);
      commitErrorMessage = msg;
      if (onToast) onToast(t('gitUi.commitToast.error', { detail: msg }));
    } finally {
      isCommitting = false;
    }
  }
</script>

<GitCommitDialog
  bind:open={gitDialogState.isCommitDialogOpen}
  paths={gitDialogState.commitPaths}
  allFiles={computedAllFiles}
  branchName={gitDialogState.repos.length > 1
    ? (gitDialogState.repos.find((r) => r.path === gitDialogState.repo)?.branch ?? null)
    : gitBranch}
  loading={isCommitting}
  errorMessage={commitErrorMessage}
  onConfirm={handleConfirmCommit}
  onCancel={() => gitDialogState.closeCommitDialog()}
/>

<GitBranchDialog
  bind:open={gitDialogState.isBranchDialogOpen}
  onBranchChanged={async () => {
    if (onRefreshGit) await onRefreshGit();
  }}
/>

<GitFileLogDialog
  bind:open={gitDialogState.isFileLogOpen}
  filePath={gitDialogState.fileLogTarget}
  onViewRevision={onViewRevision}
  onDiffRevision={onDiffRevision}
  {onPatchRevision}
  {onCompareRevisions}
  {onAskAgent}
/>

<GitMergeDialog
  bind:open={gitDialogState.isMergeOpen}
  onRepoChanged={async () => {
    if (onRefreshGit) await onRefreshGit();
  }}
  {onToast}
/>

<GitRepoLogDialog
  bind:open={gitDialogState.isRepoLogOpen}
  onOpenFileDiff={onPatchRevision}
  onRepoChanged={async () => {
    if (onRefreshGit) await onRefreshGit();
  }}
  {onToast}
/>
