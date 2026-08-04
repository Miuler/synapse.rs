<script lang="ts">
  interface Props {
    wordCount?: number;
    charCount?: number;
    line?: number;
    col?: number;
    syncStatus?: 'synced' | 'saving' | 'error';
    isVimMode?: boolean;
    onToggleVim?: () => void;
    onOpenCommandPalette?: () => void;
  }

  let {
    wordCount = 142,
    charCount = 890,
    line = 12,
    col = 24,
    syncStatus = 'synced',
    isVimMode = false,
    onToggleVim,
    onOpenCommandPalette
  }: Props = $props();
</script>

<footer class="status-bar">
  <div class="left-group">
    <button
      type="button"
      class="status-item clickable"
      onclick={() => { if (onOpenCommandPalette) onOpenCommandPalette(); }}
      title="Abrir paleta de comandos"
    >
      <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M18 3a3 3 0 0 0-3 3v12a3 3 0 0 0 3 3 3 3 0 0 0 3-3 3 3 0 0 0-3-3H6a3 3 0 0 0-3 3 3 3 0 0 0 3 3 3 3 0 0 0 3-3V6a3 3 0 0 0-3-3 3 3 0 0 0-3 3 3 3 0 0 0 3 3h12a3 3 0 0 0 3-3 3 3 0 0 0-3-3z"/>
      </svg>
      <span>Ctrl+P</span>
    </button>

    <div class="divider"></div>

    <div class="status-item">
      <span class="dot {syncStatus}"></span>
      <span>{syncStatus === 'synced' ? 'Guardado' : syncStatus === 'saving' ? 'Guardando...' : 'Error de guardado'}</span>
    </div>
  </div>

  <div class="right-group">
    <button
      type="button"
      class="status-item clickable vim-btn"
      class:active={isVimMode}
      onclick={() => { if (onToggleVim) onToggleVim(); }}
      title={isVimMode ? 'Desactivar modo VIM en el editor' : 'Activar modo VIM en el editor'}
    >
      <span class="vim-badge">VIM</span>
      <span>{isVimMode ? 'ON' : 'OFF'}</span>
    </button>
    <div class="divider"></div>
    <div class="status-item">
      <span>{wordCount} palabras</span>
    </div>
    <div class="divider"></div>
    <div class="status-item">
      <span>{charCount} caracteres</span>
    </div>
    <div class="divider"></div>
    <div class="status-item">
      <span>Lín {line}, Col {col}</span>
    </div>
    <div class="divider"></div>
    <div class="status-item highlight">
      <span>UTF-8</span>
    </div>
  </div>
</footer>

<style>
  .status-bar {
    height: 26px;
    flex-shrink: 0;
    background-color: var(--bg-secondary, #f6f8fa);
    border-top: 1px solid var(--border-primary, #d0d7de);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    font-size: 11px;
    color: var(--text-secondary, #656d76);
    user-select: none;
  }

  .left-group,
  .right-group {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .status-item.clickable {
    background: transparent;
    border: none;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    transition: all 0.15s ease;
  }

  .status-item.clickable:hover {
    background: rgba(0, 0, 0, 0.05);
    color: var(--accent, #0969da);
  }

  .status-item.highlight {
    color: var(--text-secondary, #656d76);
  }

  .vim-btn {
    font-weight: 600;
  }

  .vim-btn.active {
    color: var(--accent, #0969da);
    background: var(--accent-bg, rgba(9, 105, 218, 0.1));
  }

  .vim-badge {
    font-size: 10px;
    padding: 1px 4px;
    background: var(--border-primary, #d0d7de);
    color: var(--text-primary, #1f2328);
    border-radius: 3px;
    font-weight: 700;
  }

  .vim-btn.active .vim-badge {
    background: var(--accent, #0969da);
    color: #ffffff;
  }

  .divider {
    width: 1px;
    height: 12px;
    background-color: var(--border-primary, #d0d7de);
  }

  .icon {
    width: 12px;
    height: 12px;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .dot.synced {
    background-color: #3fb950;
    box-shadow: 0 0 6px rgba(63, 185, 80, 0.6);
  }

  .dot.saving {
    background-color: #d29922;
  }

  .dot.error {
    background-color: #f85149;
  }
</style>
