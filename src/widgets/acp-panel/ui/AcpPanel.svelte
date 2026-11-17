<script lang="ts">
  import { agentStore, type Conversation } from '@entities/acp-agent';
  import { AcpChat } from '@features/acp-chat';
  import { OctopusIcon } from '@shared/ui/icons';
  import {
    Bot,
    Settings,
    X,
    PictureInPicture2,
    PanelRight,
    Plus,
    FileJson,
    Trash2,
    History,
    ChevronDown,
    ChevronRight,
    Pin,
    PinOff,
    Ellipsis,
  } from 'lucide-svelte';
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
  let confirmDeleteAll = $state(false);
  /** Conversación con el menú ⋯ abierto. */
  let menuFor = $state<string | null>(null);
  $effect(() => {
    if (!menuFor) return;
    const close = () => (menuFor = null);
    window.addEventListener('click', close);
    return () => window.removeEventListener('click', close);
  });

  // Al abrir el panel se restauran las conversaciones que la bóveda tenía abiertas
  onMount(async () => {
    await agentStore.refresh();
    await agentStore.restoreVaultState();
  });
  $effect(() => {
    if (cwd) agentStore.loadHistory(cwd);
  });

  /** Con una conversación fijada, el panel acoplado se ensancha para la segunda columna. */
  const dockedWidth = $derived(agentStore.pinnedConversation ? width * 2 : width);

  function formatDate(ms: number) {
    return new Date(ms).toLocaleString(undefined, { day: '2-digit', month: 'short', hour: '2-digit', minute: '2-digit' });
  }

  function startWith(agentId: string) {
    if (cwd && agentId) agentStore.newConversation(cwd, agentId);
  }

  function onPickAgent(e: Event) {
    const sel = e.currentTarget as HTMLSelectElement;
    startWith(sel.value);
    sel.value = '';
  }

  function onPickHistory(e: Event) {
    const sel = e.currentTarget as HTMLSelectElement;
    if (cwd && sel.value) agentStore.openFromHistory(cwd, sel.value);
    sel.value = '';
  }

  function titleOf(c: Conversation) {
    return c.title || 'Conversación nueva';
  }

  // Redimensionar el panel acoplado desde su borde izquierdo
  function startResize(e: PointerEvent) {
    e.preventDefault();
    const x0 = e.clientX;
    const w0 = width;
    const factor = agentStore.pinnedConversation ? 2 : 1;
    const move = (ev: PointerEvent) => {
      width = Math.max(280, Math.min((window.innerWidth * 0.6) / factor, w0 - (ev.clientX - x0) / factor));
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
      aria-label="Nueva conversación con…"
      title="Nueva conversación con un agente"
      value=""
      onchange={onPickAgent}
      disabled={!cwd || agentStore.agents.length === 0}
    >
      <option value="" disabled>Nueva conversación con…</option>
      {#each agentStore.agents as a (a.id)}
        <option value={a.id}>{a.name}{a.installed ? '' : ' (se descargará)'}</option>
      {/each}
    </select>
    {#if agentStore.working || agentStore.pendingPermissions}
      <span class="acp-counts" title="Conversaciones trabajando y permisos pendientes">
        {#if agentStore.working}<span class="acp-brain">🧠</span>{agentStore.working}{/if}
        {#if agentStore.pendingPermissions}· ⚠ {agentStore.pendingPermissions}{/if}
      </span>
    {/if}
    <button
      type="button"
      class="acp-icon"
      title="Nueva conversación con el último agente (o el agente por defecto)"
      aria-label="Nueva conversación"
      disabled={!cwd || !agentStore.fallbackAgentId}
      onclick={() => agentStore.fallbackAgentId && startWith(agentStore.fallbackAgentId)}><Plus size={15} /></button
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
  {#if cwd}
    <div class="acp-subbar acp-history">
      <History size={13} />
      <select
        class="acp-mini-select grow"
        aria-label="Historial de conversaciones"
        title="Reabrir una conversación cerrada"
        value=""
        onchange={onPickHistory}
      >
        <option value="" disabled>
          {agentStore.closedHistory.length ? `Historial (${agentStore.closedHistory.length})` : 'Sin conversaciones cerradas'}
        </option>
        {#each agentStore.closedHistory as h (h.id)}
          <option value={h.id}>{h.title || 'Sin título'} · {h.agent_name} · {formatDate(h.updated_at)}</option>
        {/each}
      </select>
      {#if confirmDeleteAll}
        <button
          type="button"
          class="acp-danger"
          onclick={() => {
            confirmDeleteAll = false;
            if (cwd) agentStore.deleteAllTranscripts(cwd);
          }}>Borrar {agentStore.closedHistory.length}</button
        >
        <button type="button" class="acp-icon small" title="Cancelar" onclick={() => (confirmDeleteAll = false)}><X size={12} /></button>
      {:else}
        <button
          type="button"
          class="acp-link"
          title="Borrar las conversaciones cerradas de esta bóveda"
          disabled={!agentStore.closedHistory.length}
          onclick={() => (confirmDeleteAll = true)}>Borrar todas</button
        >
      {/if}
    </div>
  {/if}
{/snippet}

{#snippet convExtras(c: Conversation)}
  {#if c.session}
      {#if c.session.tool_servers.includes('synapse_instance')}
        <span class="acp-badge" title="El agente puede usar las herramientas MCP de Synapse (notas, búsqueda, Git…)">MCP Synapse</span>
      {/if}
      <div class="acp-more">
        <button
          type="button"
          class="acp-icon small"
          title="Más acciones"
          aria-label="Más acciones"
          aria-haspopup="menu"
          aria-expanded={menuFor === c.id}
          onclick={(e) => {
            e.stopPropagation();
            menuFor = menuFor === c.id ? null : c.id;
          }}><Ellipsis size={14} /></button
        >
        {#if menuFor === c.id}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_interactive_supports_focus -->
          <div class="acp-menu" role="menu" onclick={(e) => e.stopPropagation()}>
            <button
              type="button"
              role="menuitem"
              onclick={() => {
                menuFor = null;
                c.openRaw();
              }}><FileJson size={13} /> Ver en crudo (JSON)</button
            >
            <button
              type="button"
              role="menuitem"
              class="danger"
              onclick={() => {
                menuFor = null;
                if (cwd) agentStore.deleteTranscript(cwd, c.id);
              }}><Trash2 size={13} /> Borrar conversación</button
            >
          </div>
        {/if}
      </div>
  {/if}
{/snippet}

{#snippet row(c: Conversation, open: boolean, pinned: boolean)}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="acp-row" class:unseen={c.unseen} data-conversation={c.id} onclick={() => !pinned && agentStore.toggle(c.id)}>
    {#if c.incoming}
      <span class="acp-octopus" data-status={c.status} title="Abierta por otra bóveda de Synapse: {c.incoming.vault}{c.incoming.handle}"
        ><OctopusIcon size={14} /></span
      >
    {:else}
      <span class="acp-dot" data-status={c.status}></span>
    {/if}
    {#if !pinned}
      {#if open}<ChevronDown size={13} />{:else}<ChevronRight size={13} />{/if}
    {/if}
    <span class="acp-handle" title="Número para nombrarla desde otra conversación">{c.handle}</span>
    <span class="acp-row-title"
      >{#if c.incoming}<span class="acp-from-vault">⇠ {c.incoming.vault}{c.incoming.handle} ·</span>{/if}<span class="agent"
        >{c.agent?.name ?? c.agentId} ·</span
      > {titleOf(c)}</span
    >
    {#if c.status === 'working'}<span class="acp-brain" title="Pensando">🧠</span>{/if}
    {#if c.inbox.length}<span class="acp-inbox" title="Mensajes de otras conversaciones esperando a que termine su turno">✉ {c.inbox.length}</span>{/if}
    {#if c.waitingFor.length}<span class="acp-waiting" title="Le preguntó a otra conversación y espera su respuesta">esperando a {c.waitingFor.join(', ')}</span>{/if}
    {#if c.pendingPermissions}<span class="acp-perm-badge">permiso</span>{/if}
    {@render convExtras(c)}
    <button
      type="button"
      class="acp-icon small"
      title={pinned ? 'Devolver al acordeón' : 'Fijar al costado'}
      aria-label={pinned ? 'Desfijar' : 'Fijar al costado'}
      onclick={(e) => {
        e.stopPropagation();
        agentStore.togglePin(c.id);
      }}
    >
      {#if pinned}<PinOff size={13} />{:else}<Pin size={13} />{/if}
    </button>
    <button
      type="button"
      class="acp-icon small"
      title="Cerrar (queda en el historial)"
      aria-label="Cerrar conversación"
      onclick={(e) => {
        e.stopPropagation();
        agentStore.close(c.id);
      }}><X size={13} /></button
    >
  </div>
{/snippet}

{#snippet content()}
  {#if agentStore.agents.length === 0}
    <div class="acp-empty-panel">
      <Bot size={28} />
      <p>No hay agentes configurados.</p>
      {#if agentStore.listError}<p class="acp-error-text">{agentStore.listError}</p>{/if}
      <button type="button" class="acp-link" onclick={onOpenSettings}>Configurar agentes</button>
    </div>
  {:else if !cwd}
    <div class="acp-empty-panel">
      <Bot size={28} />
      <p>Abre una bóveda para conversar con un agente.</p>
    </div>
  {:else if agentStore.conversations.length === 0}
    <div class="acp-empty-panel">
      <Bot size={28} />
      <p>Empieza una conversación con un agente, o reabre una del historial.</p>
      <div class="acp-start">
        {#each agentStore.agents as a (a.id)}
          <button type="button" onclick={() => startWith(a.id)}>{a.name}</button>
        {/each}
      </div>
    </div>
  {:else}
    <div class="acp-columns">
      {#if agentStore.pinnedConversation}
        {@const c = agentStore.pinnedConversation}
        <section class="acp-pinned" aria-label="Conversación fijada">
          {@render row(c, true, true)}
          {#key c.id}<AcpChat {cwd} {onOpenSettings} {onOpenNote} conv={c} />{/key}
        </section>
      {/if}
      <div class="acp-accordion">
        {#each agentStore.conversations.filter((x) => x.id !== agentStore.pinned) as c (c.id)}
          {@const open = agentStore.expanded.includes(c.id)}
          <section class="acp-section" class:open aria-label="Conversación {titleOf(c)}">
            {@render row(c, open, false)}
            {#if open}
              <div class="acp-section-body">
                <AcpChat {cwd} {onOpenSettings} {onOpenNote} conv={c} />
              </div>
            {/if}
          </section>
        {/each}
      </div>
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
    {@render content()}
  </section>
{:else}
  <aside class="acp-panel docked synapse-agent-panel synapse-right-sidebar" style="width: {dockedWidth}px;" aria-label="Agente">
    <div
      role="separator"
      tabindex="-1"
      aria-label="Redimensionar panel del agente"
      class="acp-resizer"
      onpointerdown={startResize}
    ></div>
    {@render header()}
    {@render content()}
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
  .acp-columns {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .acp-pinned {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    border-right: 1px solid var(--border-primary);
  }
  .acp-accordion {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
  }
  .acp-section {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-bottom: 1px solid var(--border-primary);
  }
  .acp-section.open {
    flex: 1;
    min-height: 220px;
  }
  .acp-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    font-size: 12px;
    cursor: pointer;
    user-select: none;
    background: var(--bg-primary);
  }
  .acp-row:hover {
    background: var(--bg-secondary);
  }
  .acp-section.open > .acp-row {
    background: var(--bg-secondary);
  }
  .acp-row-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .acp-row-title .agent {
    color: var(--text-muted);
  }
  .acp-row.unseen .acp-row-title {
    font-weight: 600;
  }
  .acp-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
    background: var(--text-muted);
  }
  .acp-dot[data-status='working'] {
    background: var(--success, #1d9e75);
  }
  .acp-dot[data-status='permission'] {
    background: var(--warning, #ba7517);
  }
  .acp-dot[data-status='error'] {
    background: var(--danger);
  }
  .acp-perm-badge {
    font-size: 10px;
    padding: 0 6px;
    border-radius: 999px;
    color: var(--warning, #9a6700);
    border: 1px solid color-mix(in srgb, var(--warning, #9a6700) 50%, transparent);
  }
  .acp-section-body {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }
  .acp-counts {
    font-size: 11px;
    color: var(--text-muted);
    white-space: nowrap;
  }
  .acp-empty-panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    flex: 1;
    padding: 24px 16px;
    color: var(--text-muted);
    text-align: center;
    font-size: 13px;
  }
  .acp-start {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
    max-width: 260px;
  }
  .acp-start button {
    font: inherit;
    font-size: 12px;
    padding: 6px 10px;
    border-radius: 6px;
    border: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    color: var(--text-primary);
    cursor: pointer;
  }
  .acp-start button:hover {
    border-color: var(--accent);
  }
  .acp-brain {
    font-size: 12px;
  }
  .acp-error-text {
    color: var(--danger);
    font-size: 12px;
  }
  .acp-more {
    position: relative;
    margin-left: auto;
  }
  .acp-badge + .acp-more {
    margin-left: 0;
  }
  .acp-row .acp-badge,
  .acp-row .acp-more {
    margin-left: 0;
    flex-shrink: 0;
  }
  .acp-menu {
    position: absolute;
    right: 0;
    top: calc(100% + 4px);
    z-index: 20;
    display: flex;
    flex-direction: column;
    min-width: 190px;
    padding: 4px;
    border-radius: 8px;
    border: 1px solid var(--border-primary);
    background: var(--bg-primary);
    box-shadow: var(--modal-shadow);
  }
  .acp-menu button {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border: none;
    border-radius: 5px;
    background: none;
    font: inherit;
    font-size: 12px;
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
  }
  .acp-menu button:hover {
    background: var(--bg-tertiary);
  }
  .acp-menu button.danger {
    color: var(--danger);
  }
  .acp-handle {
    font-size: 10.5px;
    font-family: var(--mono, monospace);
    color: var(--text-muted);
  }
  .acp-inbox,
  .acp-waiting {
    font-size: 10px;
    white-space: nowrap;
    padding: 0 6px;
    border-radius: 999px;
    color: var(--accent);
    border: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
  }
  .acp-octopus {
    display: inline-flex;
    line-height: 1;
    flex: none;
    color: var(--accent);
  }
  .acp-octopus[data-status='working'] {
    animation: acpOctopus 1.4s ease-in-out infinite;
  }
  @keyframes acpOctopus {
    50% {
      transform: translateY(-1px) rotate(-8deg);
    }
  }
  .acp-from-vault {
    color: var(--accent);
    margin-right: 4px;
  }
</style>
