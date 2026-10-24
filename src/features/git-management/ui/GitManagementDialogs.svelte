<script lang="ts">
  import GitBranchDialog from './GitBranchDialog.svelte';
  import GitCommitDialog from './GitCommitDialog.svelte';
  import GitFileLogDialog from './GitFileLogDialog.svelte';
  import {
    gitDialogState,
    gitApi,
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
    onToast?: (message: string) => void;
  }

  let {
    gitBranch = null,
    gitStatuses = {},
    allFiles,
    onRefreshGit,
    onViewRevision,
    onDiffRevision,
    onToast,
  }: Props = $props();

  let isCommitting = $state(false);
  let commitErrorMessage = $state<string | null>(null);

  let computedAllFiles = $derived.by<GitCommitFileItem[]>(() => {
    if (allFiles) return allFiles;
    if (!gitStatuses) return [];
    const list: GitCommitFileItem[] = [];
    for (const [path, status] of Object.entries(gitStatuses)) {
      if (!status) continue;
      const index = typeof status.index === 'string' ? status.index : null;
      const worktree = typeof status.worktree === 'string' ? status.worktree : null;

      let statusKind: GitCommitFileItem['statusKind'] = 'modified';
      let statusLabel = 'Modificado';
      let isDeleted = false;

      if (worktree === 'D' || index === 'D') {
        statusKind = 'deleted';
        statusLabel = 'Eliminado';
        isDeleted = true;
      } else if (worktree === '?' || index === '?') {
        statusKind = 'untracked';
        statusLabel = 'No rastreado';
      } else if (index === 'A') {
        statusKind = 'added';
        statusLabel = 'Nuevo';
      } else if (index && !worktree) {
        statusKind = 'staged';
        statusLabel = 'En stage';
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
            ? `${targets.length} elementos confirmados (git commit)`
            : 'Commit realizado con éxito'
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
      if (onToast) onToast(`Error en git commit: ${msg}`);
    } finally {
      isCommitting = false;
    }
  }
</script>

<GitCommitDialog
  bind:open={gitDialogState.isCommitDialogOpen}
  paths={gitDialogState.commitPaths}
  allFiles={computedAllFiles}
  branchName={gitBranch}
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
/>
