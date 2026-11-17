<script lang="ts">
  import {
    agentStore,
    agentApi,
    modeBadge,
    shortModelLabels,
    type Attachment,
    type Conversation,
    type ConversationSummary,
    type Mention,
    type ToolContent,
  } from '@entities/acp-agent';
  import { settingsApi } from '@entities/settings';
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
    Shrink,
    ChevronDown,
    Copy,
  } from 'lucide-svelte';
  import { tick } from 'svelte';
  import { OctopusIcon } from '@shared/ui/icons';
  import { renderAgentMarkdown, vaultPathOfLink, formatRaw } from '../lib/markdown';
  import {
    ago,
    conversationTooltip,
    mentionsPrompt,
    mergeVaults,
    splitMentions,
    STATUS_ICON,
    VAULT_ICON,
    vaultToken,
    type VaultCandidate,
  } from '../lib/mentions';

  interface Props {
    /** Carpeta de trabajo del agente (la bóveda abierta). */
    cwd: string | null;
    onOpenSettings?: () => void;
    /** Abre en Synapse una nota de la bóveda (ruta relativa). */
    onOpenNote?: (relativePath: string) => void;
    /** Conversación que muestra este chat. */
    conv: Conversation;
  }

  let { cwd, onOpenSettings, onOpenNote, conv }: Props = $props();

  // Enlaces en las respuestas: notas de la bóveda se abren en Synapse; el resto, fuera
  /** Tamaño del mensaje de otro agente, para el resumen cerrado. */
  const incomingSize = (text: string) => {
    const bytes = new TextEncoder().encode(text).length;
    return bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1).replace('.', ',')} KB`;
  };

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
  // Pregunta preparada desde otra parte de la app (p. ej. el Historial de Git): se escribe aquí si
  // esta es la conversación enfocada, sin enviarla
  $effect(() => {
    const pending = agentStore.pendingDraft;
    if (!pending || agentStore.focused !== conv) return;
    draft = pending;
    agentStore.pendingDraft = null;
    tick().then(() => textarea?.focus());
  });
  let scroller = $state<HTMLDivElement | null>(null);
  let textarea = $state<HTMLTextAreaElement | null>(null);
  let dragOver = $state(false);
  let pickIndex = $state(0);

  const busy = $derived(conv.connection === 'busy');
  const ready = $derived(conv.connection === 'ready' || busy);

  // Bajar al final cuando llega contenido nuevo
  $effect(() => {
    const _n = conv.items.length;
    const last = conv.items.at(-1);
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
  // `#2`: otra conversación de esta ventana
  const hash = $derived.by(() => {
    const m = /(^|\s)#([^\s#@]*)$/.exec(draft.slice(0, caret));
    return m ? { query: m[2].toLowerCase(), start: caret - m[2].length - 1 } : null;
  });

  // Menciones elegidas en el borrador (se explican al agente al enviar)
  let pickedMentions = $state<Mention[]>([]);
  // Otras bóvedas de Synapse: abiertas (🟢/🟡) y conocidas apagadas (⚪); se consultan al escribir `@`
  let vaults = $state<VaultCandidate[]>([]);
  // Bóvedas que se están encendiendo: esperan a responder antes de enviar
  const starting = new Map<string, Promise<boolean>>();
  let startingPaths = $state<string[]>([]);
  // Una consulta cada vez que se empieza a escribir una mención, no en cada tecla
  const mentionOpen = $derived(mention !== null);
  $effect(() => {
    if (!mentionOpen) return;
    Promise.all([agentStore.peers.openVaults(), settingsApi.getRecentWorkspaces().catch(() => [])])
      .then(([open, known]) => (vaults = mergeVaults(open, known, cwd)))
      .catch(() => {});
  });

  /** Reemplaza lo escrito desde `start` hasta el cursor por `token` y recuerda la mención. */
  function insertMention(start: number, mentionItem: Mention) {
    const insert = `${mentionItem.token} `;
    draft = draft.slice(0, start) + insert + draft.slice(caret);
    if (!pickedMentions.some((m) => m.token === mentionItem.token)) pickedMentions = [...pickedMentions, mentionItem];
    const at = start + insert.length;
    tick().then(() => {
      textarea?.focus();
      textarea?.setSelectionRange(at, at);
      caret = at;
    });
  }

  const conversationRow = (c: ConversationSummary, token: string, to: string, where: string, start: number): Suggestion => ({
    label: `${STATUS_ICON[c.status] ?? ''} ${token}  ${c.title}`,
    detail: `${c.agent}${c.last_activity ? ` · ${ago(c.last_activity)}` : ''}`,
    tooltip: conversationTooltip(c),
    pick: () => insertMention(start, { token, to, title: `la conversación «${c.title}» (${c.agent}) ${where}` }),
  });

  /** Enciende una bóveda apagada (otra ventana) y espera a que responda. */
  function startVault(v: VaultCandidate) {
    if (starting.has(v.path)) return;
    startingPaths = [...startingPaths, v.path];
    const ready = (async () => {
      await settingsApi.openWorkspaceInNewWindow(v.path);
      for (let i = 0; i < 60; i++) {
        await new Promise((r) => setTimeout(r, 500));
        const peers = await agentApi.peerList({ action: 'ping' }).catch(() => []);
        if (peers.some((p) => p.path.replace(/\/+$/, '') === v.path.replace(/\/+$/, ''))) return true;
      }
      return false;
    })()
      .catch(() => false)
      .finally(() => {
        startingPaths = startingPaths.filter((p) => p !== v.path);
        vaults = vaults.map((x) => (x.path === v.path ? { ...x, state: 'active' } : x));
      });
    starting.set(v.path, ready);
  }

  function fuzzy(text: string, q: string): boolean {
    let i = 0;
    for (const c of text) if (c === q[i]) i++;
    return i === q.length;
  }

  type Suggestion = { label: string; detail: string; tooltip?: string; pick: () => void };
  const suggestions = $derived.by((): Suggestion[] => {
    if (hash) {
      const h = hash;
      return agentStore.conversations
        .filter((c) => c.id !== conv.id)
        .filter((c) => `${c.number} ${c.title} ${c.agent?.name ?? c.agentId}`.toLowerCase().includes(h.query))
        .slice(0, 8)
        .map((c) =>
          conversationRow(
            {
              handle: c.handle,
              agent: c.agent?.name ?? c.agentId,
              title: c.title || 'Conversación nueva',
              status: c.status,
              last_reply: c.lastText('agent'),
              last_question: c.lastText('user'),
              last_activity: c.lastActivity,
            },
            c.handle,
            c.handle,
            'de esta ventana',
            h.start,
          ),
        );
    }
    if (mention && mention.query.includes('#')) {
      // `@Bóveda#…`: conversaciones de otra bóveda
      const m = mention;
      const [vq, cq] = m.query.split('#');
      const v = vaults.find((x) => vaultToken(x.name).slice(1).toLowerCase() === vq);
      if (!v) return [];
      const rows = v.conversations
        .filter((c) => `${c.number ?? ''} ${c.title} ${c.agent}`.toLowerCase().includes(cq))
        .slice(0, 8)
        .map((c) => {
          const n = c.number ?? c.handle.replace(/^#/, '');
          return conversationRow(c, `${vaultToken(v.name)}#${n}`, `${v.name}#${n}`, `de la bóveda «${v.name}»`, m.start);
        });
      if ('nueva'.startsWith(cq)) {
        rows.push({
          label: `${vaultToken(v.name)}#nueva`,
          detail: 'Abre una conversación nueva en esa bóveda',
          pick: () => {
            if (v.state === 'off') startVault(v);
            insertMention(m.start, { token: `${vaultToken(v.name)}#nueva`, to: `${v.name}#nueva`, title: `una conversación nueva en la bóveda «${v.name}»` });
          },
        });
      }
      return rows;
    }
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
                  conv.attachActiveNote();
                  tick().then(() => {
                    textarea?.focus();
                    textarea?.setSelectionRange(m.start, m.start);
                    caret = m.start;
                  });
                },
              },
            ]
          : [];
      const m = mention;
      const vaultRows: Suggestion[] = vaults
        .filter((v) => fuzzy(v.name.toLowerCase(), m.query))
        .slice(0, 5)
        .map((v) => ({
          label: `${VAULT_ICON[v.state]} ${vaultToken(v.name)}`,
          detail: startingPaths.includes(v.path)
            ? 'encendiendo…'
            : v.state === 'off'
              ? 'Bóveda de Synapse apagada · elegirla la abre'
              : `Bóveda de Synapse · ${v.conversations.length} conversaciones · escribe # para elegir una`,
          tooltip: v.path,
          pick: () => {
            if (v.state === 'off') startVault(v);
            insertMention(m.start, {
              token: vaultToken(v.name),
              to: v.name,
              title: `la bóveda de Synapse «${v.name}» (otra ventana; su conversación de este hilo o una nueva)`,
            });
          },
        }));
      return [...self, ...vaultItemStore.items
        .filter((it) => fuzzy(it.relative_path.toLowerCase(), mention.query))
        .slice(0, vaultRows.length ? 6 : 8)
        .map((it) => ({
          label: it.title || nameOf(it.relative_path),
          detail: it.relative_path,
          pick: () => {
            const m = mention;
            draft = draft.slice(0, m.start) + draft.slice(caret);
            conv.attach({ kind: 'file', path: absPath(it.abs_path ?? it.relative_path), name: nameOf(it.relative_path) });
            tick().then(() => {
              textarea?.focus();
              textarea?.setSelectionRange(m.start, m.start);
              caret = m.start;
            });
          },
        })), ...vaultRows];
    }
    if (slash !== null) {
      // `/feedback` es de Synapse (compilación `develop`): pide al agente feedback sobre el MCP
      const own = agentStore.feedbackAvailable && 'feedback'.startsWith(slash)
        ? [{
            label: '/feedback',
            detail: 'Synapse · el agente registra qué mejorar del MCP y del panel (puedes añadir indicaciones)',
            pick: () => {
              draft = '/feedback ';
              tick().then(() => textarea?.focus());
            },
          }]
        : [];
      // `/usage` también es de Synapse, salvo que el agente traiga el suyo
      if ('usage'.startsWith(slash) && !conv.commands.some((c) => c.name === 'usage')) {
        own.push({
          label: '/usage',
          detail: 'Synapse · resumen de uso de esta conversación (contexto, costo, herramientas)',
          pick: () => {
            draft = '/usage';
            tick().then(() => textarea?.focus());
          },
        });
      }
      return [...own, ...conv.commands
        .filter((c) => c.name.toLowerCase().startsWith(slash))
        .slice(0, 8)
        .map((c) => ({
          label: `/${c.name}`,
          detail: c.description ?? '',
          pick: () => {
            draft = `/${c.name} `;
            tick().then(() => textarea?.focus());
          },
        }))];
    }
    return [];
  });

  $effect(() => {
    const _n = suggestions.length;
    pickIndex = 0;
  });

  async function submit() {
    const text = draft.trim();
    if (!text || !ready || busy) return;
    draft = '';
    const { prompt, used } = mentionsPrompt(text, pickedMentions);
    pickedMentions = [];
    // Si se mencionó una bóveda que se está encendiendo, se espera a que responda
    const waiting = used.flatMap((m) => {
      const v = vaults.find((x) => m.to === x.name || m.to.startsWith(`${x.name}#`));
      return v && starting.has(v.path) ? [starting.get(v.path)!] : [];
    });
    if (waiting.length) await Promise.all(waiting);
    conv.send(text, used.length ? { prompt, mentions: used } : {});
  }

  function onKeydown(e: KeyboardEvent) {
    if (suggestions.length) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const d = e.key === 'ArrowDown' ? 1 : -1;
        pickIndex = (pickIndex + d + suggestions.length) % suggestions.length;
        return;
      }
      // Enter con el comando ya escrito completo lo envía (no vuelve a elegir la misma sugerencia)
      const complete = e.key === 'Enter' && suggestions[pickIndex].label === draft.trim();
      if ((e.key === 'Enter' || e.key === 'Tab') && !complete) {
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
      for (const p of JSON.parse(raw) as string[]) conv.attach({ kind: 'file', path: absPath(p), name: nameOf(p) });
    } catch {
      /* datos de arrastre inválidos */
    }
  }

  // Pegar imágenes del portapapeles (capturas, imágenes copiadas) como adjunto del mensaje
  let pasted = 0;
  const imageTitle = $derived(
    conv.session && !conv.session.capabilities.image
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
        conv.attach({ kind: 'image', path: `pegada-${Date.now()}-${pasted}`, name: `Imagen ${pasted}.${ext}`, mimeType: file.type, data });
      };
      reader.readAsDataURL(file);
    }
  }

  // Imagen ampliada (lightbox): fondo oscuro, la imagen centrada; Esc, clic fuera o ✕ la cierran
  let zoomed = $state<Extract<Attachment, { kind: 'image' }> | null>(null);
  $effect(() => {
    if (!zoomed) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        zoomed = null;
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  });

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
  // Modelo y modo, en corto junto a la línea de contexto (nombres completos en el tooltip y el menú)
  let optsOpen = $state(false);
  $effect(() => {
    if (!optsOpen) return;
    const close = () => (optsOpen = false);
    window.addEventListener('click', close);
    return () => window.removeEventListener('click', close);
  });
  const currentModelName = $derived(
    conv.session?.models.find((m) => m.id === conv.currentModel)?.name || conv.currentModel || '',
  );
  const modelShort = $derived.by(() => {
    const models = conv.session?.models ?? [];
    const i = models.findIndex((m) => m.id === conv.currentModel);
    return i >= 0 ? shortModelLabels(models.map((m) => m.name || m.id))[i] : '';
  });
  const currentModeName = $derived(
    conv.session?.modes.find((m) => m.id === conv.currentMode)?.name || conv.currentMode || '',
  );
  /** El modo solo se muestra si no es el de inicio del agente. */
  const modeShown = $derived(
    conv.currentMode && conv.currentMode !== conv.session?.current_mode ? modeBadge(conv.currentMode, currentModeName) : '',
  );
  const optsTitle = $derived(
    [currentModelName && `Modelo: ${currentModelName}`, currentModeName && `Modo: ${currentModeName}`].filter(Boolean).join('\n'),
  );

  const usagePct = $derived(conv.usage ? Math.min(100, (conv.usage.used / conv.usage.size) * 100) : 0);
