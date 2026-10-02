<script lang="ts">
  import { commandRegistry, type AppCommand } from '@entities/command';
  import { Command, Dialog } from 'bits-ui';
  import { Search } from 'lucide-svelte';

  interface Props {
    isOpen?: boolean;
    onClose?: () => void;
  }

  let { isOpen = $bindable(false), onClose }: Props = $props();

  let searchValue = $state('');

  // Escuchar atajo global Ctrl+P o Cmd+P para abrir/cerrar la paleta de comandos
  $effect(() => {
    const handleKeydown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && (e.key === 'p' || e.key === 'P')) {
        e.preventDefault();
        isOpen = !isOpen;
        if (isOpen) {
          searchValue = '';
        }
      }
    };

    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  });

  function closePalette() {
    isOpen = false;
    searchValue = '';
    if (onClose) onClose();
  }

  function executeCommand(cmd: AppCommand) {
    cmd.action();
    closePalette();
  }

  let allCommands = $derived(commandRegistry.all);
</script>

<Dialog.Root
  open={isOpen}
  onOpenChange={(open) => {
    isOpen = open;
    if (!open) {
      closePalette();
    }
  }}
>
  <Dialog.Portal>
    <Dialog.Overlay class="palette-backdrop" />
    <Dialog.Content class="palette-container">
      <Dialog.Title class="sr-only">Paleta de Comandos</Dialog.Title>
      <Command.Root class="command-root" loop>
        <div class="input-wrapper">
          <Search size={16} class="search-icon" />
          <Command.Input
            class="command-input"
            bind:value={searchValue}
            placeholder="Escribe un comando o busca..."
          />
          <span class="esc-badge">ESC</span>
        </div>

        <Command.List class="results-container">
          <Command.Empty class="empty-state">No se encontraron comandos</Command.Empty>

          {#each allCommands as cmd (cmd.id)}
            <Command.Item
              class="command-item"
              value={`${cmd.name} ${cmd.category} ${cmd.shortcut || ''}`}
              onSelect={() => executeCommand(cmd)}
            >
              <span class="category-tag">{cmd.category}</span>
              <span class="command-name">{cmd.name}</span>
              {#if cmd.shortcut}
                <kbd class="shortcut-badge">{cmd.shortcut}</kbd>
              {/if}
            </Command.Item>
          {/each}
        </Command.List>

        <footer class="palette-footer">
          <span><kbd>↑</kbd> <kbd>↓</kbd> Navegar</span>
          <span><kbd>↵</kbd> Ejecutar</span>
          <span><kbd>esc</kbd> Cerrar</span>
        </footer>
      </Command.Root>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.sr-only) {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border-width: 0;
  }

  :global(.palette-backdrop) {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background-color: rgba(10, 12, 16, 0.7);
    backdrop-filter: blur(10px);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 14vh;
    z-index: 1000;
    animation: fadeIn 0.15s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  :global(.palette-container) {
    position: fixed;
    top: 14vh;
    left: 50%;
    transform: translateX(-50%);
    width: 620px;
    max-width: 90%;
    background-color: var(--bg-primary, #ffffff);
    border-radius: 12px;
    border: 1px solid var(--border-primary, #d0d7de);
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.1);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    z-index: 1001;
    outline: none;
    animation: slideDown 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes slideDown {
    from { transform: translateX(-50%) translateY(-12px) scale(0.98); opacity: 0; }
    to { transform: translateX(-50%) translateY(0) scale(1); opacity: 1; }
  }

  :global(.palette-container .command-root) {
    display: flex;
    flex-direction: column;
    width: 100%;
    outline: none;
  }

  :global(.palette-container .input-wrapper) {
    display: flex;
    align-items: center;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    background: var(--bg-secondary, #f6f8fa);
  }

  :global(.palette-container .search-icon) {
    width: 18px;
    height: 18px;
    color: var(--text-secondary, #656d76);
    margin-right: 12px;
    flex-shrink: 0;
  }

  :global(.palette-container .command-input) {
    flex-grow: 1;
    background: transparent;
    border: none;
    color: var(--text-primary, #1f2328);
    font-size: 15px;
    outline: none;
    font-family: inherit;
  }

  :global(.palette-container .command-input::placeholder) {
    color: var(--text-secondary, #656d76);
  }

  :global(.palette-container .esc-badge) {
    font-size: 11px;
    font-family: var(--mono, monospace);
    color: var(--text-secondary, #656d76);
    background: var(--bg-primary, #ffffff);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--border-primary, #d0d7de);
  }

  :global(.palette-container .results-container) {
    max-height: 320px;
    overflow-y: auto;
    padding: 6px 0;
    outline: none;
  }

  :global(.palette-container .empty-state) {
    padding: 24px;
    text-align: center;
    color: var(--text-secondary, #656d76);
    font-size: 14px;
  }

  :global(.palette-container .command-item) {
    display: flex;
    align-items: center;
    padding: 10px 16px;
    cursor: pointer;
    font-size: 14px;
    color: var(--text-secondary, #656d76);
    transition: background-color 0.1s ease, color 0.1s ease;
    user-select: none;
    outline: none;
  }

  :global(.palette-container .command-item:hover),
  :global(.palette-container .command-item[data-selected]),
  :global(.palette-container .command-item[data-highlighted]) {
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.1));
    color: var(--text-primary, #1f2328);
  }

  :global(.palette-container .command-item[data-selected] .category-tag),
  :global(.palette-container .command-item[data-highlighted] .category-tag) {
    color: var(--accent, #0969da);
  }

  :global(.palette-container .category-tag) {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary, #656d76);
    margin-right: 12px;
    min-width: 80px;
  }

  :global(.palette-container .command-name) {
    flex-grow: 1;
  }

  :global(.palette-container .shortcut-badge) {
    font-family: var(--mono, monospace);
    font-size: 11px;
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-primary, #1f2328);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--border-primary, #d0d7de);
  }

  :global(.palette-container .palette-footer) {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    padding: 8px 16px;
    background-color: rgba(0, 0, 0, 0.03);
    border-top: 1px solid var(--border-primary, #d0d7de);
    font-size: 11px;
    color: var(--text-secondary, #656d76);
  }

  :global(.palette-container .palette-footer kbd) {
    font-family: var(--mono, monospace);
    background: var(--bg-secondary, #f6f8fa);
    padding: 1px 4px;
    border-radius: 3px;
    color: var(--text-primary, #1f2328);
    border: 1px solid var(--border-primary, #d0d7de);
  }
</style>
