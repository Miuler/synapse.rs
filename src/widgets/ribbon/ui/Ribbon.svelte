<script lang="ts">
  import { Toolbar, Tooltip } from 'bits-ui';
  import {
    Folder,
    Search,
    FilePlus,
    Command,
    Network,
    Settings
  } from 'lucide-svelte';

  interface Props {
    activeTab?: string;
    onAction?: (actionId: string) => void;
  }

  let { activeTab = $bindable('files'), onAction }: Props = $props();

  const topTools = [
    { id: 'files', label: 'Explorador de archivos (Alt+1, Ctrl+B)', icon: 'folder' },
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

<Tooltip.Provider delayDuration={200}>
  <Toolbar.Root class="ribbon" orientation="vertical" aria-label="Barra de herramientas lateral">
    <div class="top-actions">
      {#each topTools as tool}
        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              <Toolbar.Button
                {...props}
                class="ribbon-btn {activeTab === tool.id ? 'active' : ''}"
                onclick={(e) => {
                  (props as Record<string, any>).onclick?.(e);
                  handleToolClick(tool.id);
                }}
                aria-label={tool.label}
              >
                {#if tool.icon === 'folder'}
                  <Folder size={18} class="icon" />
                {:else if tool.icon === 'search'}
                  <Search size={18} class="icon" />
                {:else if tool.icon === 'file-plus'}
                  <FilePlus size={18} class="icon" />
                {:else if tool.icon === 'command'}
                  <Command size={18} class="icon" />
                {:else if tool.icon === 'graph'}
                  <Network size={18} class="icon" />
                {/if}
              </Toolbar.Button>
            {/snippet}
          </Tooltip.Trigger>
          <Tooltip.Portal>
            <Tooltip.Content side="right" sideOffset={10} class="ribbon-tooltip">
              {tool.label}
            </Tooltip.Content>
          </Tooltip.Portal>
        </Tooltip.Root>
      {/each}
    </div>

    <div class="bottom-actions">
      <Tooltip.Root>
        <Tooltip.Trigger>
          {#snippet child({ props })}
            <Toolbar.Button
              {...props}
              class="ribbon-btn {activeTab === 'settings' ? 'active' : ''}"
              onclick={(e) => {
                (props as Record<string, any>).onclick?.(e);
                handleToolClick('settings');
              }}
              aria-label="Configuración"
            >
              <Settings size={18} class="icon" />
            </Toolbar.Button>
          {/snippet}
        </Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content side="right" sideOffset={10} class="ribbon-tooltip">
            Configuración
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
    </div>
  </Toolbar.Root>
</Tooltip.Provider>

<style>
  :global(.ribbon) {
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

  :global(.ribbon-btn) {
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
    outline: none;
  }

  :global(.ribbon-btn:hover) {
    color: var(--text-primary, #1f2328);
    background-color: rgba(0, 0, 0, 0.05);
  }

  :global(.ribbon-btn.active) {
    color: var(--accent, #0969da);
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.1));
  }

  :global(.ribbon-btn.active::before) {
    content: '';
    position: absolute;
    left: -6px;
    width: 3px;
    height: 18px;
    background-color: var(--accent, #0969da);
    border-radius: 0 4px 4px 0;
  }

  :global(.ribbon-btn .icon) {
    width: 20px;
    height: 20px;
  }

  :global(.ribbon-tooltip) {
    background: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
    padding: 5px 10px;
    border-radius: 6px;
    font-size: 12px;
    white-space: nowrap;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.1);
    border: 1px solid var(--border-primary, #d0d7de);
    z-index: 1000;
    animation: tooltip-fade 0.15s ease-out;
  }

  @keyframes tooltip-fade {
    from {
      opacity: 0;
      transform: translateX(-4px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }
</style>
