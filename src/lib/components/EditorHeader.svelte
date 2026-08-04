<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { isTauriEnvironment } from '../services/tauri';

  export interface TabItem {
    path: string;
    title: string;
    isDirty?: boolean;
  }

  interface Props {
    title?: string;
    tabs?: TabItem[];
    activeTabPath?: string | null;
    isEditing?: boolean;
    showSaveButton?: boolean;
    onSelectTab?: (path: string) => void;
    onCloseTab?: (path: string) => void;
    onToggleView?: () => void;
    onSplitView?: () => void;
    onOpenCommandPalette?: () => void;
    onSave?: () => void;
    onAction?: (actionId: string) => void;
  }

  let {
    title = 'Nota de bienvenida.md',
    tabs = [],
    activeTabPath = null,
    isEditing = $bindable(true),
    showSaveButton = false,
    onSelectTab,
    onCloseTab,
    onToggleView,
    onSplitView,
    onOpenCommandPalette,
    onSave,
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

    {#if tabs && tabs.length > 0}
      <div class="tabs-bar">
        {#each tabs as tab (tab.path)}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="tab-title-container"
            class:active={tab.path === activeTabPath}
            class:is-dirty={tab.isDirty}
            onclick={() => { if (onSelectTab) onSelectTab(tab.path); }}
          >
            <svg class="file-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
              <polyline points="14 2 14 8 20 8"/>
            </svg>
            <span class="tab-title">{tab.title}{tab.isDirty ? ' *' : ''}</span>
            <button
              type="button"
              class="close-tab-btn"
              onclick={(e) => {
                e.stopPropagation();
                if (onCloseTab) onCloseTab(tab.path);
              }}
              title="Cerrar pestaña"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18"/>
                <line x1="6" y1="6" x2="18" y2="18"/>
              </svg>
            </button>
          </div>
        {/each}
      </div>
    {:else if title}
      <div class="tab-title-container active">
        <svg class="file-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
          <polyline points="14 2 14 8 20 8"/>
        </svg>
        <span class="tab-title">{title}</span>
        <button type="button" class="close-tab-btn" onclick={() => { if (onCloseTab) onCloseTab(''); }} title="Cerrar pestaña">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18"/>
            <line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>
    {/if}
  </div>

  <div class="right-section">
    {#if showSaveButton}
      <button
        type="button"
        class="save-btn"
        onclick={() => {
          if (onSave) onSave();
        }}
        title="Guardar / Grabar cambios (Ctrl+S)"
      >
        <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/>
          <polyline points="17 21 17 13 7 13 7 21"/>
          <polyline points="7 3 7 8 15 8"/>
        </svg>
        <span>Grabar</span>
      </button>
    {/if}

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

  .tabs-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    overflow-x: auto;
    max-width: calc(100vw - 340px);
    padding: 2px 0;
    scrollbar-width: thin;
  }

  .tab-title-container {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    background: var(--bg-secondary, #f6f8fa);
    border-radius: 6px;
    border: 1px solid var(--border-primary, #d0d7de);
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
    opacity: 0.75;
  }

  .tab-title-container:hover {
    opacity: 1;
    background: var(--bg-primary, #ffffff);
  }

  .tab-title-container.active {
    opacity: 1;
    background: var(--bg-primary, #ffffff);
    border-color: var(--accent, #0969da);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
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

  .close-tab-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    background: transparent;
    border-radius: 4px;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    margin-left: 4px;
    transition: all 0.15s ease;
  }

  .close-tab-btn:hover {
    background: rgba(0, 0, 0, 0.1);
    color: var(--text-primary, #1f2328);
  }

  .close-tab-btn svg {
    width: 12px;
    height: 12px;
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

  .save-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    background: var(--accent-bg, rgba(9, 105, 218, 0.1));
    border: 1px solid var(--accent-border, rgba(9, 105, 218, 0.3));
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--accent, #0969da);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .save-btn:hover {
    background: var(--accent, #0969da);
    color: #ffffff;
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
