<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { invokeTauri } from '@shared/api';
  import { onMount } from 'svelte';

  /** Preguntas del MCP al usuario (por ejemplo, si una bóveda puede consultar otra). */
  interface Ask {
    requestId: string;
    title: string;
    message: string;
  }

  let queue = $state<Ask[]>([]);
  const current = $derived(queue[0] ?? null);

  onMount(() => {
    const off = listen<Ask>('mcp://confirm', ({ payload }) => {
      queue = [...queue, payload];
    });
    return () => {
      off.then((f) => f());
    };
  });

  function answer(allowed: boolean) {
    const ask = queue[0];
    if (!ask) return;
    queue = queue.slice(1);
    invokeTauri('mcp_confirm_reply', { requestId: ask.requestId, allowed }).catch(() => {});
  }
</script>

{#if current}
  <div class="mcp-confirm-backdrop">
    <div class="mcp-confirm synapse-dialog" role="alertdialog" aria-modal="true" aria-labelledby="mcp-confirm-title">
      <h3 id="mcp-confirm-title">{current.title}</h3>
      <p>{current.message}</p>
      <p class="hint">Se recuerda para esta bóveda. Puedes cambiarlo en <code>.synapse/vault-access.json</code>.</p>
      <div class="actions">
        <button type="button" onclick={() => answer(false)}>No permitir</button>
        <button type="button" class="primary" onclick={() => answer(true)}>Permitir</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .mcp-confirm-backdrop {
    position: fixed;
    inset: 0;
    z-index: 3000;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.35);
  }
  .mcp-confirm {
    width: min(440px, 90vw);
    padding: 16px 18px;
    border-radius: 10px;
    border: 1px solid var(--border-primary);
    background: var(--bg-primary);
    color: var(--text-primary);
    box-shadow: var(--modal-shadow);
  }
  h3 {
    margin: 0 0 8px;
    font-size: 15px;
  }
  p {
    margin: 0 0 8px;
    font-size: 13px;
    line-height: 1.45;
  }
  .hint {
    font-size: 11.5px;
    color: var(--text-muted);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 12px;
  }
  button {
    font: inherit;
    font-size: 13px;
    padding: 5px 12px;
    border-radius: 6px;
    border: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    color: var(--text-primary);
    cursor: pointer;
  }
  button.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-contrast, #fff);
  }
</style>
