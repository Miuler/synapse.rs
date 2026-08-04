<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { isTauriEnvironment } from '../services/tauri';

  interface Props {
    title?: string;
    isEditing?: boolean;
    onToggleView?: () => void;
    onSplitView?: () => void;
    onOpenCommandPalette?: () => void;
    onAction?: (actionId: string) => void;
  }

  let {
    title = 'Nota de bienvenida.md',
    isEditing = $bindable(true),
    onToggleView,
    onSplitView,
    onOpenCommandPalette,
    onAction
  }: Props = $props();

  function triggerAction(actionId: string) {
    if (onAction) onAction(actionId);
  }

  async function handleWindowControl(action: 'minimize' | 'maximize' | 'close') {
    if (!isTauriEnvironment()) return;
    const appWindow = getCurrentWindow();
    if (action === 'minimize') {
      await appWindow.minimize();
    } else if (action === 'maximize') {
      await appWindow.toggleMaximize();
    } else if (action === 'close') {
      await appWindow.close();
    }
  }
</script>

<header class="editor-header" data-tauri-drag-region>
  <div class="left-section">
    <div class="nav-buttons">
      <button type="button" class="icon-btn" onclick={() => triggerAction('nav-back')} title="Navegar atrás">
        <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="15 18 9 12 15 6"/>
        </svg>
      </button>
      <button type="button" class="icon-btn" onclick={() => triggerAction('nav-forward')} title="Navegar adelante">
        <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="9 18 15 12 9 6"/>
        </svg>
      </button>
    </div>

    <div class="tab-title-container">
      <svg class="file-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
        <polyline points="14 2 14 8 20 8"/>
      </svg>
      <span class="tab-title">{title}</span>
    </div>
  </div>

  <div class="right-section">
    <button
      type="button"
      class="view-toggle-btn"
      onclick={() => {
        isEditing = !isEditing;
        if (onToggleView) onToggleView();
      }}
      title={isEditing ? 'Cambiar a modo Lectura' : 'Cambiar a modo Edición'}
    >
      <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        {#if isEditing}
          <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
          <circle cx="12" cy="12" r="3"/>
        {:else}
          <path d="M12 20h9"/>
          <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/>
        {/if}
      </svg>
      <span>{isEditing ? 'Lectura' : 'Edición'}</span>
    </button>

    <button type="button" class="icon-btn" onclick={() => { if (onSplitView) onSplitView(); }} title="Dividir panel verticalmente">
      <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
        <line x1="12" y1="3" x2="12" y2="21"/>
      </svg>
    </button>

    <button type="button" class="icon-btn" onclick={() => triggerAction('more-options')} title="Más opciones de la nota">
      <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="1"/>
        <circle cx="19" cy="12" r="1"/>
        <circle cx="5" cy="12" r="1"/>
      </svg>
    </button>

    <!-- CONTROLES DE VENTANA PERSONALIZADOS -->
    <div class="window-controls">
      <button type="button" class="win-btn minimize" onclick={() => handleWindowControl('minimize')} title="Minimizar">
        <svg class="win-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="5" y1="12" x2="19" y2="12"/>
        </svg>
      </button>
      <button type="button" class="win-btn maximize" onclick={() => handleWindowControl('maximize')} title="Maximizar / Restaurar">
        <svg class="win-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="5" y="5" width="14" height="14" rx="1"/>
        </svg>
      </button>
      <button type="button" class="win-btn close" onclick={() => handleWindowControl('close')} title="Cerrar">
        <svg class="win-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="18" y1="6" x2="6" y2="18"/>
          <line x1="6" y1="6" x2="18" y2="18"/>
        </svg>
      </button>
    </div>
  </div>
</header>

<style>
  .editor-header {
    height: 42px;
    flex-shrink: 0;
    background-color: var(--bg-primary, #ffffff);
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    user-select: none;
  }

  .left-section,
  .right-section {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .nav-buttons {
    display: flex;
    gap: 2px;
  }

  .tab-title-container {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    background: var(--bg-secondary, #f6f8fa);
    border-radius: 6px;
    border: 1px solid var(--border-primary, #d0d7de);
  }

  .file-icon {
    width: 14px;
    height: 14px;
    color: var(--accent, #0969da);
  }

  .tab-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary, #1f2328);
  }

  .icon-btn {
    width: 30px;
    height: 30px;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-secondary, #656d76);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .icon-btn:hover {
    color: var(--text-primary, #1f2328);
    background-color: rgba(0, 0, 0, 0.05);
  }

  .view-toggle-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    height: 30px;
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    color: var(--text-primary, #1f2328);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .view-toggle-btn:hover {
    background: rgba(0, 0, 0, 0.05);
    color: var(--accent, #0969da);
    border-color: var(--accent, #0969da);
  }

  .icon {
    width: 16px;
    height: 16px;
  }

  .window-controls {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: 8px;
    padding-left: 8px;
    border-left: 1px solid var(--border-primary, #d0d7de);
  }

  .win-btn {
    width: 28px;
    height: 28px;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-secondary, #656d76);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .win-btn:hover {
    color: var(--text-primary, #1f2328);
    background-color: rgba(0, 0, 0, 0.05);
  }

  .win-btn.close:hover {
    color: #ffffff;
    background-color: #e5534b;
  }

  .win-icon {
    width: 14px;
    height: 14px;
  }
</style>
