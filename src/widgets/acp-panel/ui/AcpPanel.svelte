<script lang="ts">
  import { agentStore } from '@entities/acp-agent';
  import { AcpChat } from '@features/acp-chat';
  import { Bot, Settings, X, PictureInPicture2, PanelRight, Plus, FileJson, Trash2, History } from 'lucide-svelte';
  import { onMount } from 'svelte';

  interface Props {
    /** Carpeta de trabajo (bóveda abierta). */
    cwd: string | null;
    onClose: () => void;
    onOpenSettings: () => void;
    /** Abre una nota de la bóveda (ruta relativa) desde un enlace del agente. */
    onOpenNote?: (relativePath: string) => void;
  }

  let { cwd, onClose, onOpenSettings, onOpenNote }: Props = $props();

  // Historial de conversaciones de la bóveda
  let confirmDeleteAll = $state(false);
  $effect(() => {
    if (cwd) agentStore.loadHistory(cwd);
  });

  function formatDate(ms: number) {
    const d = new Date(ms);
    return d.toLocaleString(undefined, { day: '2-digit', month: 'short', hour: '2-digit', minute: '2-digit' });
  }

  function onPickHistory(e: Event) {
    const id = (e.currentTarget as HTMLSelectElement).value;
    if (!cwd) return;
    if (id === '__new__') agentStore.newSession(cwd);
    else if (id) agentStore.resume(cwd, id);
  }

  const inHistory = $derived(agentStore.history.some((h) => h.id === agentStore.transcriptId));

  const WIDTH_KEY = 'synapse.agent.width';
  const POS_KEY = 'synapse.agent.float';

  function loadJson<T>(key: string, fallback: T): T {
    try {
      return JSON.parse(localStorage.getItem(key) ?? 'null') ?? fallback;
    } catch {
      return fallback;
    }
  }
  function saveJson(key: string, value: unknown) {
    try {
      localStorage.setItem(key, JSON.stringify(value));
    } catch {
      /* sin almacenamiento */
    }
  }

  let width = $state<number>(loadJson(WIDTH_KEY, 380));
  let float = $state<{ x: number; y: number; w: number; h: number }>(
    loadJson(POS_KEY, { x: Math.max(16, window.innerWidth - 440), y: 80, w: 420, h: 560 }),
  );

  // Al abrir el panel, el último agente usado retoma su última conversación (si ya está instalado)
  onMount(async () => {
    await agentStore.refresh();
    const agent = agentStore.activeAgent;
    if (cwd && agent?.installed && !agentStore.session && agentStore.connection === 'idle' && !agentStore.items.length) {
      agentStore.connectLatest(cwd);
    }
  });

  function onSelect(e: Event) {
    const id = (e.currentTarget as HTMLSelectElement).value;
    if (id && cwd) agentStore.select(id, cwd);
  }

  // Redimensionar el panel acoplado desde su borde izquierdo
  function startResize(e: PointerEvent) {
    e.preventDefault();
    const x0 = e.clientX;
    const w0 = width;
    const move = (ev: PointerEvent) => {
      width = Math.max(280, Math.min(window.innerWidth * 0.6, w0 - (ev.clientX - x0)));
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      saveJson(WIDTH_KEY, width);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  }

  // Mover el panel flotante arrastrando la cabecera
  function startDrag(e: PointerEvent) {
    if (!agentStore.floating || e.button !== 0) return;
    if ((e.target as HTMLElement).closest('button, select')) return;
    e.preventDefault();
    const x0 = e.clientX - float.x;
    const y0 = e.clientY - float.y;
    const move = (ev: PointerEvent) => {
      float = {
        ...float,
        x: Math.max(0, Math.min(window.innerWidth - 120, ev.clientX - x0)),
        y: Math.max(0, Math.min(window.innerHeight - 40, ev.clientY - y0)),
      };
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      saveJson(POS_KEY, float);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  }

  // Guardar el tamaño del panel flotante cuando el usuario lo cambia con la esquina
  let floatEl = $state<HTMLElement | null>(null);
  $effect(() => {
    if (!floatEl) return;
    const obs = new ResizeObserver(() => {
      if (!floatEl) return;
      float = { ...float, w: floatEl.offsetWidth, h: floatEl.offsetHeight };
      saveJson(POS_KEY, float);
    });
    obs.observe(floatEl);
    return () => obs.disconnect();
  });
</script>

{#snippet header()}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header class="acp-header synapse-header" class:draggable={agentStore.floating} onpointerdown={startDrag}>
    <Bot size={15} />
    <select
      class="acp-select"
      aria-label="Agente"
      value={agentStore.activeAgentId ?? ''}
      onchange={onSelect}
      disabled={!cwd || agentStore.agents.length === 0}
    >
      <option value="" disabled>Elegir agente…</option>
      {#each agentStore.agents as a (a.id)}
        <option value={a.id}>{a.name}{a.installed ? '' : ' (se descargará)'}</option>
      {/each}
    </select>
    <button
      type="button"
      class="acp-icon"
      title="Nueva conversación"
      aria-label="Nueva conversación"
      disabled={!cwd || !agentStore.activeAgent}
      onclick={() => cwd && agentStore.newSession(cwd)}><Plus size={15} /></button
    >
    <button
      type="button"
      class="acp-icon"
      title="Ver la conversación en crudo (JSON, se abre con la aplicación por defecto)"
      aria-label="Ver conversación en crudo"
      disabled={!inHistory}
      onclick={() => agentStore.openRaw()}><FileJson size={14} /></button
    >
    <button
      type="button"
      class="acp-icon"
      title={agentStore.floating ? 'Acoplar a la derecha' : 'Modo flotante'}
      aria-pressed={agentStore.floating}
      onclick={() => agentStore.setFloating(!agentStore.floating)}
    >
      {#if agentStore.floating}<PanelRight size={14} />{:else}<PictureInPicture2 size={14} />{/if}
    </button>
    <button type="button" class="acp-icon" title="Ajustes de agentes" onclick={onOpenSettings}>
      <Settings size={14} />
    </button>
    <button type="button" class="acp-icon" title="Cerrar panel" onclick={onClose}><X size={14} /></button>
  </header>
  {#if cwd && agentStore.activeAgent}
    <div class="acp-subbar acp-history">
      <History size={13} />
      <select
        class="acp-mini-select grow"
        aria-label="Historial de conversaciones"
        title="Retomar una conversación anterior"
        value={inHistory ? agentStore.transcriptId : ''}
        onchange={onPickHistory}
      >
        <option value="" disabled>{agentStore.history.length ? 'Conversación actual' : 'Sin conversaciones guardadas'}</option>
        <option value="__new__">＋ Nueva conversación</option>
        {#each agentStore.history as h (h.id)}
          <option value={h.id}>{h.title || 'Sin título'} · {h.agent_name} · {formatDate(h.updated_at)}</option>
        {/each}
      </select>
      <button
        type="button"
        class="acp-icon small"
        title="Borrar esta conversación del historial"
        aria-label="Borrar conversación"
        disabled={!inHistory}
        onclick={() => cwd && agentStore.transcriptId && agentStore.deleteTranscript(cwd, agentStore.transcriptId)}
        ><Trash2 size={13} /></button
      >
      {#if confirmDeleteAll}
        <button
          type="button"
          class="acp-danger"
          onclick={() => {
            confirmDeleteAll = false;
            if (cwd) agentStore.deleteAllTranscripts(cwd);
          }}>Borrar {agentStore.history.length}</button
        >
        <button type="button" class="acp-icon small" title="Cancelar" onclick={() => (confirmDeleteAll = false)}><X size={12} /></button>
      {:else}
        <button
          type="button"
          class="acp-link"
          title="Borrar todas las conversaciones de esta bóveda"
          disabled={!agentStore.history.length}
          onclick={() => (confirmDeleteAll = true)}>Borrar todas</button
        >
      {/if}
    </div>
  {/if}
  {#if agentStore.session && (agentStore.session.modes.length || agentStore.session.models.length || agentStore.session.tool_servers.length)}
    <div class="acp-subbar">
      {#if agentStore.session.modes.length}
        <select
          class="acp-mini-select"
          aria-label="Modo del agente"
          title="Modo del agente"
          value={agentStore.currentMode ?? ''}
          onchange={(e) => agentStore.setMode(e.currentTarget.value)}
        >
          {#each agentStore.session.modes as m (m.id)}
            <option value={m.id} title={m.description ?? ''}>{m.name || m.id}</option>
          {/each}
        </select>
      {/if}
      {#if agentStore.session.models.length}
        <select
          class="acp-mini-select"
          aria-label="Modelo"
          title="Modelo"
          value={agentStore.currentModel ?? ''}
          onchange={(e) => agentStore.setModel(e.currentTarget.value)}
        >
          {#each agentStore.session.models as m (m.id)}
            <option value={m.id}>{m.name || m.id}</option>
          {/each}
        </select>
      {/if}
      {#if agentStore.session.tool_servers.includes('synapse_instance')}
        <span class="acp-badge" title="El agente puede usar las herramientas MCP de Synapse (notas, búsqueda, Git…)">MCP Synapse</span>
      {/if}
    </div>
  {/if}
{/snippet}

{#if agentStore.floating}
  <section
    class="acp-panel floating synapse-agent-panel"
    bind:this={floatEl}
    style="left: {float.x}px; top: {float.y}px; width: {float.w}px; height: {float.h}px;"
    aria-label="Agente"
  >
    {@render header()}
    <AcpChat {cwd} {onOpenSettings} {onOpenNote} />
  </section>
{:else}
  <aside class="acp-panel docked synapse-agent-panel synapse-right-sidebar" style="width: {width}px;" aria-label="Agente">
    <div
      role="separator"
      tabindex="-1"
      aria-label="Redimensionar panel del agente"
      class="acp-resizer"
      onpointerdown={startResize}
    ></div>
    {@render header()}
    <AcpChat {cwd} {onOpenSettings} {onOpenNote} />
  </aside>
{/if}

<style>
  .acp-panel {
    display: flex;
    flex-direction: column;
    background: var(--bg-primary);
    color: var(--text-primary);
    min-height: 0;
  }
  .acp-panel.docked {
    position: relative;
    flex-shrink: 0;
    height: 100%;
    border-left: 1px solid var(--border-primary);
  }
  .acp-panel.floating {
    position: fixed;
    z-index: 900;
    min-width: 300px;
    min-height: 260px;
    resize: both;
    overflow: hidden;
    border: 1px solid var(--border-primary);
    border-radius: var(--island-radius, 10px);
    box-shadow: var(--modal-shadow);
  }
  .acp-resizer {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 6px;
    cursor: ew-resize;
    z-index: 2;
  }
  .acp-header {
    /* Misma altura que la barra de pestañas, para que sus líneas inferiores coincidan */
    box-sizing: border-box;
    height: var(--header-height, 42px);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-primary);
    user-select: none;
  }
  .acp-header.draggable {
    cursor: move;
  }
  .acp-subbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
  }
  .acp-mini-select {
    max-width: 48%;
    font: inherit;
    font-size: 11px;
    padding: 1px 4px;
    border-radius: 5px;
    border: 1px solid var(--border-primary);
    background: var(--bg-primary);
    color: var(--text-primary);
  }
  .acp-badge {
    margin-left: auto;
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 999px;
    color: var(--accent);
    border: 1px solid color-mix(in srgb, var(--accent) 45%, transparent);
  }
  .acp-select {
    flex: 1;
    min-width: 0;
    font: inherit;
    font-size: 12px;
    padding: 3px 6px;
    border-radius: 6px;
    border: 1px solid var(--border-primary);
    background: var(--bg-primary);
    color: var(--text-primary);
  }
  .acp-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .acp-icon:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .acp-icon.small {
    width: 22px;
    height: 22px;
  }
  .acp-history {
    flex-wrap: nowrap;
    color: var(--text-muted);
  }
  .acp-mini-select.grow {
    flex: 1;
    min-width: 0;
    max-width: none;
  }
  .acp-link,
  .acp-danger {
    font: inherit;
    font-size: 11px;
    white-space: nowrap;
    border-radius: 5px;
    cursor: pointer;
  }
  .acp-link {
    border: none;
    background: none;
    padding: 2px 4px;
    color: var(--text-secondary);
  }
  .acp-link:hover:not(:disabled) {
    color: var(--danger);
  }
  .acp-link:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .acp-danger {
    padding: 2px 8px;
    border: 1px solid var(--danger-border);
    background: var(--danger-bg);
    color: var(--danger);
  }
  .acp-icon:hover:not(:disabled) {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }
</style>
