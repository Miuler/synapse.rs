import { vaultRepository } from '@shared/repositories';

export type DiffVisualTab = 'side_by_side' | 'highlight' | 'text';
export type DiffTextMode = 'split' | 'unified' | 'raw';

class WorkspaceDiffSettings {
  visualTab = $state<DiffVisualTab>('side_by_side');
  textMode = $state<DiffTextMode>('split');
  isLoaded = $state(false);

  async initFromWorkspace() {
    if (this.isLoaded) return;
    try {
      const uiState = await vaultRepository.getVaultUiState();
      if (uiState) {
        if (uiState.diff_visual_mode === 'side_by_side' || uiState.diff_visual_mode === 'highlight' || uiState.diff_visual_mode === 'text') {
          this.visualTab = uiState.diff_visual_mode;
        }
        if (uiState.diff_text_mode === 'split' || uiState.diff_text_mode === 'unified' || uiState.diff_text_mode === 'raw') {
          this.textMode = uiState.diff_text_mode;
        }
      }
    } catch (e) {
      console.warn('Error al cargar preferencias de diff del workspace:', e);
    } finally {
      this.isLoaded = true;
    }
  }

  setVisualTab(tab: DiffVisualTab) {
    this.visualTab = tab;
    this.persist();
  }

  setTextMode(mode: DiffTextMode) {
    this.textMode = mode;
    this.persist();
  }

  private persistTimeout: ReturnType<typeof setTimeout> | null = null;
  private persist() {
    if (this.persistTimeout) clearTimeout(this.persistTimeout);
    this.persistTimeout = setTimeout(async () => {
      try {
        await vaultRepository.saveVaultUiState(
          undefined,
          undefined,
          this.visualTab,
          this.textMode,
        );
      } catch (e) {
        console.warn('Error al persistir preferencias de diff del workspace:', e);
      }
    }, 200);
  }
}

export const workspaceDiffSettings = new WorkspaceDiffSettings();
