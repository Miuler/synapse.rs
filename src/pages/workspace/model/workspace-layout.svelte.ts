import { settingsApi } from '@entities/settings';

export function getInitialSidebarWidth(): number {
  try {
    const saved = localStorage.getItem('synapse_sidebar_width');
    if (saved) {
      const parsed = parseInt(saved, 10);
      if (!isNaN(parsed) && parsed >= 140 && parsed <= 900) return parsed;
    }
  } catch {}
  return 240;
}

export function getInitialSplitRatio(): number {
  try {
    const saved = localStorage.getItem('synapse_split_ratio');
    if (saved) {
      const parsed = parseFloat(saved);
      if (!isNaN(parsed) && parsed >= 20 && parsed <= 80) return parsed;
    }
  } catch {}
  return 50;
}

export class WorkspaceLayoutState {
  sidebarWidth = $state<number>(getInitialSidebarWidth());
  isResizingSidebar = $state(false);
  splitRatio = $state<number>(getInitialSplitRatio());
  isResizingSplit = $state(false);
  isDragOverSplitRight = $state(false);
  activeFocusedPane = $state<'left' | 'right'>('left');
  splitContainerRef = $state<HTMLDivElement | null>(null);

  setSidebarWidth(width: number) {
    this.sidebarWidth = Math.max(140, Math.min(900, width));
    try {
      localStorage.setItem('synapse_sidebar_width', String(this.sidebarWidth));
      settingsApi.saveVaultUiState({ sidebarWidth: this.sidebarWidth });
    } catch {}
  }

  handleSidebarResizeStart(e: MouseEvent | PointerEvent, onCollapse?: () => void, onExpand?: () => void) {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    this.isResizingSidebar = true;
    document.body.classList.add('is-resizing-col');

    const handlePointerMove = (ev: MouseEvent | PointerEvent) => {
      const newWidth = ev.clientX - 48; // 48px ancho del ribbon
      if (newWidth < 70) {
        if (onCollapse) onCollapse();
      } else {
        if (onExpand) onExpand();
        const maxWidth = Math.max(300, window.innerWidth - 200);
        this.sidebarWidth = Math.max(140, Math.min(newWidth, Math.min(900, maxWidth)));
      }
    };

    const handlePointerUp = () => {
      this.isResizingSidebar = false;
      document.body.classList.remove('is-resizing-col');
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
      window.removeEventListener('pointercancel', handlePointerUp);
      window.removeEventListener('mousemove', handlePointerMove);
      window.removeEventListener('mouseup', handlePointerUp);
      try {
        localStorage.setItem('synapse_sidebar_width', String(this.sidebarWidth));
        if (this.sidebarWidth >= 140) {
          settingsApi.saveVaultUiState({ sidebarWidth: this.sidebarWidth });
        }
      } catch {}
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp);
    window.addEventListener('pointercancel', handlePointerUp);
    window.addEventListener('mousemove', handlePointerMove);
    window.addEventListener('mouseup', handlePointerUp);
  }

  handleSplitResizeStart(e: MouseEvent | PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    this.isResizingSplit = true;
    document.body.classList.add('is-resizing-col');

    const handlePointerMove = (ev: MouseEvent | PointerEvent) => {
      if (!this.splitContainerRef) return;
      const rect = this.splitContainerRef.getBoundingClientRect();
      const relativeX = ev.clientX - rect.left;
      const newRatio = (relativeX / rect.width) * 100;
      this.splitRatio = Math.max(20, Math.min(80, newRatio));
    };

    const handlePointerUp = () => {
      this.isResizingSplit = false;
      document.body.classList.remove('is-resizing-col');
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
      window.removeEventListener('pointercancel', handlePointerUp);
      window.removeEventListener('mousemove', handlePointerMove);
      window.removeEventListener('mouseup', handlePointerUp);
      try {
        localStorage.setItem('synapse_split_ratio', String(this.splitRatio));
      } catch {}
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp);
    window.addEventListener('pointercancel', handlePointerUp);
    window.addEventListener('mousemove', handlePointerMove);
    window.addEventListener('mouseup', handlePointerUp);
  }
}
