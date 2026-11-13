<script lang="ts">
  import { agentStore, type Attachment, type ToolContent } from '@entities/acp-agent';
  import { vaultItemStore } from '@entities/vault-item';
  import {
    Bot,
    Send,
    Square,
    Wrench,
    ShieldAlert,
    Settings,
    AlertCircle,
    Loader2,
    FileText,
    TextSelect,
    ImageIcon,
    X,
    Circle,
    CircleCheck,
    CircleDot,
  } from 'lucide-svelte';
  import { tick } from 'svelte';
  import { renderAgentMarkdown, vaultPathOfLink, formatRaw } from '../lib/markdown';

  interface Props {
    /** Carpeta de trabajo del agente (la bóveda abierta). */
    cwd: string | null;
    onOpenSettings?: () => void;
    /** Abre en Synapse una nota de la bóveda (ruta relativa). */
    onOpenNote?: (relativePath: string) => void;
  }

  let { cwd, onOpenSettings, onOpenNote }: Props = $props();

  // Enlaces en las respuestas: notas de la bóveda se abren en Synapse; el resto, fuera
  function onMessageClick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest('a');
    if (!a) return;
    const href = a.getAttribute('href') ?? '';
    e.preventDefault();
    const rel = vaultPathOfLink(href, cwd);
    if (rel) onOpenNote?.(rel);
    else if (/^https?:/i.test(href)) window.open(href, '_blank', 'noopener');
  }

  let draft = $state('');
  let scroller = $state<HTMLDivElement | null>(null);
  let textarea = $state<HTMLTextAreaElement | null>(null);
  let dragOver = $state(false);
  let pickIndex = $state(0);

  const busy = $derived(agentStore.connection === 'busy');
  const ready = $derived(agentStore.connection === 'ready' || busy);

  // Bajar al final cuando llega contenido nuevo
  $effect(() => {
    const _n = agentStore.items.length;
    const last = agentStore.items.at(-1);
    const _t = last && 'text' in last ? last.text.length : 0;
    tick().then(() => scroller?.scrollTo({ top: scroller.scrollHeight }));
  });

  const absPath = (p: string) => (p.startsWith('/') || !cwd ? p : `${cwd.replace(/\/$/, '')}/${p}`);
  const nameOf = (p: string) => p.split('/').pop() || p;
  const relOf = (p: string) => (cwd && p.startsWith(cwd) ? p.slice(cwd.length).replace(/^\//, '') : p);

  // Autocompletado: `@nota` menciona una nota, `/comando` al inicio usa un comando del agente
  let caret = $state(0);
  const mention = $derived.by(() => {
    const m = /(^|\s)@([^\s@]*)$/.exec(draft.slice(0, caret));
    return m ? { query: m[2].toLowerCase(), start: caret - m[2].length - 1 } : null;
  });
  const slash = $derived(/^\/(\S*)$/.exec(draft)?.[1]?.toLowerCase() ?? null);

  function fuzzy(text: string, q: string): boolean {
    let i = 0;
    for (const c of text) if (c === q[i]) i++;
    return i === q.length;
  }

  const suggestions = $derived.by((): { label: string; detail: string; pick: () => void }[] => {
    if (mention) {
      // `@this`: el archivo de la pestaña que se está viendo
      const note = agentStore.activeNote;
      const self =
        note && 'this'.startsWith(mention.query)
          ? [
              {
                label: '@this',
                detail: `Archivo actual · ${relOf(note.path)}`,
                pick: () => {
                  const m = mention;
                  draft = draft.slice(0, m.start) + draft.slice(caret);
                  agentStore.attachActiveNote();
                  tick().then(() => {
                    textarea?.focus();
                    textarea?.setSelectionRange(m.start, m.start);
                    caret = m.start;
                  });
                },
              },
            ]
          : [];
      return [...self, ...vaultItemStore.items
        .filter((it) => fuzzy(it.relative_path.toLowerCase(), mention.query))
        .slice(0, 8)
        .map((it) => ({
          label: it.title || nameOf(it.relative_path),
          detail: it.relative_path,
          pick: () => {
            const m = mention;
            draft = draft.slice(0, m.start) + draft.slice(caret);
            agentStore.attach({ kind: 'file', path: absPath(it.abs_path ?? it.relative_path), name: nameOf(it.relative_path) });
            tick().then(() => {
              textarea?.focus();
              textarea?.setSelectionRange(m.start, m.start);
              caret = m.start;
            });
          },
        }))];
    }
    if (slash !== null) {
      return agentStore.commands
        .filter((c) => c.name.toLowerCase().startsWith(slash))
        .slice(0, 8)
        .map((c) => ({
          label: `/${c.name}`,
          detail: c.description ?? '',
          pick: () => {
            draft = `/${c.name} `;
            tick().then(() => textarea?.focus());
          },
        }));
    }
    return [];
  });

  $effect(() => {
    const _n = suggestions.length;
    pickIndex = 0;
  });

  function submit() {
    const text = draft.trim();
    if (!text || !ready || busy) return;
    draft = '';
    agentStore.send(text);
  }

  function onKeydown(e: KeyboardEvent) {
    if (suggestions.length) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const d = e.key === 'ArrowDown' ? 1 : -1;
        pickIndex = (pickIndex + d + suggestions.length) % suggestions.length;
        return;
      }
      if (e.key === 'Enter' || e.key === 'Tab') {
        e.preventDefault();
        suggestions[pickIndex].pick();
        return;
      }
    }
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      submit();
    }
  }

  // Arrastrar notas desde el explorador al chat
  const DRAG_MIME = 'application/x-synapse-vault-paths';
  function onDrop(e: DragEvent) {
    dragOver = false;
    const raw = e.dataTransfer?.getData(DRAG_MIME);
    if (!raw) return;
    e.preventDefault();
    try {
      for (const p of JSON.parse(raw) as string[]) agentStore.attach({ kind: 'file', path: absPath(p), name: nameOf(p) });
    } catch {
      /* datos de arrastre inválidos */
    }
  }

  // Pegar imágenes del portapapeles (capturas, imágenes copiadas) como adjunto del mensaje
  let pasted = 0;
  const imageTitle = $derived(
    agentStore.session && !agentStore.session.capabilities.image
      ? 'Este agente no acepta imágenes: se le indicará que hay una imagen sin enviarla'
      : 'Imagen pegada',
  );
  function onPaste(e: ClipboardEvent) {
    const files = [...(e.clipboardData?.items ?? [])]
      .filter((i) => i.kind === 'file' && i.type.startsWith('image/'))
      .map((i) => i.getAsFile())
      .filter((f): f is File => !!f);
    if (!files.length) return;
    e.preventDefault();
    for (const file of files) {
      const reader = new FileReader();
      reader.onload = () => {
        const url = String(reader.result);
        const data = url.slice(url.indexOf(',') + 1);
        pasted += 1;
        const ext = file.type.split('/')[1] ?? 'png';
        agentStore.attach({ kind: 'image', path: `pegada-${Date.now()}-${pasted}`, name: `Imagen ${pasted}.${ext}`, mimeType: file.type, data });
      };
      reader.readAsDataURL(file);
    }
  }

  function chipLabel(a: Attachment) {
    if (a.kind !== 'selection') return a.name;
    return a.startLine ? `${a.name} L${a.startLine}–${a.endLine}` : `${a.name} (fragmento)`;
  }

  /** Diff por líneas, simple: lo quitado y lo agregado. */
  function diffLines(c: ToolContent) {
    const oldLines = (c.oldText ?? '').split('\n');
    const newLines = (c.newText ?? '').split('\n');
    const oldSet = new Set(oldLines);
    const newSet = new Set(newLines);
    return [
      ...oldLines.filter((l) => !newSet.has(l)).map((l) => ({ sign: '-', text: l })),
      ...newLines.filter((l) => !oldSet.has(l)).map((l) => ({ sign: '+', text: l })),
    ];
  }

  const statusText: Record<string, string> = {
    installing: 'Descargando e instalando el agente…',
    starting: 'Iniciando el agente…',
  };

  const compact = (n: number) =>
    n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)}M` : n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n);
  function formatCost(c: { amount: number; currency: string }) {
    try {
      return new Intl.NumberFormat(undefined, { style: 'currency', currency: c.currency || 'USD', maximumFractionDigits: 4 }).format(c.amount);
    } catch {
      return `${c.amount} ${c.currency}`;
    }
  }
  const usagePct = $derived(agentStore.usage ? Math.min(100, (agentStore.usage.used / agentStore.usage.size) * 100) : 0);
</script>

<div class="acp-chat">
  {#if agentStore.agents.length === 0}
    <div class="acp-empty">
      <Bot size={28} />
      <p>No hay agentes configurados.</p>
      {#if agentStore.listError}<p class="acp-error-text">{agentStore.listError}</p>{/if}
      <button type="button" class="acp-btn primary" onclick={() => onOpenSettings?.()}>
        <Settings size={14} /> Configurar agentes
      </button>
    </div>
  {:else if !agentStore.activeAgent}
    <div class="acp-empty">
      <Bot size={28} />
      <p>Elige un agente en la cabecera para empezar.</p>
    </div>
  {:else if !cwd}
    <div class="acp-empty">
      <Bot size={28} />
      <p>Abre una bóveda para conversar con el agente.</p>
    </div>
  {:else}
    <div class="acp-messages" bind:this={scroller}>
      {#if agentStore.connection === 'installing' || agentStore.connection === 'starting'}
        <div class="acp-status"><Loader2 size={14} class="acp-spin" /> {statusText[agentStore.connection]}</div>
      {:else if agentStore.connection === 'idle' && agentStore.items.length === 0}
        <div class="acp-empty">
          <button type="button" class="acp-btn primary" onclick={() => agentStore.connectLatest(cwd)}>
            <Bot size={14} /> Conectar con {agentStore.activeAgent.name}
          </button>
        </div>
      {/if}

      {#if agentStore.plan.length}
        <div class="acp-plan" aria-label="Plan del agente">
          {#each agentStore.plan as entry, i (i)}
            <div class="acp-plan-entry" data-status={entry.status}>
              {#if entry.status === 'completed'}<CircleCheck size={13} />{:else if entry.status === 'in_progress'}<CircleDot
                  size={13}
                />{:else}<Circle size={13} />{/if}
              <span>{entry.content}</span>
            </div>
          {/each}
        </div>
      {/if}

      {#each agentStore.items as item (item.id)}
        {#if item.type === 'user'}
          <div class="acp-msg user">
            {item.text}
            {#if item.attachments.length}
              <div class="acp-chips sent">
                {#each item.attachments as a, i (i)}
                  {#if a.kind === 'image'}
                    <img class="acp-sent-image" src="data:{a.mimeType};base64,{a.data}" alt={a.name} title={a.name} />
                  {:else}
                    <span class="acp-chip" title={a.path}
                      >{#if a.kind === 'selection'}<TextSelect size={11} />{:else}<FileText size={11} />{/if}{chipLabel(a)}</span
                    >
                  {/if}
                {/each}
              </div>
            {/if}
          </div>
        {:else if item.type === 'agent'}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div class="acp-msg agent markdown" onclick={onMessageClick}>{@html renderAgentMarkdown(item.text)}</div>
        {:else if item.type === 'thought'}
          <details class="acp-thought"><summary>Razonamiento</summary>{item.text}</details>
        {:else if item.type === 'tool'}
          <details class="acp-tool" data-status={item.status ?? ''}>
            <summary>
              <Wrench size={13} />
              <span class="acp-tool-title">{item.title}</span>
              {#if item.status}<span class="acp-tool-status">{item.status}</span>{/if}
            </summary>
            {#if item.kind}<div class="acp-tool-loc">Tipo: {item.kind}</div>{/if}
            {#each item.locations as loc, i (i)}
              <div class="acp-tool-loc">{relOf(loc.path)}{loc.line ? `:${loc.line}` : ''}</div>
            {/each}
            {#if item.rawInput != null}
              <div class="acp-tool-label">Entrada</div>
              <pre class="acp-tool-text">{formatRaw(item.rawInput)}</pre>
            {/if}
            {#each item.content as c, i (i)}
              {#if c.type === 'diff'}
                <div class="acp-diff">
                  <div class="acp-diff-path">{relOf(c.path ?? '')}</div>
                  {#each diffLines(c) as l, j (j)}
                    <div class="acp-diff-line {l.sign === '+' ? 'add' : 'del'}">{l.sign} {l.text}</div>
                  {/each}
                </div>
              {:else if c.text}
                <pre class="acp-tool-text">{c.text}</pre>
              {/if}
            {/each}
            {#if item.rawOutput != null}
              <div class="acp-tool-label">Salida</div>
              <pre class="acp-tool-text">{formatRaw(item.rawOutput)}</pre>
            {/if}
            {#if item.rawInput == null && item.rawOutput == null && !item.content.length && !item.locations.length}
              <div class="acp-tool-loc">El agente no envió más detalles de esta herramienta.</div>
            {/if}
          </details>
        {:else if item.type === 'permission'}
          <div class="acp-permission" class:answered={item.answered !== null}>
            <div class="acp-permission-title"><ShieldAlert size={14} /> {item.request.title}</div>
            {#if item.answered === null}
              <div class="acp-permission-actions">
                {#each item.request.options as opt (opt.id)}
                  <button
                    type="button"
                    class="acp-btn"
                    class:primary={opt.kind.startsWith('allow')}
                    class:danger={opt.kind.startsWith('reject')}
                    onclick={() => agentStore.answer(item.id, opt.id)}>{opt.name}</button
                  >
                {/each}
              </div>
            {:else}
              <div class="acp-permission-done">
                {item.request.options.find((o) => o.id === item.answered)?.name ?? 'Cancelado'}
              </div>
            {/if}
          </div>
        {:else if item.type === 'error'}
          <div class="acp-msg error"><AlertCircle size={13} /> {item.text}</div>
        {/if}
      {/each}
      {#if busy}
        <div class="acp-status"><Loader2 size={14} class="acp-spin" /> Pensando…</div>
      {/if}
    </div>

    {#if agentStore.usage}
      <div class="acp-usage-row" title="Ventana de contexto del modelo: {agentStore.usage.used.toLocaleString()} de {agentStore.usage.size.toLocaleString()} tokens usados">
        <div class="acp-usage" class:warn={usagePct >= 80}>
          <div class="acp-usage-bar" style="width: {usagePct}%"></div>
        </div>
        <span class="acp-usage-text">
          Contexto {compact(agentStore.usage.used)} / {compact(agentStore.usage.size)} ({usagePct.toFixed(0)}%)
          {#if agentStore.usage.cost}· {formatCost(agentStore.usage.cost)}{/if}
        </span>
      </div>
    {/if}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="acp-composer"
      class:drag-over={dragOver}
      ondragover={(e) => {
        if (e.dataTransfer?.types.includes(DRAG_MIME)) {
          e.preventDefault();
          dragOver = true;
        }
      }}
      ondragleave={() => (dragOver = false)}
      ondrop={onDrop}
    >
      {#if suggestions.length}
        <div class="acp-suggest" role="listbox">
          {#each suggestions as sug, i (sug.detail + sug.label)}
            <button
              type="button"
              role="option"
              aria-selected={i === pickIndex}
              class:active={i === pickIndex}
              onmousedown={(e) => {
                e.preventDefault();
                sug.pick();
              }}
            >
              <span class="acp-suggest-label">{sug.label}</span>
              <span class="acp-suggest-detail">{sug.detail}</span>
            </button>
          {/each}
        </div>
      {/if}
      {#if agentStore.pendingAttachments.length}
        <div class="acp-chips">
          {#each agentStore.pendingAttachments as a (a.kind + a.path + (a.kind === 'selection' ? a.startLine : ''))}
            <span class="acp-chip" class:image={a.kind === 'image'} title={a.kind === 'image' ? imageTitle : a.path}>
              {#if a.kind === 'image'}<img src="data:{a.mimeType};base64,{a.data}" alt="" /><ImageIcon size={11} />{:else if a.kind === 'selection'}<TextSelect size={11} />{:else}<FileText size={11} />{/if}
              {chipLabel(a)}
              <button type="button" aria-label="Quitar {chipLabel(a)}" onclick={() => agentStore.detach(a)}><X size={10} /></button>
            </span>
          {/each}
        </div>
      {/if}
      <form
        class="acp-input"
        onsubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <textarea
          bind:this={textarea}
          bind:value={draft}
          onkeydown={onKeydown}
          onpaste={onPaste}
          onkeyup={(e) => (caret = e.currentTarget.selectionStart)}
          onclick={(e) => (caret = e.currentTarget.selectionStart)}
          oninput={(e) => (caret = e.currentTarget.selectionStart)}
          rows="3"
          placeholder={ready
            ? `Escribe a ${agentStore.activeAgent?.name ?? 'el agente'}… @ nota, / comando, Enter envía`
            : 'El agente no está conectado'}
          disabled={!ready}
        ></textarea>
        {#if busy}
          <button type="button" class="acp-send" title="Detener" onclick={() => agentStore.cancel()}>
            <Square size={14} />
          </button>
        {:else}
          <button type="submit" class="acp-send" title="Enviar" disabled={!ready || !draft.trim()}>
            <Send size={14} />
          </button>
        {/if}
      </form>
    </div>
  {/if}
</div>

<style>
  .acp-chat {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }
  .acp-empty {
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
  .acp-error-text {
    color: var(--danger);
    font-size: 12px;
  }
  .acp-messages {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 13px;
  }
  .acp-msg {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    line-height: 1.45;
    padding: 8px 10px;
    border-radius: 8px;
  }
  .acp-msg.user {
    align-self: flex-end;
    max-width: 85%;
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
  }
  .acp-msg.agent {
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
  }
  .acp-msg.error {
    display: flex;
    gap: 6px;
    align-items: flex-start;
    color: var(--danger);
    background: var(--danger-bg);
    border: 1px solid var(--danger-border);
  }
  .acp-thought {
    color: var(--text-muted);
    font-size: 12px;
    white-space: pre-wrap;
  }
  .acp-thought summary {
    cursor: pointer;
  }
  .acp-tool {
    font-size: 12px;
    color: var(--text-secondary);
    padding: 4px 8px;
    border-radius: 6px;
    border: 1px dashed var(--border-primary);
  }
  .acp-tool-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .acp-tool-status {
    font-family: monospace;
    font-size: 11px;
    color: var(--text-muted);
  }
  .acp-tool[data-status='failed'] {
    border-color: var(--danger-border);
  }
  .acp-permission {
    padding: 8px 10px;
    border-radius: 8px;
    border: 1px solid var(--warning, #9a6700);
    background: color-mix(in srgb, var(--warning, #9a6700) 10%, transparent);
  }
  .acp-permission.answered {
    opacity: 0.7;
  }
  .acp-permission-title {
    display: flex;
    gap: 6px;
    align-items: center;
    font-weight: 600;
    margin-bottom: 6px;
  }
  .acp-permission-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .acp-permission-done {
    font-size: 12px;
    color: var(--text-muted);
  }
  .acp-status {
    display: flex;
    gap: 6px;
    align-items: center;
    color: var(--text-muted);
    font-size: 12px;
  }
  :global(.acp-spin) {
    animation: acp-spin 0.9s linear infinite;
  }
  @keyframes acp-spin {
    to {
      transform: rotate(360deg);
    }
  }
  .acp-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    font: inherit;
    font-size: 12px;
    border-radius: 6px;
    border: 1px solid var(--border-primary);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    cursor: pointer;
  }
  .acp-btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-contrast, #fff);
  }
  .acp-btn.danger {
    color: var(--danger);
    border-color: var(--danger-border);
  }
  .acp-input {
    display: flex;
    gap: 6px;
    padding: 8px;
  }
  .acp-composer {
    position: relative;
    border-top: 1px solid var(--border-primary);
  }
  .acp-composer.drag-over {
    outline: 2px dashed var(--accent);
    outline-offset: -4px;
  }
  .acp-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 6px 8px 0;
  }
  .acp-chips.sent {
    padding: 6px 0 0;
  }
  .acp-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    padding: 1px 6px;
    border-radius: 999px;
    font-size: 11px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-primary);
    color: var(--text-secondary);
  }
  .acp-chip button {
    display: inline-flex;
    border: none;
    background: none;
    padding: 0;
    color: inherit;
    cursor: pointer;
  }
  .acp-suggest {
    position: absolute;
    left: 8px;
    right: 8px;
    bottom: 100%;
    margin-bottom: 4px;
    display: flex;
    flex-direction: column;
    max-height: 220px;
    overflow-y: auto;
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: 8px;
    box-shadow: var(--popover-shadow);
    z-index: 5;
  }
  .acp-suggest button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
    padding: 5px 8px;
    border: none;
    background: none;
    text-align: left;
    font: inherit;
    font-size: 12px;
    color: var(--text-primary);
    cursor: pointer;
  }
  .acp-suggest button.active {
    background: color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .acp-suggest-detail {
    font-size: 11px;
    color: var(--text-muted);
  }
  .acp-plan {
    position: sticky;
    top: -12px;
    z-index: 1;
    margin: -12px -12px 0;
    padding: 8px 12px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-primary);
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
  }
  .acp-plan-entry {
    display: flex;
    gap: 6px;
    align-items: center;
    color: var(--text-secondary);
  }
  .acp-plan-entry[data-status='completed'] span {
    text-decoration: line-through;
    color: var(--text-muted);
  }
  .acp-plan-entry[data-status='in_progress'] {
    color: var(--accent);
    font-weight: 600;
  }
  .acp-tool summary {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    list-style: none;
  }
  .acp-chip.image img {
    width: 18px;
    height: 18px;
    object-fit: cover;
    border-radius: 3px;
  }
  .acp-sent-image {
    max-width: 120px;
    max-height: 90px;
    border-radius: 6px;
    border: 1px solid var(--border-primary);
    object-fit: cover;
  }
  .acp-tool-label {
    margin-top: 6px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }
  .acp-tool[open] summary .acp-tool-title {
    white-space: normal;
  }
  .acp-msg.markdown {
    white-space: normal;
  }
  .acp-msg.markdown :global(p) {
    margin: 0 0 0.6em;
  }
  .acp-msg.markdown :global(:last-child) {
    margin-bottom: 0;
  }
  .acp-msg.markdown :global(h1),
  .acp-msg.markdown :global(h2),
  .acp-msg.markdown :global(h3),
  .acp-msg.markdown :global(h4) {
    margin: 0.8em 0 0.4em;
    font-size: 1.05em;
    line-height: 1.3;
  }
  .acp-msg.markdown :global(h1:first-child),
  .acp-msg.markdown :global(h2:first-child),
  .acp-msg.markdown :global(h3:first-child) {
    margin-top: 0;
  }
  .acp-msg.markdown :global(ul),
  .acp-msg.markdown :global(ol) {
    margin: 0 0 0.6em;
    padding-left: 1.4em;
  }
  .acp-msg.markdown :global(code) {
    font-family: var(--font-mono, monospace);
    font-size: 0.9em;
    padding: 0 3px;
    border-radius: 3px;
    background: var(--bg-tertiary);
  }
  .acp-msg.markdown :global(pre) {
    margin: 0 0 0.6em;
    padding: 8px;
    border-radius: 6px;
    overflow-x: auto;
    background: var(--bg-tertiary);
  }
  .acp-msg.markdown :global(pre code) {
    padding: 0;
    background: none;
  }
  .acp-msg.markdown :global(a) {
    color: var(--accent);
    cursor: pointer;
  }
  .acp-msg.markdown :global(table) {
    border-collapse: collapse;
    margin: 0 0 0.6em;
    font-size: 0.92em;
  }
  .acp-msg.markdown :global(th),
  .acp-msg.markdown :global(td) {
    border: 1px solid var(--border-primary);
    padding: 3px 6px;
  }
  .acp-msg.markdown :global(blockquote) {
    margin: 0 0 0.6em;
    padding-left: 8px;
    border-left: 3px solid var(--border-primary);
    color: var(--text-secondary);
  }
  .acp-tool-loc {
    font-family: monospace;
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 4px;
  }
  .acp-tool-text {
    margin: 4px 0 0;
    font-size: 11px;
    white-space: pre-wrap;
    max-height: 160px;
    overflow: auto;
  }
  .acp-diff {
    margin-top: 4px;
    font-family: monospace;
    font-size: 11px;
    max-height: 220px;
    overflow: auto;
  }
  .acp-diff-path {
    color: var(--text-muted);
  }
  .acp-diff-line {
    white-space: pre-wrap;
  }
  .acp-diff-line.add {
    color: var(--success);
    background: var(--success-bg);
  }
  .acp-diff-line.del {
    color: var(--danger);
    background: var(--danger-bg);
  }
  .acp-usage-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 10px;
  }
  .acp-usage-row .acp-usage {
    flex: 1;
  }
  .acp-usage.warn :global(.acp-usage-bar) {
    background: var(--warning, #9a6700);
  }
  .acp-usage-text {
    font-size: 10.5px;
    color: var(--text-muted);
    white-space: nowrap;
  }
  .acp-usage {
    height: 4px;
    border-radius: 2px;
    overflow: hidden;
    background: var(--bg-tertiary);
  }
  .acp-usage-bar {
    height: 100%;
    background: var(--accent);
  }
  .acp-input textarea {
    flex: 1;
    resize: none;
    font: inherit;
    font-size: 13px;
    padding: 6px 8px;
    border-radius: 6px;
    border: 1px solid var(--border-primary);
    background: var(--bg-primary);
    color: var(--text-primary);
  }
  .acp-send {
    align-self: flex-end;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 6px;
    border: 1px solid var(--accent);
    background: var(--accent);
    color: var(--accent-contrast, #fff);
    cursor: pointer;
  }
  .acp-send:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
