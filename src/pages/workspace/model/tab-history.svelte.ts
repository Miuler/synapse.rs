const MAX_TAB_HISTORY = 50;

export class TabHistoryState {
  history = $state<string[]>([]);
  currentIndex = $state<number>(-1);
  private isNavigating = false;
  private lastNavTime = 0;

  get canGoBack(): boolean {
    return this.currentIndex > 0;
  }

  get canGoForward(): boolean {
    return this.currentIndex >= 0 && this.currentIndex < this.history.length - 1;
  }

  recordVisit(path: string) {
    if (this.isNavigating || !path) return;
    if (
      this.currentIndex >= 0 &&
      this.currentIndex < this.history.length &&
      this.history[this.currentIndex] === path
    ) {
      return;
    }
    let truncated = this.history.slice(0, this.currentIndex + 1);
    truncated.push(path);
    if (truncated.length > MAX_TAB_HISTORY) {
      truncated = truncated.slice(truncated.length - MAX_TAB_HISTORY);
    }
    this.history = truncated;
    this.currentIndex = truncated.length - 1;
  }

  navigateBack(onSelect: (path: string) => void) {
    if (this.currentIndex <= 0) return;
    const now = performance.now();
    if (now - this.lastNavTime < 150) return;
    this.lastNavTime = now;

    const targetIndex = this.currentIndex - 1;
    this.currentIndex = targetIndex;
    const targetPath = this.history[targetIndex];
    this.isNavigating = true;
    try {
      onSelect(targetPath);
    } finally {
      this.isNavigating = false;
    }
  }

  navigateForward(onSelect: (path: string) => void) {
    if (this.currentIndex >= this.history.length - 1) return;
    const now = performance.now();
    if (now - this.lastNavTime < 150) return;
    this.lastNavTime = now;

    const targetIndex = this.currentIndex + 1;
    this.currentIndex = targetIndex;
    const targetPath = this.history[targetIndex];
    this.isNavigating = true;
    try {
      onSelect(targetPath);
    } finally {
      this.isNavigating = false;
    }
  }

  removeTab(path: string, activeTabPath?: string | null) {
    const newHistory: string[] = [];
    let newIndex = -1;
    for (let i = 0; i < this.history.length; i++) {
      const item = this.history[i];
      if (item !== path) {
        if (newHistory.length === 0 || newHistory[newHistory.length - 1] !== item) {
          if (i <= this.currentIndex) {
            newIndex = newHistory.length;
          }
          newHistory.push(item);
        }
      }
    }
    this.history = newHistory;
    if (this.history.length === 0) {
      this.currentIndex = -1;
    } else {
      this.currentIndex = Math.max(0, Math.min(newIndex, this.history.length - 1));
      if (activeTabPath) {
        const found = this.history.lastIndexOf(activeTabPath);
        if (found !== -1) {
          this.currentIndex = found;
        }
      }
    }
  }

  replaceTab(oldPath: string, newPath: string) {
    this.history = this.history.map((p) => (p === oldPath ? newPath : p));
  }

  syncActiveTab(activeTabPath: string | null) {
    if (activeTabPath) {
      const foundIdx = this.history.slice(0, this.currentIndex).lastIndexOf(activeTabPath);
      if (foundIdx !== -1) {
        this.currentIndex = foundIdx;
      }
    }
  }

  clear() {
    this.history = [];
    this.currentIndex = -1;
  }
}