</script>

<div class="acp-chat">
  {#if zoomed}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="acp-lightbox" role="dialog" tabindex="-1" aria-modal="true" aria-label={zoomed.name} onclick={() => (zoomed = null)}>
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
      <figure onclick={(e) => e.stopPropagation()}>
        <img src="data:{zoomed.mimeType};base64,{zoomed.data}" alt={zoomed.name} />
        <figcaption>{zoomed.name}</figcaption>
      </figure>
      <button type="button" class="acp-lightbox-close" aria-label="Cerrar" onclick={() => (zoomed = null)}><X size={18} /></button>
    </div>
  {/if}
  {#if conv}
    <div class="acp-messages" bind:this={scroller}>
      {#if conv.connection === 'installing' || conv.connection === 'starting'}
        <div class="acp-status"><Loader2 size={14} class="acp-spin" /> {statusText[conv.connection]}</div>
      {:else if conv.connection === 'idle' && conv.items.length === 0}
        <div class="acp-empty">
          <button type="button" class="acp-btn primary" onclick={() => conv.connect()}>
            <Bot size={14} /> Conectar con {conv.agent?.name ?? "el agente"}
          </button>
        </div>
      {/if}

      {#if conv.plan.length}
        <div class="acp-plan" aria-label="Plan del agente">
          {#each conv.plan as entry, i (i)}
            <div class="acp-plan-entry" data-status={entry.status}>
              {#if entry.status === 'completed'}<CircleCheck size={13} />{:else if entry.status === 'in_progress'}<CircleDot
                  size={13}
                />{:else}<Circle size={13} />{/if}
              <span>{entry.content}</span>
            </div>
          {/each}
        </div>
      {/if}

      {#each conv.items as item (item.id)}
        {#if item.type === 'user' && item.from}
          <!-- Mensaje de otro agente: cerrado; al abrirlo se ve renderizado (tu agente ya lo resume) -->
          <details class="acp-msg user from-agent acp-incoming">
            <summary class="acp-from">
              {#if item.from.vault}<OctopusIcon size={12} title="De otra bóveda de Synapse" />{/if}
              De {item.from.agent} · {item.from.handle}
              <span class="acp-incoming-size">{incomingSize(item.text)}</span>
            </summary>
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="markdown" onclick={onMessageClick}>{@html renderAgentMarkdown(item.text)}</div>
            <button type="button" class="acp-incoming-copy" title="Copiar el texto tal como llegó" onclick={() => navigator.clipboard?.writeText(item.text)}>
              <Copy size={11} /> Copiar texto
            </button>
          </details>
        {:else if item.type === 'user'}
          <div class="acp-msg user" class:from-agent={!!item.from}>
            {#if item.from}<div class="acp-from">
                {#if item.from.vault}<OctopusIcon size={12} title="De otra bóveda de Synapse" />{/if}
                De {item.from.agent} · {item.from.handle}
              </div>{/if}
            {#each splitMentions(item.text, item.mentions) as part, i (i)}{#if 'mention' in part}<span class="acp-mention" title={part.mention.title}>{part.mention.token}</span>{:else}{part.text}{/if}{/each}
            {#if item.attachments.length}
              <div class="acp-chips sent">
                {#each item.attachments as a, i (i)}
                  {#if a.kind === 'image'}
                    <button type="button" class="acp-image-btn" title="Ver {a.name}" onclick={() => (zoomed = a)}>
                      <img class="acp-sent-image" src="data:{a.mimeType};base64,{a.data}" alt={a.name} />
                    </button>
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
          <details class="acp-thought"><summary title="Razonamiento del agente" aria-label="Razonamiento">🧠<span class="acp-thought-label"> Razonamiento</span></summary>{item.text}</details>
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
                    onclick={() => conv.answer(item.id, opt.id)}>{opt.name}</button
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
        {:else if item.type === 'info'}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div class="acp-msg info markdown" onclick={onMessageClick}>{@html renderAgentMarkdown(item.text)}</div>
        {:else if item.type === 'compact'}
          <div class="acp-compact-marker" role="separator">— contexto compactado —</div>
        {/if}
      {/each}
      {#if busy}
        <div class="acp-status acp-thinking"><span class="acp-brain" aria-hidden="true">🧠</span> Pensando…</div>
      {/if}
    </div>

    {#if conv.session || conv.usage}
      <div class="acp-usage-row">
        {#if conv.session && (conv.session.models.length || conv.session.modes.length)}
          <div class="acp-agent-opts">
            <button
              type="button"
              class="acp-opts-btn"
              title={optsTitle}
              aria-label="Modelo y modo del agente"
              aria-haspopup="menu"
              aria-expanded={optsOpen}
              onclick={(e) => {
                e.stopPropagation();
                optsOpen = !optsOpen;
              }}
            >
              {#if modeShown}<span class="acp-mode-chip">{modeShown}</span>{/if}
              {#if modelShort}<span class="acp-model-chip">{modelShort}</span>{/if}
              <ChevronDown size={11} />
            </button>
            {#if optsOpen}
              <div class="acp-opts-menu" role="menu">
                {#if conv.session.models.length}
                  <div class="acp-opts-head">Modelo</div>
                  {#each conv.session.models as m (m.id)}
                    <button
                      type="button"
                      role="menuitemradio"
                      aria-checked={m.id === conv.currentModel}
                      class:selected={m.id === conv.currentModel}
                      onclick={() => {
                        optsOpen = false;
                        conv.setModel(m.id);
                      }}>{m.name || m.id}</button
                    >
                  {/each}
                {/if}
                {#if conv.session.modes.length}
                  <div class="acp-opts-head">Modo</div>
                  {#each conv.session.modes as m (m.id)}
                    <button
                      type="button"
                      role="menuitemradio"
                      aria-checked={m.id === conv.currentMode}
                      class:selected={m.id === conv.currentMode}
                      title={m.description ?? ''}
                      onclick={() => {
                        optsOpen = false;
                        conv.setMode(m.id);
                      }}>{m.name || m.id}</button
                    >
                  {/each}
                {/if}
              </div>
            {/if}
          </div>
        {/if}
        {#if conv.usage}
          <div
            class="acp-usage"
            class:warn={usagePct >= 80}
            title="Ventana de contexto del modelo: {conv.usage.used.toLocaleString()} de {conv.usage.size.toLocaleString()} tokens ({compact(conv.usage.used)} / {compact(conv.usage.size)}){conv.usage.cost ? ` · ${formatCost(conv.usage.cost)}` : ''}"
          >
            <div class="acp-usage-bar" style="width: {usagePct}%"></div>
          </div>
          <span class="acp-usage-text" title="{compact(conv.usage.used)} / {compact(conv.usage.size)} tokens">{usagePct.toFixed(0)}%</span>
        {:else}
          <span class="acp-usage-spacer"></span>
        {/if}
        {#if conv.canCompact && conv.usage}
          <button
            type="button"
            class="acp-compact"
            class:shown={usagePct >= 60 || conv.compacting}
            class:warn={usagePct >= 80}
            title="Pide al agente que resuma su contexto (/compact) para liberar espacio"
            disabled={conv.compacting || busy}
            onclick={() => conv.compact()}
          >
            <Shrink size={11} />
            {conv.compacting ? 'Compactando…' : 'Compactar'}
          </button>
        {/if}
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
              title={sug.tooltip}
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
      {#if conv.pendingAttachments.length}
        <div class="acp-chips">
          {#each conv.pendingAttachments as a (a.kind + a.path + (a.kind === 'selection' ? a.startLine : ''))}
            <span class="acp-chip" class:image={a.kind === 'image'} title={a.kind === 'image' ? imageTitle : a.path}>
              {#if a.kind === 'image'}<button type="button" class="acp-image-btn" title="Ver la imagen" onclick={() => (zoomed = a)}><img src="data:{a.mimeType};base64,{a.data}" alt="" /></button><ImageIcon size={11} />{:else if a.kind === 'selection'}<TextSelect size={11} />{:else}<FileText size={11} />{/if}
              {chipLabel(a)}
              <button type="button" aria-label="Quitar {chipLabel(a)}" onclick={() => conv.detach(a)}><X size={10} /></button>
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
            ? `Escribe a ${conv.agent?.name ?? 'el agente'}… @ nota, / comando, Enter envía`
            : 'El agente no está conectado'}
          disabled={!ready}
        ></textarea>
        {#if busy}
          <button type="button" class="acp-send" title="Detener" onclick={() => conv.cancel()}>
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
  .acp-thought:not([open]) {
    align-self: flex-start;
  }
  .acp-thought:not([open]) .acp-thought-label {
    display: none;
  }
  .acp-thought summary {
    list-style: none;
    cursor: pointer;
  }
  .acp-thought summary::-webkit-details-marker {
    display: none;
  }
  .acp-image-btn {
    display: inline-flex;
    padding: 0;
    border: none;
    background: none;
    cursor: zoom-in;
  }
  .acp-lightbox {
    position: fixed;
    inset: 0;
    z-index: 2000;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.72);
    cursor: zoom-out;
  }
  .acp-lightbox figure {
    margin: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    cursor: default;
  }
  .acp-lightbox img {
    max-width: 90vw;
    max-height: 85vh;
    object-fit: contain;
    border-radius: 6px;
    background: #fff;
  }
  .acp-lightbox figcaption {
    color: #eee;
    font-size: 12px;
  }
  .acp-lightbox-close {
    position: absolute;
    top: 16px;
    right: 16px;
    display: inline-flex;
    padding: 6px;
    border: none;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.15);
    color: #fff;
    cursor: pointer;
  }
  .acp-msg.user.from-agent {
    align-self: flex-start;
    background: color-mix(in srgb, var(--text-muted) 12%, transparent);
    border-color: var(--border-primary);
  }
  .acp-mention {
    padding: 0 4px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--accent-primary, #6366f1) 18%, transparent);
    font-weight: 500;
    cursor: help;
  }
  .acp-incoming > summary {
    cursor: pointer;
    list-style: revert;
    margin-bottom: 0;
  }
  .acp-incoming[open] > summary {
    margin-bottom: 6px;
  }
  .acp-incoming-size {
    margin-left: 4px;
    opacity: 0.8;
  }
  .acp-incoming-copy {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-top: 6px;
    padding: 2px 6px;
    border: 1px solid var(--border-primary);
    border-radius: 4px;
    background: transparent;
    color: var(--text-muted);
    font-size: 10.5px;
    cursor: pointer;
  }
  .acp-from {
    font-size: 10.5px;
    color: var(--text-muted);
    margin-bottom: 2px;
  }
  .acp-brain {
    display: inline-block;
    animation: acpBrain 1.4s ease-in-out infinite;
  }
  @keyframes acpBrain {
    50% {
      transform: scale(1.18);
    }
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
  .acp-msg.info {
    background: color-mix(in srgb, var(--accent) 6%, var(--bg-primary));
    border: 1px dashed color-mix(in srgb, var(--accent) 35%, transparent);
    font-size: 12px;
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
  .acp-compact {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 0 2px;
    border: none;
    background: none;
    font: inherit;
    font-size: 10.5px;
    color: var(--text-muted);
    cursor: pointer;
    white-space: nowrap;
    visibility: hidden;
  }
  .acp-compact.shown,
  .acp-usage-row:hover .acp-compact {
    visibility: visible;
  }
  .acp-compact:hover:not(:disabled) {
    color: var(--accent);
    text-decoration: underline;
  }
  .acp-compact.warn {
    color: var(--warning, #9a6700);
  }
  .acp-compact:disabled {
    cursor: default;
    opacity: 0.7;
  }
  .acp-compact-marker {
    align-self: stretch;
    text-align: center;
    font-size: 11px;
    color: var(--text-muted);
    margin: 4px 0;
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
  .acp-agent-opts {
    position: relative;
    flex: 0 1 auto;
    min-width: 0;
    max-width: 65%;
  }
  .acp-model-chip {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .acp-opts-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    padding: 0 3px;
    border: none;
    border-radius: 4px;
    background: none;
    font: inherit;
    font-size: 10.5px;
    color: var(--text-secondary);
    cursor: pointer;
    white-space: nowrap;
  }
  .acp-opts-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }
  .acp-mode-chip {
    color: var(--accent);
  }
  .acp-opts-menu {
    position: absolute;
    left: 0;
    bottom: calc(100% + 4px);
    z-index: 20;
    display: flex;
    flex-direction: column;
    min-width: 200px;
    max-height: 320px;
    overflow-y: auto;
    padding: 4px;
    border-radius: 8px;
    border: 1px solid var(--border-primary);
    background: var(--bg-primary);
    box-shadow: var(--modal-shadow);
  }
  .acp-opts-head {
    padding: 4px 8px 2px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }
  .acp-opts-menu button {
    padding: 5px 8px;
    border: none;
    border-radius: 5px;
    background: none;
    font: inherit;
    font-size: 12px;
    text-align: left;
    color: var(--text-primary);
    cursor: pointer;
  }
  .acp-opts-menu button:hover {
    background: var(--bg-tertiary);
  }
  .acp-opts-menu button.selected {
    color: var(--accent);
    font-weight: 600;
  }
  .acp-usage-spacer {
    flex: 1;
  }
</style>
