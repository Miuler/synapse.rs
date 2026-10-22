<script lang="ts">
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { isTauriEnvironment } from '@shared/repositories';
  import { Minus, Square, X } from 'lucide-svelte';

  interface Props {
    class?: string;
  }

  let { class: className = '' }: Props = $props();

  async function handleWindowControl(action: 'minimize' | 'maximize' | 'close') {
    if (!isTauriEnvironment()) {
      console.warn('handleWindowControl invocado fuera del entorno Tauri');
      return;
    }
    try {
      const appWindow = getCurrentWebviewWindow();
      if (action === 'minimize') {
        await appWindow.minimize();
      } else if (action === 'maximize') {
        await appWindow.toggleMaximize();
      } else if (action === 'close') {
        await appWindow.close();
      }
    } catch (err) {
      console.error(`Error al ejecutar control de ventana (${action}):`, err);
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="window-controls {className}"
  data-tauri-drag-region="false"
  onmousedown={(e) => e.stopPropagation()}
>
  <button
    type="button"
    class="win-btn minimize"
    onclick={(e) => {
      e.stopPropagation();
      handleWindowControl('minimize');
    }}
    title="Minimizar"
    aria-label="Minimizar ventana"
  >
    <Minus size={13} class="win-icon" />
  </button>
  <button
    type="button"
    class="win-btn maximize"
    onclick={(e) => {
      e.stopPropagation();
      handleWindowControl('maximize');
    }}
    title="Maximizar / Restaurar"
    aria-label="Maximizar o restaurar ventana"
  >
    <Square size={12} class="win-icon" />
  </button>
  <button
    type="button"
    class="win-btn close"
    onclick={(e) => {
      e.stopPropagation();
      handleWindowControl('close');
    }}
    title="Cerrar"
    aria-label="Cerrar ventana"
  >
    <X size={13} class="win-icon" />
  </button>
</div>

<style>
  .window-controls {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: 6px;
    padding-left: 6px;
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
    background-color: var(--hover-bg, rgba(0, 0, 0, 0.05));
  }

  .win-btn.close:hover {
    color: #ffffff;
    background-color: #e5534b;
  }

  :global(.win-icon) {
    width: 14px;
    height: 14px;
  }
</style>
