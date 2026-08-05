<script lang="ts">
  interface Props {
    activeTab?: string;
    onAction?: (actionId: string) => void;
  }

  let { activeTab = $bindable('files'), onAction }: Props = $props();

  const topTools = [
    { id: 'files', label: 'Explorador de archivos', icon: 'folder' },
    { id: 'search', label: 'Buscar en notas', icon: 'search' },
    { id: 'new-note', label: 'Crear nueva nota', icon: 'file-plus' },
    { id: 'command-palette', label: 'Paleta de comandos (Ctrl+P)', icon: 'command' },
    { id: 'graph', label: 'Vista de gráfico', icon: 'graph' },
  ];

  function handleToolClick(id: string) {
    if (id === 'files' || id === 'search') {
      if (activeTab === id) {
        activeTab = '';
      } else {
        activeTab = id;
      }
    }
    if (onAction) onAction(id);
  }
</script>

<aside class="ribbon" aria-label="Barra de herramientas lateral">
  <div class="top-actions">
    {#each topTools as tool}
      <button
        type="button"
        class="ribbon-btn"
        class:active={activeTab === tool.id}
        onclick={() => handleToolClick(tool.id)}
        aria-label={tool.label}
      >
        <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          {#if tool.icon === 'folder'}
            <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2z"/>
          {:else if tool.icon === 'search'}
            <circle cx="11" cy="11" r="8"/>
            <line x1="21" y1="21" x2="16.65" y2="16.65"/>
          {:else if tool.icon === 'file-plus'}
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
            <polyline points="14 2 14 8 20 8"/>
            <line x1="12" y1="18" x2="12" y2="12"/>
            <line x1="9" y1="15" x2="15" y2="15"/>
          {:else if tool.icon === 'command'}
            <path d="M18 3a3 3 0 0 0-3 3v12a3 3 0 0 0 3 3 3 3 0 0 0 3-3 3 3 0 0 0-3-3H6a3 3 0 0 0-3 3 3 3 0 0 0 3 3 3 3 0 0 0 3-3V6a3 3 0 0 0-3-3 3 3 0 0 0-3 3 3 3 0 0 0 3 3h12a3 3 0 0 0 3-3 3 3 0 0 0-3-3z"/>
          {:else if tool.icon === 'graph'}
            <circle cx="18" cy="5" r="3"/>
            <circle cx="6" cy="12" r="3"/>
            <circle cx="18" cy="19" r="3"/>
            <line x1="8.59" y1="13.51" x2="15.42" y2="17.49"/>
            <line x1="15.41" y1="6.51" x2="8.59" y2="10.49"/>
          {/if}
        </svg>
        <span class="tooltip">{tool.label}</span>
      </button>
    {/each}
  </div>

  <div class="bottom-actions">
    <button
      type="button"
      class="ribbon-btn"
      class:active={activeTab === 'settings'}
      onclick={() => handleToolClick('settings')}
      aria-label="Configuración"
    >
      <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="3"/>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
      </svg>
      <span class="tooltip">Configuración</span>
    </button>
  </div>
</aside>

<style>
  .ribbon {
    width: 48px;
    height: 100%;
    background-color: var(--bg-secondary, #f6f8fa);
    border-right: 1px solid var(--border-primary, #d0d7de);
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    align-items: center;
    padding: 10px 0;
    user-select: none;
    z-index: 10;
    flex-shrink: 0;
  }

  .top-actions,
  .bottom-actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
    align-items: center;
  }

  .ribbon-btn {
    position: relative;
    width: 36px;
    height: 36px;
    background: transparent;
    border: none;
    border-radius: 8px;
    color: var(--text-secondary, #656d76);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .ribbon-btn:hover {
    color: var(--text-primary, #1f2328);
    background-color: rgba(0, 0, 0, 0.05);
  }

  .ribbon-btn.active {
    color: var(--accent, #0969da);
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.1));
  }

  .ribbon-btn.active::before {
    content: '';
    position: absolute;
    left: -6px;
    width: 3px;
    height: 18px;
    background-color: var(--accent, #0969da);
    border-radius: 0 4px 4px 0;
  }

  .icon {
    width: 20px;
    height: 20px;
  }

  .tooltip {
    position: absolute;
    left: 50px;
    background: #ffffff;
    color: #1f2328;
    padding: 5px 10px;
    border-radius: 6px;
    font-size: 12px;
    white-space: nowrap;
    opacity: 0;
    pointer-events: none;
    transform: translateX(-6px);
    transition: all 0.18s ease;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.1);
    border: 1px solid var(--border-primary, #d0d7de);
    z-index: 100;
  }

  .ribbon-btn:hover .tooltip {
    opacity: 1;
    transform: translateX(0);
  }
</style>
