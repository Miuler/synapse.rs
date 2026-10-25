export class TreeSelectionState {
  selectedPaths = $state<string[]>([]);
  lastFocusedPath = $state<string | null>(null);
  anchorPath = $state<string | null>(null);

  isSelected(path: string): boolean {
    return this.selectedPaths.includes(path);
  }

  selectSingle(path: string) {
    this.selectedPaths = [path];
    this.lastFocusedPath = path;
    this.anchorPath = path;
  }

  toggleSelect(path: string) {
    if (this.selectedPaths.includes(path)) {
      this.selectedPaths = this.selectedPaths.filter((p) => p !== path);
    } else {
      this.selectedPaths = [...this.selectedPaths, path];
    }
    this.lastFocusedPath = path;
    this.anchorPath = path;
  }

  selectRange(endPath: string, visiblePaths: string[]) {
    if (!this.anchorPath || !visiblePaths.includes(this.anchorPath)) {
      this.selectSingle(endPath);
      return;
    }
    const startIdx = visiblePaths.indexOf(this.anchorPath);
    const endIdx = visiblePaths.indexOf(endPath);
    if (startIdx === -1 || endIdx === -1) {
      this.selectSingle(endPath);
      return;
    }
    const [min, max] = startIdx < endIdx ? [startIdx, endIdx] : [endIdx, startIdx];
    this.selectedPaths = visiblePaths.slice(min, max + 1);
    this.lastFocusedPath = endPath;
  }

  selectAll(paths: string[]) {
    this.selectedPaths = [...paths];
    if (paths.length > 0) {
      this.lastFocusedPath = paths[0];
      this.anchorPath = paths[0];
    }
  }

  clear() {
    this.selectedPaths = [];
    this.lastFocusedPath = null;
    this.anchorPath = null;
  }
}
