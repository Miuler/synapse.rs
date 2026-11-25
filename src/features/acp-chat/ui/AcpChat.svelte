<script lang="ts">
  import { t } from '@shared/lib/i18n';
  import { internalDragPaths, NATIVE_DROP, type NativeDrop } from '@shared/lib/native-drop';
  import {
    agentStore,
    agentApi,
    agentImages,
    modeBadge,
    shortModelLabels,
    type Attachment,
    type Conversation,
    type ConversationSummary,
    type Mention,
    type ToolContent,
    groupsForModel,
    percent,
    quotaLevel,
    quotaTooltip,
    tightestWindow,
    windowLabel,
  } from '@entities/acp-agent';
  import { settingsApi } from '@entities/settings';
  import { vaultItemStore } from '@entities/vault-item';
  import ElicitationCard from './ElicitationCard.svelte';
  import ChatSearch from './ChatSearch.svelte';
  import SideQuestion from './SideQuestion.svelte';
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
    ClipboardType,
    Clock,
    Check,
  } from 'lucide-svelte';
  import { tick, untrack } from 'svelte';
  import { OctopusIcon } from '@shared/ui/icons';
  import { renderAgentMarkdown, renderUserMarkdown, vaultPathOfLink, formatRaw } from '../lib/markdown';
  import { taskAwareCopy, richCopyOfElement, writeRichClipboard } from '@shared/lib/rich-copy';
  import { appSettings } from '@entities/settings';
  import { stepViewerZoom } from '@shared/lib/viewer-zoom';
  import {
    ago,
    conversationTooltip,
    fileMention,
    mentionsPrompt,
    mergeVaults,
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
    /** Enlace `file://` a un archivo fuera de la bóveda */
    onOutsideFile?: (absolutePath: string) => void;
    /** Conversación que muestra este chat. */
    conv: Conversation;
  }

  let { cwd, onOpenSettings, onOpenNote, onOutsideFile, conv }: Props = $props();

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
    // Se conserva el ancla (p. ej. `#L56-L65`, líneas a las que saltar)
    const hash = href.includes('#') ? href.slice(href.indexOf('#') + 1) : '';
    if (rel) onOpenNote?.(hash ? `${rel}#${decodeURIComponent(hash)}` : rel);
    else if (href.startsWith('file://')) onOutsideFile?.(decodeURIComponent(href.replace(/^file:\/\//, '')));
    else if (/^https?:/i.test(href)) window.open(href, '_blank', 'noopener');
    // Enlace relativo (p. ej. solo el nombre del archivo): el espacio de trabajo lo resuelve
    else if (href && !/^[a-z][a-z0-9+.-]*:/i.test(href) && !href.startsWith('#')) {
      let target = href;
      try {
        target = decodeURIComponent(href);
      } catch {
        /* tal cual */
      }
      onOpenNote?.(target.replace(/^\.\//, ''));
    }
  }

  // Pregunta preparada desde otra parte de la app (p. ej. el Historial de Git): se escribe aquí si
  // esta es la conversación enfocada, sin enviarla
  $effect(() => {
    const pending = agentStore.pendingDraft;
    if (!pending || agentStore.focused !== conv) return;
    conv.draft = pending;
    agentStore.pendingDraft = null;
    tick().then(() => textarea?.focus());
  });
  // Búsqueda en la conversación (Ctrl+F)
  let searchOpen = $state(false);
  let searchRequest = $state(0);
  $effect(() => {
    if (agentStore.searchRequest !== conv.id) return;
    agentStore.searchRequest = null;
    searchOpen = true;
    untrack(() => searchRequest++);
  });
  function closeSearch() {
    searchOpen = false;
    textarea?.focus();
  }
  let scroller = $state<HTMLDivElement | null>(null);
  let textarea = $state<HTMLTextAreaElement | null>(null);
  let chatRoot = $state<HTMLDivElement | null>(null);

  // Zoom del chat (mensajes y caja de texto): Ctrl+rueda aquí; Ctrl++/−/0 los gestiona el espacio de trabajo
  $effect(() => {
    const el = chatRoot;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if (!e.ctrlKey || !e.deltaY) return;
      e.preventDefault();
      appSettings.setViewerZoom('chat', stepViewerZoom(appSettings.chatZoom, e.deltaY < 0 ? 1 : -1));
    };
    el.addEventListener('wheel', onWheel, { passive: false });
    return () => el.removeEventListener('wheel', onWheel);
  });

  // Copiar una respuesta: Markdown tal cual o con formato (HTML + texto, casillas como ☐ / ✅)
  let copied = $state<{ id: string; how: 'md' | 'rich' } | null>(null);
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;
  function flagCopied(id: string, how: 'md' | 'rich') {
    copied = { id, how };
    clearTimeout(copiedTimer);
    copiedTimer = setTimeout(() => (copied = null), 1500);
  }
  function copyMarkdown(id: string, text: string) {
    navigator.clipboard?.writeText(text).then(() => flagCopied(id, 'md')).catch(() => {});
  }
  function copyRich(id: string, button: HTMLElement) {
    const msg = button.closest('.acp-agent-wrap')?.querySelector<HTMLElement>('.acp-msg.agent');
    if (msg) writeRichClipboard(richCopyOfElement(msg)).then((ok) => ok && flagCopied(id, 'rich'));
  }

  // La caja crece con el texto (desde 3 líneas) hasta el 40 % de la ventana; después, scroll propio
  $effect(() => {
    void conv.draft;
    const el = textarea;
    // Con el panel oculto no hay medidas; se ajusta al mostrarlo
    if (!el || !agentStore.panelOpen) return;
    el.style.height = 'auto';
    el.style.height = `${el.scrollHeight + 2}px`;
  });
  let dragOver = $state(false);
  let pickIndex = $state(0);

  const busy = $derived(conv.connection === 'busy');
  const ready = $derived(conv.connection === 'ready' || busy);

  // Bajar al final cuando llega contenido nuevo; si llegó con el panel oculto, al mostrarlo. Sin nada
  // nuevo, al mostrarlo se conserva el scroll.
  let scrolledFor = '';
  $effect(() => {
    const last = conv.items.at(-1);
    const content = `${conv.items.length}:${last && 'text' in last ? last.text.length : 0}`;
    if (!agentStore.panelOpen || content === scrolledFor) return;
    scrolledFor = content;
    // Buscando, lo nuevo no se lleva la coincidencia que se está viendo
    if (untrack(() => searchOpen)) return;
    tick().then(() => scroller?.scrollTo({ top: scroller.scrollHeight }));
  });

  const absPath = (p: string) => (p.startsWith('/') || !cwd ? p : `${cwd.replace(/\/$/, '')}/${p}`);
  const nameOf = (p: string) => p.split('/').pop() || p;
  const relOf = (p: string) => (cwd && p.startsWith(cwd) ? p.slice(cwd.length).replace(/^\//, '') : p);

  // Autocompletado: `@nota` menciona una nota, `/comando` al inicio usa un comando del agente
  let caret = $state(0);
  const mention = $derived.by(() => {
    const m = /(^|\s)@([^\s@]*)$/.exec(conv.draft.slice(0, caret));
    return m ? { query: m[2].toLowerCase(), start: caret - m[2].length - 1 } : null;
  });
  const slash = $derived(/^\/(\S*)$/.exec(conv.draft)?.[1]?.toLowerCase() ?? null);
  // `#2`: otra conversación de esta ventana
  const hash = $derived.by(() => {
    const m = /(^|\s)#([^\s#@]*)$/.exec(conv.draft.slice(0, caret));
    return m ? { query: m[2].toLowerCase(), start: caret - m[2].length - 1 } : null;
  });

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
    conv.draft = conv.draft.slice(0, start) + insert + conv.draft.slice(caret);
    if (!conv.pickedMentions.some((m) => m.token === mentionItem.token)) conv.pickedMentions = [...conv.pickedMentions, mentionItem];
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

  type Suggestion = { label: string; detail: string; tooltip?: string; header?: string; pick: () => void };

  /** Conversaciones de esta ventana (`#2`) que coinciden con `query`. */
  function localConversationRows(query: string, start: number): Suggestion[] {
    return agentStore.conversations
      .filter((c) => c.id !== conv.id)
      .filter((c) => `${c.number} ${c.title} ${c.agent?.name ?? c.agentId}`.toLowerCase().includes(query))
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
          start,
        ),
      );
  }

  /** `Bóveda#…`: conversaciones de otra bóveda, o una nueva. */
  function remoteConversationRows(query: string, start: number): Suggestion[] {
    const [vq, cq] = query.split('#');
    const v = vaults.find((x) => vaultToken(x.name).slice(1).toLowerCase() === vq);
    if (!v) return [];
    const rows = v.conversations
      .filter((c) => `${c.number ?? ''} ${c.title} ${c.agent}`.toLowerCase().includes(cq))
      .slice(0, 8)
      .map((c) => {
        const n = c.number ?? c.handle.replace(/^#/, '');
        return conversationRow(c, `${vaultToken(v.name)}#${n}`, `${v.name}#${n}`, `de la bóveda «${v.name}»`, start);
      });
    if ('nueva'.startsWith(cq)) {
      rows.push({
        label: `${vaultToken(v.name)}#nueva`,
        detail: 'Abre una conversación nueva en esa bóveda',
        pick: () => {
          if (v.state === 'off') startVault(v);
          insertMention(start, { token: `${vaultToken(v.name)}#nueva`, to: `${v.name}#nueva`, title: `una conversación nueva en la bóveda «${v.name}»` });
        },
      });
    }
    return rows;
  }
  const suggestions = $derived.by((): Suggestion[] => {
    if (hash) return localConversationRows(hash.query, hash.start);
    if (mention) {
      // Filtro opcional: `@agent:` / `@a:` solo agentes (bóvedas y conversaciones), `@file:` / `@f:` solo
      // archivos. El prefijo solo filtra: al elegir se inserta la mención sin él.
      const m = mention;
      const scoped = /^(agents?|a|files?|f):(.*)$/.exec(m.query);
      const scope = scoped ? (scoped[1].startsWith('a') ? 'agents' : 'files') : null;
      const q = scoped ? scoped[2] : m.query;
      if (q.includes('#') && scope !== 'files') return remoteConversationRows(q, m.start);
      // `@this`: el archivo de la pestaña que se está viendo
      const note = agentStore.activeNote;
      const self: Suggestion[] =
        scope !== 'agents' && note && 'this'.startsWith(q)
          ? [
              {
                label: '@this',
                detail: `Archivo actual · ${relOf(note.path)}`,
                pick: () => {
                  conv.attachActiveNote();
                  insertMention(m.start, fileMention(relOf(note.path)));
                },
              },
            ]
          : [];
      const vaultRows: Suggestion[] =
        scope === 'files'
          ? []
          : vaults
              .filter((v) => fuzzy(v.name.toLowerCase(), q))
              .slice(0, scope ? 8 : 5)
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
      // Con `@a:` también las conversaciones de esta ventana (`#2`)
      const localRows = scope === 'agents' ? localConversationRows(q, m.start) : [];
      const fileRows: Suggestion[] =
        scope === 'agents'
          ? []
          : vaultItemStore.items
              .filter((it) => fuzzy(it.relative_path.toLowerCase(), q))
              .slice(0, scope ? 10 : vaultRows.length ? 6 : 8)
              .map((it) => ({
                label: it.title || nameOf(it.relative_path),
                detail: it.relative_path,
                pick: () => {
                  conv.attach({ kind: 'file', path: absPath(it.abs_path ?? it.relative_path), name: nameOf(it.relative_path) });
                  insertMention(m.start, fileMention(it.relative_path));
                },
              }));
      const files = [...self, ...fileRows];
      const agents = [...vaultRows, ...localRows];
      // Encabezados con el atajo, solo cuando se ven los dos grupos
      if (!scope && files.length && agents.length) {
        files[0] = { ...files[0], header: 'Archivos · @f: para ver solo estos' };
        agents[0] = { ...agents[0], header: 'Bóvedas de Synapse · @a: para ver solo estas' };
      }
      // Si lo escrito es el comienzo del nombre de una bóveda, las bóvedas van primero
      const vaultFirst = !!q && vaults.some((v) => v.name.toLowerCase().startsWith(q) || vaultToken(v.name).slice(1).toLowerCase().startsWith(q));
      return vaultFirst ? [...agents, ...files] : [...files, ...agents];
    }
    if (slash !== null) {
      // `/feedback` es de Synapse (compilación `develop`): pide al agente feedback sobre el MCP
      const own = agentStore.feedbackAvailable && 'feedback'.startsWith(slash)
        ? [{
            label: '/feedback',
            detail: 'Synapse · el agente registra qué mejorar del MCP y del panel (puedes añadir indicaciones)',
            pick: () => {
              conv.draft = '/feedback ';
              tick().then(() => textarea?.focus());
            },
          }]
        : [];
      // `/usage` también es de Synapse, salvo que el agente traiga el suyo
      if ('usage'.startsWith(slash) && !conv.commands.some((c) => c.name === 'usage')) {
        own.push({
          label: '/usage',
          detail: 'Synapse · uso de esta conversación y, si el agente lo permite, cuota de la cuenta',
          pick: () => {
            conv.draft = '/usage';
            tick().then(() => textarea?.focus());
          },
        });
      }
      // `/btw` y `/fork` los resuelve Synapse aunque el agente tenga los suyos
      const native = !!conv.session?.capabilities.fork_session;
      const how = native ? 'contexto completo' : 'contexto reenviado como texto';
      // «Synapse» marca lo que resuelve Synapse; con `session/fork`, la bifurcación es del agente
      for (const [name, detail] of [
        ['btw', `Synapse · pregunta aparte, sin interrumpir al agente (${how})`],
        ['fork', `${native ? '' : 'Synapse · '}abre otra conversación a partir de esta (${how})`],
      ]) {
        if (!name.startsWith(slash)) continue;
        own.push({
          label: `/${name}`,
          detail,
          pick: () => {
            conv.draft = `/${name} `;
            tick().then(() => textarea?.focus());
          },
        });
      }
      return [...own, ...conv.commands
        .filter((c) => c.name.toLowerCase().startsWith(slash) && c.name !== 'btw' && c.name !== 'fork')
        .slice(0, 8)
        .map((c) => ({
          label: `/${c.name}`,
          detail: c.description ?? '',
          pick: () => {
            conv.draft = `/${c.name} `;
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
    const text = conv.draft.trim();
    if (!text || !ready) return;
    const own = /^\/(btw|fork)(?:\s+([\s\S]*))?$/.exec(text);
    if (own) {
      const rest = own[2] ?? '';
      if (own[1] === 'btw' && !rest) return;
      conv.draft = '';
      conv.pickedMentions = [];
      void (own[1] === 'btw' ? conv.ask(rest) : conv.fork(rest));
      return;
    }
    conv.draft = '';
    const { prompt, used } = mentionsPrompt(text, conv.pickedMentions);
    conv.pickedMentions = [];
    // Mientras el agente trabaja, el mensaje queda en cola y se envía al terminar el turno
    if (busy) {
      conv.enqueue(text, used.length ? { prompt, mentions: used } : {});
      return;
    }
    // Si se mencionó una bóveda que se está encendiendo, se espera a que responda
    const waiting = used.flatMap((m) => {
      const v = vaults.find((x) => m.to === x.name || m.to.startsWith(`${x.name}#`));
      return v && starting.has(v.path) ? [starting.get(v.path)!] : [];
    });
    if (waiting.length) await Promise.all(waiting);
    conv.send(text, used.length ? { prompt, mentions: used } : {});
  }

  /** Vuelve a poner en la caja los mensajes sacados de la cola (con sus menciones) */
  function restoreQueued(items: { text: string; mentions: Mention[] }[]) {
    if (!items.length) return;
    conv.draft = [...items.map((q) => q.text), conv.draft].filter((t) => t.trim()).join('\n\n');
    for (const q of items) for (const m of q.mentions) if (!conv.pickedMentions.some((x) => x.token === m.token)) conv.pickedMentions = [...conv.pickedMentions, m];
    tick().then(() => {
      textarea?.focus();
      textarea?.setSelectionRange(conv.draft.length, conv.draft.length);
    });
  }

  /** Detener: corta el turno y lo que estaba en cola vuelve a la caja de texto, sin enviarse */
  function stopTurn() {
    void conv.cancel();
    restoreQueued(conv.takeQueue());
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && conv.side) {
      e.preventDefault();
      e.stopPropagation();
      conv.closeSide();
      return;
    }
    if (suggestions.length) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const d = e.key === 'ArrowDown' ? 1 : -1;
        pickIndex = (pickIndex + d + suggestions.length) % suggestions.length;
        return;
      }
      // Enter con el comando ya escrito completo lo envía (no vuelve a elegir la misma sugerencia)
      const complete = e.key === 'Enter' && suggestions[pickIndex].label === conv.draft.trim();
      if ((e.key === 'Enter' || e.key === 'Tab') && !complete) {
        e.preventDefault();
        suggestions[pickIndex].pick();
        return;
      }
    }
    // ↑ con la caja vacía: el último mensaje en cola vuelve a la caja para editarlo (sale de la cola)
    if (e.key === 'ArrowUp' && !conv.draft && conv.queue.length) {
      e.preventDefault();
      const item = conv.unqueue(conv.queue[conv.queue.length - 1].id, true);
      if (item) restoreQueued([item]);
      return;
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
      attachPaths(JSON.parse(raw) as string[]);
    } catch {
      /* datos de arrastre inválidos */
    }
  }
  function attachPaths(paths: string[]) {
    for (const p of paths) conv.attach({ kind: 'file', path: absPath(p), name: nameOf(p) });
  }
  /** En Linux los datos del arrastre no llegan a la página: los trae el soltado nativo. */
  function nativeDrop(node: HTMLElement) {
    const onNative = (e: Event) => {
      const { paths, internal } = (e as CustomEvent<NativeDrop>).detail;
      dragOver = false;
      if (internal) attachPaths(paths);
    };
    node.addEventListener(NATIVE_DROP, onNative);
    return { destroy: () => node.removeEventListener(NATIVE_DROP, onNative) };
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

  // Una imagen del chat se abre en una pestaña con el visor de imágenes (zoom, mover, ajustar)
  function openImage(a: Extract<Attachment, { kind: 'image' }>) {
    agentImages.open({ name: a.name, mimeType: a.mimeType, data: a.data });
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
  const currentEffortName = $derived(
    conv.session?.efforts.find((e) => e.id === conv.currentEffort)?.name || conv.currentEffort || '',
  );
  /** El esfuerzo por defecto del modelo no se muestra en la etiqueta. */
  const effortShown = $derived(conv.currentEffort === 'default' ? '' : currentEffortName);
  const currentModeName = $derived(
    conv.session?.modes.find((m) => m.id === conv.currentMode)?.name || conv.currentMode || '',
  );
  /** El modo solo se muestra si no es el de inicio del agente. */
  const modeShown = $derived(
    conv.currentMode && conv.currentMode !== conv.session?.current_mode ? modeBadge(conv.currentMode, currentModeName) : '',
  );
  const optsTitle = $derived(
    [currentModelName && `Modelo: ${currentModelName}`, currentEffortName && `Esfuerzo: ${currentEffortName}`, currentModeName && `Modo: ${currentModeName}`].filter(Boolean).join('\n'),
  );

  // Cuota de la cuenta: el límite más apretado del modelo elegido
  const quota = $derived(agentStore.quotas[conv.agentId] ?? null);
  const quotaWindow = $derived(quota ? tightestWindow(groupsForModel(quota, currentModelName || conv.currentModel)) : null);
  $effect(() => {
    if (conv.session && agentStore.panelOpen) void agentStore.refreshQuota(untrack(() => conv.agentId));
  });

  const usagePct = $derived(conv.usage ? Math.min(100, (conv.usage.used / conv.usage.size) * 100) : 0);
</script>

<div class="acp-chat" bind:this={chatRoot}>
  {#if conv}
    {#if searchOpen && scroller}<ChatSearch {scroller} request={searchRequest} onClose={closeSearch} />{/if}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      class="acp-messages"
      tabindex="-1"
      bind:this={scroller}
      style={`zoom:${appSettings.chatZoom}`}
      oncopy={(e) => {
        // Con casillas de tarea: se copian como ☐ / ✅ (como en el modo lectura de las notas)
        const out = taskAwareCopy(window.getSelection(), e.currentTarget as HTMLElement);
        if (!out || !e.clipboardData) return;
        e.preventDefault();
        e.clipboardData.setData('text/html', out.html);
        e.clipboardData.setData('text/plain', out.text);
      }}
    >
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
          <div class="acp-user-wrap">
          <div class="acp-msg user markdown" class:from-agent={!!item.from}>
            {#if item.from}<div class="acp-from">
                {#if item.from.vault}<OctopusIcon size={12} title="De otra bóveda de Synapse" />{/if}
                De {item.from.agent} · {item.from.handle}
              </div>{/if}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="acp-user-text" onclick={onMessageClick}>{@html renderUserMarkdown(item.text, item.mentions)}</div>
            {#if item.attachments.length}
              <div class="acp-chips sent">
                {#each item.attachments as a, i (i)}
                  {#if a.kind === 'image'}
                    <button type="button" class="acp-image-btn" title="Ver {a.name}" onclick={() => openImage(a)}>
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
            <div class="acp-msg-actions">
              <button type="button" class="acp-copy-user" title="Copiar el texto tal como lo escribiste (para reusarlo)" onclick={() => copyMarkdown(String(item.id), item.text)}>
                {#if copied?.id === String(item.id) && copied.how === 'md'}<Check size={11} /> Copiado{:else}<Copy size={11} /> Copiar{/if}
              </button>
            </div>
          </div>
        {:else if item.type === 'agent'}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div class="acp-agent-wrap">
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="acp-msg agent markdown" onclick={onMessageClick}>{@html renderAgentMarkdown(item.text)}</div>
            <div class="acp-msg-actions">
              <button type="button" class="acp-copy-md" title="Copiar el Markdown tal cual" onclick={() => copyMarkdown(item.id, item.text)}>
                {#if copied?.id === item.id && copied.how === 'md'}<Check size={11} /> Copiado{:else}<Copy size={11} /> Markdown{/if}
              </button>
              <button
                type="button"
                class="acp-copy-rich"
                title="Copiar con formato, para pegar en un correo o en Teams (las tareas como ☐ / ✅)"
                onclick={(e) => copyRich(item.id, e.currentTarget)}
              >
                {#if copied?.id === item.id && copied.how === 'rich'}<Check size={11} /> Copiado{:else}<ClipboardType size={11} /> Con formato{/if}
              </button>
            </div>
          </div>
        {:else if item.type === 'thought'}
          <details class="acp-thought">
            <summary title="Razonamiento del agente">
              <span aria-hidden="true">🧠</span>
              <span class="acp-thought-label">Razonamiento<span class="acp-thought-ellipsis" aria-hidden="true">…</span></span>
            </summary>
            <div class="acp-thought-body">{item.text.trim()}</div>
          </details>
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
        {:else if item.type === 'elicitation'}
          <ElicitationCard
            request={item.request}
            answered={item.answered}
            onRespond={(action, content) => conv.answerElicitation(item.id, action, content)}
          />
        {:else if item.type === 'error'}
          <div class="acp-msg error"><AlertCircle size={13} /> {item.text}</div>
        {:else if item.type === 'info'}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div class="acp-msg info markdown" onclick={onMessageClick}>{@html renderAgentMarkdown(item.text)}</div>
        {:else if item.type === 'compact'}
          <div class="acp-compact-marker" role="separator">— contexto compactado —</div>
        {:else if item.type === 'fork'}
          <div class="acp-compact-marker acp-fork-marker" role="separator" title={item.title}>— bifurcada de {item.title || item.handle} —</div>
        {/if}
      {/each}
      {#if busy}
        <div class="acp-status acp-thinking"><span class="acp-brain" aria-hidden="true">🧠</span> Pensando…</div>
      {/if}
    </div>

    {#if conv.session || conv.usage}
      <div class="acp-usage-row">
        {#if conv.session && (conv.session.models.length || conv.session.efforts.length || conv.session.modes.length)}
          <div class="acp-agent-opts">
            <button
              type="button"
              class="acp-opts-btn"
              title={optsTitle}
              aria-label="Modelo, esfuerzo y modo del agente"
              aria-haspopup="menu"
              aria-expanded={optsOpen}
              onclick={(e) => {
                e.stopPropagation();
                optsOpen = !optsOpen;
              }}
            >
              {#if modeShown}<span class="acp-mode-chip">{modeShown}</span>{/if}
              {#if modelShort}<span class="acp-model-chip">{modelShort}{effortShown ? ` · ${effortShown}` : ''}</span>{/if}
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
                {#if conv.session.efforts.length}
                  <div class="acp-opts-head">Esfuerzo</div>
                  {#each conv.session.efforts as e (e.id)}
                    <button
                      type="button"
                      role="menuitemradio"
                      aria-checked={e.id === conv.currentEffort}
                      class:selected={e.id === conv.currentEffort}
                      title={e.description ?? ''}
                      onclick={() => {
                        optsOpen = false;
                        conv.setEffort(e.id);
                      }}>{e.name || e.id}</button
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
        {#if quota && quotaWindow}
          <button
            type="button"
            class="acp-quota {quotaLevel(quotaWindow.remaining)}"
            aria-label={t('quota.label')}
            title={quotaTooltip(quota, conv.agent?.name ?? conv.agentId)}
            onclick={() => agentStore.refreshQuota(conv.agentId, { now: true })}
          >
            <span class="acp-quota-dot" aria-hidden="true"></span>
            {percent(quotaWindow.remaining)}
            <span class="acp-quota-window">{quotaWindow.window === 'weekly' ? t('quota.weeklyShort') : windowLabel(quotaWindow.window)}</span>
          </button>
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
      use:nativeDrop
      class:drag-over={dragOver}
      ondragover={(e) => {
        if (internalDragPaths() || e.dataTransfer?.types.includes(DRAG_MIME)) {
          e.preventDefault();
          dragOver = true;
        }
      }}
      ondragleave={() => (dragOver = false)}
      ondrop={onDrop}
    >
      {#if conv.side}
        <SideQuestion
          side={conv.side}
          zoom={appSettings.chatZoom}
          onLink={onMessageClick}
          onClose={() => {
            conv.closeSide();
            textarea?.focus();
          }}
          onPromote={() => conv.promoteSide()}
        />
      {/if}
      {#if suggestions.length}
        <div class="acp-suggest" role="listbox">
          {#each suggestions as sug, i (sug.detail + sug.label)}
            {#if sug.header}<div class="acp-suggest-header">{sug.header}</div>{/if}
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
      {#if conv.queue.length}
        <div class="acp-queue" aria-label="Mensajes en cola">
          {#each conv.queue as q (q.id)}
            <div class="acp-queued">
              <Clock size={11} />
              <span class="acp-queued-text" title={q.text}>{q.text}</span>
              {#if q.attachments.length}<span class="acp-queued-att">+{q.attachments.length}</span>{/if}
              <button type="button" aria-label="Quitar de la cola" title="Quitar de la cola" onclick={() => conv.unqueue(q.id)}><X size={10} /></button>
            </div>
          {/each}
          <div class="acp-queue-hint">En cola: se envía al terminar el turno · ↑ edita el último · Detener lo devuelve a la caja</div>
        </div>
      {/if}
      {#if conv.pendingAttachments.length}
        <div class="acp-chips">
          {#each conv.pendingAttachments as a (a.kind + a.path + (a.kind === 'selection' ? a.startLine : ''))}
            <span class="acp-chip" class:image={a.kind === 'image'} title={a.kind === 'image' ? imageTitle : a.path}>
              {#if a.kind === 'image'}<button type="button" class="acp-image-btn" title="Ver la imagen" onclick={() => openImage(a)}><img src="data:{a.mimeType};base64,{a.data}" alt="" /></button><ImageIcon size={11} />{:else if a.kind === 'selection'}<TextSelect size={11} />{:else}<FileText size={11} />{/if}
              {chipLabel(a)}
              <button type="button" aria-label="Quitar {chipLabel(a)}" onclick={() => conv.detach(a)}><X size={10} /></button>
            </span>
          {/each}
        </div>
      {/if}
      <form
        style={`zoom:${appSettings.chatZoom}`}
        class="acp-input"
        onsubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <textarea
          bind:this={textarea}
          bind:value={conv.draft}
          onkeydown={onKeydown}
          onpaste={onPaste}
          onkeyup={(e) => (caret = e.currentTarget.selectionStart)}
          onclick={(e) => (caret = e.currentTarget.selectionStart)}
          oninput={(e) => (caret = e.currentTarget.selectionStart)}
          rows="3"
          placeholder={busy
            ? `${conv.agent?.name ?? 'El agente'} está trabajando: Enter deja el mensaje en cola, /btw pregunta aparte`
            : ready
            ? `Escribe a ${conv.agent?.name ?? 'el agente'}… @ nota, / comando, Enter envía`
            : 'El agente no está conectado'}
          disabled={!ready}
        ></textarea>
        {#if busy}
          <button type="button" class="acp-send" title="Detener (lo que esté en cola vuelve a la caja)" onclick={stopTurn}>
            <Square size={14} />
          </button>
        {:else}
          <button type="submit" class="acp-send" title="Enviar" disabled={!ready || !conv.draft.trim()}>
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
  .acp-messages:focus {
    outline: none;
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
  .acp-user-wrap {
    align-self: flex-end;
    max-width: 85%;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
  }
  .acp-user-wrap:hover .acp-msg-actions,
  .acp-user-wrap .acp-msg-actions:focus-within {
    opacity: 1;
  }
  .acp-msg.user :global(.acp-mention) {
    padding: 0 4px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--accent-primary, #6366f1) 18%, transparent);
    font-weight: 500;
    cursor: help;
  }
  .acp-msg.user {
    align-self: flex-end;
    max-width: 100%;
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
  }
  .acp-msg.agent {
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
  }
  .acp-agent-wrap {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .acp-msg-actions {
    display: flex;
    gap: 4px;
    justify-content: flex-end;
    opacity: 0;
    transition: opacity 0.12s;
  }
  .acp-agent-wrap:hover .acp-msg-actions,
  .acp-msg-actions:focus-within {
    opacity: 1;
  }
  .acp-msg-actions button {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 1px 6px;
    border: 1px solid var(--border-primary);
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
  }
  .acp-msg-actions button:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
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
  }
  .acp-thought:not([open]) {
    align-self: flex-start;
  }
  .acp-thought summary {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    list-style: none;
    cursor: pointer;
    user-select: none;
  }
  .acp-thought summary::-webkit-details-marker {
    display: none;
  }
  .acp-thought[open] .acp-thought-ellipsis {
    display: none;
  }
  .acp-thought-body {
    margin-top: 2px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    line-height: 1.4;
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
  .acp-queue {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin-bottom: 6px;
  }
  .acp-queued {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 6px;
    border: 1px dashed var(--border-primary);
    border-radius: 6px;
    background: var(--bg-secondary);
    color: var(--text-secondary);
    font-size: 12px;
  }
  .acp-queued-text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-primary);
  }
  .acp-queued-att {
    font-size: 11px;
  }
  .acp-queued button {
    display: inline-flex;
    padding: 2px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 3px;
  }
  .acp-queued button:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .acp-queue-hint {
    font-size: 11px;
    color: var(--text-muted, var(--text-secondary));
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
  .acp-suggest-header {
    padding: 4px 8px 2px;
    font-size: 10.5px;
    color: var(--text-muted);
    text-transform: none;
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
  .acp-image-btn {
    display: inline-flex;
    padding: 0;
    border: none;
    background: none;
    cursor: zoom-in;
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
  .acp-quota {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0 2px;
    border: none;
    background: none;
    color: var(--text-muted);
    font-size: 10.5px;
    white-space: nowrap;
    cursor: pointer;
  }
  .acp-quota:hover {
    color: var(--text-primary);
  }
  .acp-quota-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--success, #2da44e);
  }
  .acp-quota.warn .acp-quota-dot {
    background: var(--warning, #bf8700);
  }
  .acp-quota.low {
    color: var(--danger, #cf222e);
  }
  .acp-quota.low .acp-quota-dot {
    background: var(--danger, #cf222e);
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
    max-height: 40vh;
    overflow-y: auto;
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
