<script lang="ts">
  import { commandRegistry, type AppCommand } from '../services/commands.svelte';

  interface Props {
    isOpen?: boolean;
    onClose?: () => void;
  }

  let { isOpen = $bindable(false), onClose }: Props = $props();

  let searchQuery = $state('');
  let selectedIndex = $state(0);
  let inputElement = $state<HTMLInputElement | null>(null);

  // Obtener comandos filtrados dinámicamente según searchQuery
  let filteredCommands = $derived(commandRegistry.search(searchQuery));

  // Escuchar atajo global Ctrl+P o Cmd+P para abrir/cerrar la paleta
  $effect(() => {
    const handleKeydown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'p') {
        e.preventDefault();
        isOpen = !isOpen;
        if (isOpen) {
          resetState();
        }
      }
    };

    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  });

  // Enfocar input automáticamente cuando el modal se abre
  $effect(() => {
    if (isOpen) {
      resetState();
      setTimeout(() => inputElement?.focus(), 40);
    }
  });

  function resetState() {
    selectedIndex = 0;
    searchQuery = '';
  }

  function closePalette() {
    isOpen = false;
    if (onClose) onClose();
  }

  function executeCommand(cmd: AppCommand) {
    cmd.action();
    closePalette();
  }

  function handleInputKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (filteredCommands.length > 0) {
        selectedIndex = (selectedIndex + 1) % filteredCommands.length;
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (filteredCommands.length > 0) {
        selectedIndex = (selectedIndex - 1 + filteredCommands.length) % filteredCommands.length;
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (filteredCommands[selectedIndex]) {
        executeCommand(filteredCommands[selectedIndex]);
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      closePalette();
    }
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="backdrop" onclick={closePalette}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="palette-container" onclick={(e) => e.stopPropagation()}>
      <div class="input-wrapper">
        <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="11" cy="11" r="8"/>
          <line x1="21" y1="21" x2="16.65" y2="16.65"/>
        </svg>
        <input
          bind:this={inputElement}
          bind:value={searchQuery}
          onkeydown={handleInputKeydown}
          placeholder="Escribe un comando o busca..."
          type="text"
        />
        <span class="esc-badge">ESC</span>
      </div>

      <div class="results-container">
        {#if filteredCommands.length === 0}
          <div class="empty-state">No se encontraron comandos</div>
        {:else}
          <ul class="command-list">
            {#each filteredCommands as cmd, index}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <li
                class="command-item"
                class:selected={index === selectedIndex}
                onclick={() => executeCommand(cmd)}
                onmouseenter={() => (selectedIndex = index)}
              >
                <span class="category-tag">{cmd.category}</span>
                <span class="command-name">{cmd.name}</span>
                {#if cmd.shortcut}
                  <kbd class="shortcut-badge">{cmd.shortcut}</kbd>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <footer class="palette-footer">
        <span><kbd>↑</kbd> <kbd>↓</kbd> Navegar</span>
        <span><kbd>↵</kbd> Ejecutar</span>
        <span><kbd>esc</kbd> Cerrar</span>
      </footer>
    </div>
  </div>
{/if}

<style>
  .backdrop {
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

  .palette-container {
    width: 620px;
    max-width: 90%;
    background-color: var(--bg-dark, #121418);
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.7);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: slideDown 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes slideDown {
    from { transform: translateY(-12px) scale(0.98); opacity: 0; }
    to { transform: translateY(0) scale(1); opacity: 1; }
  }

  .input-wrapper {
    display: flex;
    align-items: center;
    padding: 14px 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    background: rgba(255, 255, 255, 0.02);
  }

  .search-icon {
    width: 18px;
    height: 18px;
    color: #6e7681;
    margin-right: 12px;
  }

  input {
    flex-grow: 1;
    background: transparent;
    border: none;
    color: #f0f6fc;
    font-size: 15px;
    outline: none;
    font-family: inherit;
  }

  input::placeholder {
    color: #6e7681;
  }

  .esc-badge {
    font-size: 11px;
    font-family: var(--mono, monospace);
    color: #6e7681;
    background: rgba(255, 255, 255, 0.05);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .results-container {
    max-height: 320px;
    overflow-y: auto;
    padding: 6px 0;
  }

  .empty-state {
    padding: 24px;
    text-align: center;
    color: #6e7681;
    font-size: 14px;
  }

  .command-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .command-item {
    display: flex;
    align-items: center;
    padding: 10px 16px;
    cursor: pointer;
    font-size: 14px;
    color: #8b949e;
    transition: background-color 0.1s ease, color 0.1s ease;
  }

  .command-item.selected {
    background-color: rgba(51, 204, 255, 0.1);
    color: #f0f6fc;
  }

  .command-item.selected .category-tag {
    color: var(--cyan, #33ccff);
  }

  .category-tag {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #6e7681;
    margin-right: 12px;
    min-width: 80px;
  }

  .command-name {
    flex-grow: 1;
  }

  .shortcut-badge {
    font-family: var(--mono, monospace);
    font-size: 11px;
    background: rgba(255, 255, 255, 0.06);
    color: #c9d1d9;
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .palette-footer {
    display: flex;
    gap: 16px;
    padding: 8px 16px;
    background-color: rgba(0, 0, 0, 0.2);
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    font-size: 11px;
    color: #6e7681;
  }

  .palette-footer kbd {
    font-family: var(--mono, monospace);
    background: rgba(255, 255, 255, 0.06);
    padding: 1px 4px;
    border-radius: 3px;
    color: #8b949e;
  }
</style>
