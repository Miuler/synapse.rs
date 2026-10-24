import { settingsApi } from '@entities/settings';

export type PendingCloseAction = {
  type: 'single-tab' | 'single-split-tab' | 'multiple-tabs' | 'window-close';
  paths: string[];
} | null;

export class UnsavedChangesState {
  lockedFiles = $state<Set<string>>(new Set());
  unsavedDialogOpen = $state(false);
  unsavedDialogFiles = $state<string[]>([]);
  unsavedDialogIsWindowClose = $state(false);
  isForceSavingUnsaved = $state(false);
  pendingCloseAction = $state<PendingCloseAction>(null);

  setLockedFiles(files: string[]) {
    this.lockedFiles = new Set(files);
  }

  isLocked(path: string | null | undefined): boolean {
    if (!path) return false;
    return this.lockedFiles.has(path);
  }

  async toggleLock(path: string, isStandaloneMode = false) {
    const next = new Set(this.lockedFiles);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    this.lockedFiles = next;

    if (!isStandaloneMode) {
      await settingsApi.saveVaultUiState({
        lockedFiles: Array.from(next),
      });
    }
  }

  openDialog(
    actionType: 'single-tab' | 'single-split-tab' | 'multiple-tabs' | 'window-close',
    paths: string[],
    isWindowClose = false
  ) {
    this.pendingCloseAction = { type: actionType, paths };
    this.unsavedDialogFiles = paths;
    this.unsavedDialogIsWindowClose = isWindowClose;
    this.unsavedDialogOpen = true;
  }

  closeDialog() {
    this.unsavedDialogOpen = false;
    this.pendingCloseAction = null;
    this.unsavedDialogFiles = [];
    this.unsavedDialogIsWindowClose = false;
    this.isForceSavingUnsaved = false;
  }
}
