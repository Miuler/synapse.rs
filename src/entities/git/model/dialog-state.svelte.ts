/**
 * Estado reactivo compartido para la apertura y control de diálogos de Git.
 * Permite que cualquier componente (VaultExplorer, StatusBar, Menús, Atajos)
 * active diálogos de Git sin acoplamiento horizontal entre features.
 */

class GitDialogState {
  isBranchDialogOpen = $state(false);
  isCommitDialogOpen = $state(false);
  isFileLogOpen = $state(false);
  commitPaths = $state<string[]>([]);
  fileLogTarget = $state<string | null>(null);

  openBranchDialog() {
    this.isBranchDialogOpen = true;
  }

  closeBranchDialog() {
    this.isBranchDialogOpen = false;
  }

  openCommitDialog(paths: string[]) {
    this.commitPaths = paths;
    this.isCommitDialogOpen = true;
  }

  closeCommitDialog() {
    this.isCommitDialogOpen = false;
    this.commitPaths = [];
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
