<script lang="ts">
  import { i18n, t } from '@shared/lib/i18n';
  import { ContextMenu } from 'bits-ui';
  import { internalDragPaths, NATIVE_DROP, type NativeDrop } from '@shared/lib/native-drop';
  import {
    agentStore,
    AgentIcon,
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
  // La pregunta aparte lleva dentro este mismo chat
  import Self from './AcpChat.svelte';
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
    Folder,
    TextSelect,
    ImageIcon,
    X,
    Circle,
    CircleCheck,
    CircleDot,
    Shrink,
    ChevronDown,
    Copy,
    Clock,
    Check,
    MessageCircleQuestion,
  } from 'lucide-svelte';
  import { tick, untrack } from 'svelte';
  import { OctopusIcon } from '@shared/ui/icons';
  import { renderAgentMarkdown, renderUserMarkdown, vaultPathOfLink, formatRaw } from '../lib/markdown';
  import { blockCopy, singleBlockCopy } from '../lib/copy-blocks';
  import { taskAwareCopy, richCopyOfElement, writeRichClipboard } from '@shared/lib/rich-copy';
  import { appSettings } from '@entities/settings';
  import { stepViewerZoom } from '@shared/lib/viewer-zoom';
  import {
    ago,
    conversationTooltip,
    fileMention,
    mentionsPrompt,
    foldersOf,
    folderMention,
    bestMatches,
    matches,
    mentionAt,
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
  /** El chat de una pregunta aparte (`/btw`): sin otras `/btw` ni `/fork`, ni modo, cuota o menciones a conversaciones. */
  const isAside = $derived(conv.isAside);
  const items = $derived(conv.history.length ? [...conv.history, ...conv.items] : conv.items);

  // Enlaces en las respuestas: notas de la bóveda se abren en Synapse; el resto, fuera
  /** Tamaño del mensaje de otro agente, para el resumen cerrado. */
  const incomingSize = (text: string) => {
    const bytes = new TextEncoder().encode(text).length;
    if (bytes < 1024) return `${bytes} B`;
    const kb = (bytes / 1024).toFixed(1);
    return `${i18n.locale === 'es' ? kb.replace('.', ',') : kb} KB`;
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
      // `defaultPrevented`: ya lo atendió el chat de la pregunta aparte, que está dentro
      if (!e.ctrlKey || !e.deltaY || e.defaultPrevented) return;
      e.preventDefault();
      appSettings.setViewerZoom('chat', stepViewerZoom(appSettings.chatZoom, e.deltaY < 0 ? 1 : -1));
    };
    el.addEventListener('wheel', onWheel, { passive: false });
    return () => el.removeEventListener('wheel', onWheel);
  });

  // Copiar: una respuesta va como Markdown (texto plano) y con formato (HTML, casillas como ☐ / ✅);
  // quien pega elige: un campo de texto toma el Markdown y un correo, el formato
  let copied = $state<string | null>(null);
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;
  function flagCopied(id: string) {
    copied = id;
    clearTimeout(copiedTimer);
    copiedTimer = setTimeout(() => (copied = null), 1500);
  }
  function copyMarkdown(id: string, text: string) {
    navigator.clipboard?.writeText(text).then(() => flagCopied(id)).catch(() => {});
  }
  function copyReply(id: string, text: string, button: HTMLElement) {
    const msg = button.closest('.acp-agent-wrap')?.querySelector<HTMLElement>('.acp-msg.agent');
    if (msg) writeRichClipboard({ html: richCopyOfElement(msg).html, text }).then((ok) => ok && flagCopied(id));
  }
  // Clic derecho en una herramienta: copiar lo marcado dentro de ella, o todo (nombre, entrada y salida)
  let toolMenu = $state<{ el: HTMLElement; selection: string } | null>(null);
  function openToolMenu(el: HTMLElement) {
    const sel = window.getSelection();
    const inside = !!sel?.anchorNode && el.contains(sel.anchorNode);
    toolMenu = { el, selection: inside ? sel.toString() : '' };
  }
  /** El texto de una herramienta, esté o no desplegada, con una línea por parte. */
  function toolText(el: HTMLElement | undefined): string {
    if (!el) return '';
    const parts = [...el.children].filter((c) => c.tagName !== 'SUMMARY' && !c.classList.contains('empty'));
    const text = (c: Element) =>
      c.classList.contains('acp-diff') ? [...c.children].map((line) => line.textContent).join('\n') : c.textContent;
    return [el.querySelector('.acp-tool-title'), ...parts].map((c) => (c ? text(c) : '')).join('\n');
  }
  function copyPlain(text: string) {
    navigator.clipboard?.writeText(text).catch(() => {});
  }
  // Clic derecho en una respuesta: copiar el bloque que hay debajo, o la respuesta entera
  let replyMenu = $state<{ id: number; message: HTMLElement; block: Element | null } | null>(null);
  /** Al pulsar el botón derecho, antes de que salga el menú: decide cuál sale. */
  function aimReplyMenu(e: PointerEvent, id: number) {
    if (e.button !== 2) return;
    const message = e.currentTarget as HTMLElement;
    const target = e.target as Element;
    // Con texto marcado, o sobre un enlace o una imagen, queda el menú del sistema: su «Copiar» ya
    // lleva el Markdown de los bloques completos
    if (window.getSelection()?.isCollapsed === false || target.closest('a, img')) {
      replyMenu = null;
      return;
    }
    let block: Element | null = target;
    while (block && block.parentElement !== message) block = block.parentElement;
    replyMenu = { id, message, block };
  }
  function copyBlock(source: string) {
    if (replyMenu?.block) void writeRichClipboard(singleBlockCopy(replyMenu.message, replyMenu.block, source));
  }
  function copyWholeReply(source: string) {
    if (replyMenu) void writeRichClipboard({ html: richCopyOfElement(replyMenu.message).html, text: source });
  }
  const replySource = (id: string) => {
    const item = conv?.items.find((i) => String(i.id) === id);
    return item?.type === 'agent' ? item.text : undefined;
  };

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
  // Una pregunta aparte se conecta al volver a preguntarle: su caja admite texto sin sesión
  const ready = $derived(conv.connection === 'ready' || busy || (isAside && (conv.connection === 'idle' || conv.connection === 'error')));

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

  // Autocompletado: `@nota` o `@carpeta` la mencionan, `/comando` al inicio usa un comando del agente
  const vaultFolders = $derived(foldersOf(vaultItemStore.items.map((it) => it.relative_path)));
  let caret = $state(0);
  const mention = $derived(mentionAt(conv.draft.slice(0, caret), conv.pickedMentions));
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
    pick: () => insertMention(start, { token, to, title: t('acp.chat.convTitle', { title: c.title, agent: c.agent, where }) }),
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

  type Suggestion = { label: string; detail: string; tooltip?: string; header?: string; pick: () => void };

  /** Conversaciones de esta ventana (`#2`) que coinciden con `query`. */
  function localConversationRows(query: string, start: number): Suggestion[] {
    if (isAside) return [];
    return agentStore.conversations
      .filter((c) => c.id !== conv.id)
      .filter((c) => `${c.number} ${c.title} ${c.agent?.name ?? c.agentId}`.toLowerCase().includes(query))
      .slice(0, 8)
      .map((c) =>
        conversationRow(
          {
            handle: c.handle,
            agent: c.agent?.name ?? c.agentId,
            title: c.title || t('acp.store.newConversation'),
            status: c.status,
            last_reply: c.lastText('agent'),
            last_question: c.lastText('user'),
            last_activity: c.lastActivity,
          },
          c.handle,
          c.handle,
          t('acp.chat.inThisWindow'),
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
        return conversationRow(c, `${vaultToken(v.name)}#${n}`, `${v.name}#${n}`, t('acp.chat.inVault', { vault: v.name }), start);
      });
    const newTag = t('acp.mentions.newTag');
    if (newTag.startsWith(cq)) {
      rows.push({
        label: `${vaultToken(v.name)}#${newTag}`,
        detail: t('acp.chat.openNewRemoteDetail'),
        pick: () => {
          if (v.state === 'off') startVault(v);
          insertMention(start, {
            token: `${vaultToken(v.name)}#${newTag}`,
            to: `${v.name}#${newTag}`,
            title: t('acp.chat.newConvInVaultTitle', { vault: v.name }),
          });
        },
      });
    }
    return rows;
  }
  const suggestions = $derived.by((): Suggestion[] => {
    if (hash) return localConversationRows(hash.query, hash.start);
    if (mention) {
      // Filtro opcional: `@agent:` / `@a:` solo agentes (bóvedas y conversaciones), `@file:` / `@f:` solo
      // archivos y carpetas. El prefijo solo filtra: al elegir se inserta la mención sin él.
      const m = mention;
      const scoped = /^(agents?|a|files?|f):(.*)$/.exec(m.query);
      const scope = scoped ? (scoped[1].startsWith('a') ? 'agents' : 'files') : null;
      const q = scoped ? scoped[2] : m.query;
      if (q.includes('#') && scope !== 'files') return isAside ? [] : remoteConversationRows(q, m.start);
      // `@this`: el archivo de la pestaña que se está viendo
      const note = agentStore.activeNote;
      const self: Suggestion[] =
        scope !== 'agents' && note && 'this'.startsWith(q)
          ? [
              {
                label: '@this',
                detail: t('acp.chat.currentFileDetail', { path: relOf(note.path) }),
                pick: () => {
                  conv.attachActiveNote();
                  insertMention(m.start, fileMention(relOf(note.path)));
                },
              },
            ]
          : [];
      const vaultRows: Suggestion[] =
        scope === 'files' || isAside
          ? []
          : vaults
              .filter((v) => matches(v.name.toLowerCase(), q))
              .slice(0, scope ? 8 : 5)
              .map((v) => ({
                label: `${VAULT_ICON[v.state]} ${vaultToken(v.name)}`,
                detail: startingPaths.includes(v.path)
                  ? t('acp.chat.startingVault')
                  : v.state === 'off'
                    ? t('acp.chat.vaultOffDetail')
                    : t('acp.chat.vaultOnDetail', { count: v.conversations.length }),
                tooltip: v.path,
                pick: () => {
                  if (v.state === 'off') startVault(v);
                  insertMention(m.start, {
                    token: vaultToken(v.name),
                    to: v.name,
                    title: t('acp.chat.vaultMentionTitle', { vault: v.name }),
                  });
                },
              }));
      // Con `@a:` también las conversaciones de esta ventana (`#2`)
      const localRows = scope === 'agents' ? localConversationRows(q, m.start) : [];
      const fileRows: Suggestion[] =
        scope === 'agents'
          ? []
          : bestMatches(vaultItemStore.items, (it) => it.relative_path, q, scope ? 10 : vaultRows.length ? 6 : 8)
              .map((it) => ({
                label: it.title || nameOf(it.relative_path),
                detail: it.relative_path,
                pick: () => {
                  conv.attach({ kind: 'file', path: absPath(it.abs_path ?? it.relative_path), name: nameOf(it.relative_path) });
                  insertMention(m.start, fileMention(it.relative_path));
                },
              }));
      // Carpetas: las que empiezan así van antes que los archivos (`@docs` → `docs/`)
      const startsFolder = (path: string) => !!q && `${nameOf(path).toLowerCase()}/`.startsWith(q);
      const folders =
        scope === 'agents' || !q
          ? []
          : vaultFolders
              .filter((path) => matches(`${path.toLowerCase()}/`, q))
              .sort((a, b) => Number(startsFolder(b)) - Number(startsFolder(a)))
              .slice(0, scope ? 8 : 3);
      const folderRows: Suggestion[] = folders.map((path) => ({
        label: `📁 ${nameOf(path)}/`,
        detail: `${path}/`,
        pick: () => {
          conv.attach({ kind: 'file', path: absPath(path), name: `${nameOf(path)}/` });
          insertMention(m.start, folderMention(path));
        },
      }));
      const files = folders.some(startsFolder) ? [...self, ...folderRows, ...fileRows] : [...self, ...fileRows, ...folderRows];
      const agents = [...vaultRows, ...localRows];
      // Encabezados con el atajo, solo cuando se ven los dos grupos
      if (!scope && files.length && agents.length) {
        files[0] = { ...files[0], header: t('acp.chat.filesHeader') };
        agents[0] = { ...agents[0], header: t('acp.chat.vaultsHeader') };
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
            detail: t('acp.chat.feedbackDetail'),
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
          detail: t('acp.chat.usageDetail'),
          pick: () => {
            conv.draft = '/usage';
            tick().then(() => textarea?.focus());
          },
        });
      }
      // `/btw` y `/fork` los resuelve Synapse aunque el agente tenga los suyos
      const native = !!conv.session?.capabilities.fork_session;
      const how = native ? t('acp.chat.forkFullContext') : t('acp.chat.forkReplayContext');
      // «Synapse» marca lo que resuelve Synapse; con `session/fork`, la bifurcación es del agente
      for (const [name, detail, tooltip] of [
        ['btw', t('aside.commandDetail', { how }), t('aside.commandTooltip', { agent: conv.agent?.name ?? conv.agentId })],
        ['fork', t('acp.chat.forkDetail', { prefix: native ? '' : 'Synapse · ', how }), undefined],
      ]) {
        if (isAside || !name!.startsWith(slash)) continue;
        own.push({
          label: `/${name}`,
          detail: detail!,
          tooltip,
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

  /** La pregunta aparte a la vista; sin ella, las que haya quedan recogidas en su tarjeta. */
  const openAside = $derived(conv.asides.find((a) => a.id === conv.openAside) ?? null);
  let sidePanel = $state<ReturnType<typeof SideQuestion> | null>(null);
  const minimizedAsides = $derived(openAside ? 0 : conv.asides.length);
  const asideWorking = $derived(conv.asides.some((a) => a.status === 'working'));
  const asideUnread = $derived(conv.asides.some((a) => a.unseen));

  /** La tarjeta abre la que tenga una respuesta sin leer o, si no, la última. */
  async function openAsides() {
    const aside = conv.asides.findLast((a) => a.unseen) ?? conv.asides.at(-1);
    if (!aside) return;
    conv.showAside(aside.id);
    await tick();
    sidePanel?.focus();
  }

  async function submit() {
    const text = conv.draft.trim();
    if (!text || !ready) return;
    const own = /^\/(btw|fork)(?:\s+([\s\S]*))?$/.exec(text);
    if (own && isAside) {
      conv.push({ type: 'info', text: t('aside.noNested', { command: `/${own[1]}` }) });
      return;
    }
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
    const opts = used.length ? { prompt, mentions: used } : {};
    if (isAside) void conv.askAside(text, opts);
    else conv.send(text, opts);
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
    if (e.key === 'Escape' && openAside) {
      e.preventDefault();
      e.stopPropagation();
      conv.minimizeAsides();
      return;
    }
    if (suggestions.length) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const d = e.key === 'ArrowDown' ? 1 : -1;
        pickIndex = (pickIndex + d + suggestions.length) % suggestions.length;
        return;
      }
      // Enter con el comando ya escrito completo lo envía (no vuelve a elegir la misma sugerencia), y
      // también si la mención acaba en un espacio: se siguió escribiendo (Tab sí la elige)
      const complete = e.key === 'Enter' && (suggestions[pickIndex].label === conv.draft.trim() || !!mention?.query.endsWith(' '));
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
      ? t('acp.chat.imageNotSupportedTitle')
      : t('acp.chat.pastedImageTitle'),
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
        conv.attach({ kind: 'image', path: `pegada-${Date.now()}-${pasted}`, name: t('acp.chat.pastedImageName', { n: pasted, ext }), mimeType: file.type, data });
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
    return a.startLine ? `${a.name} L${a.startLine}–${a.endLine}` : t('acp.chat.selectionFragment', { name: a.name });
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

  const statusText = $derived<Record<string, string>>({
    installing: t('acp.chat.installing'),
    starting: t('acp.chat.starting'),
  });

  const compact = (n: number) =>
    n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)}M` : n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n);
  function formatCost(c: { amount: number; currency: string }) {
    try {
      return new Intl.NumberFormat(i18n.locale, { style: 'currency', currency: c.currency || 'USD', maximumFractionDigits: 4 }).format(c.amount);
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
    [
      currentModelName && t('acp.chat.modelLabel', { name: currentModelName }),
      currentEffortName && t('acp.chat.effortLabel', { name: currentEffortName }),
      currentModeName && t('acp.chat.modeLabel', { name: currentModeName }),
    ]
      .filter(Boolean)
      .join('\n'),
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
        // Bloques completos de una respuesta: su Markdown original. En un fragmento con casillas de
        // tarea, estas van como ☐ / ✅ (como en el modo lectura de las notas)
        // `defaultPrevented`: ya lo escribió el botón «Copiar» (aunque haya una selección) o el chat
        // de la pregunta aparte, si está dentro
        if (e.defaultPrevented) return;
        const root = e.currentTarget as HTMLElement;
        const sel = window.getSelection();
        const out = blockCopy(sel, root, replySource) ?? taskAwareCopy(sel, root);
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
            <Bot size={14} /> {t('acp.chat.connectWith', { agent: conv.agent?.name ?? t('acp.chat.fallbackAgentLower') })}
          </button>
        </div>
      {/if}

      {#if conv.plan.length}
        <div class="acp-plan" aria-label={t('acp.chat.planAria')}>
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

      {#each items as item (item.id)}
        {#if item.type === 'user' && item.from}
          <!-- Mensaje de otro agente: cerrado; al abrirlo se ve renderizado (tu agente ya lo resume) -->
          <details class="acp-msg user from-agent acp-incoming">
            <summary class="acp-from">
              {#if item.from.vault}<OctopusIcon size={12} title={t('acp.chat.fromAnotherVault')} />{/if}
              {t('acp.chat.fromHeader', { agent: item.from.agent, handle: item.from.handle })}
              <span class="acp-incoming-size">{incomingSize(item.text)}</span>
            </summary>
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="markdown" onclick={onMessageClick}>{@html renderAgentMarkdown(item.text)}</div>
            <button type="button" class="acp-incoming-copy" title={t('acp.chat.copyIncomingTitle')} onclick={() => navigator.clipboard?.writeText(item.text)}>
              <Copy size={11} /> {t('acp.chat.copyText')}
            </button>
          </details>
        {:else if item.type === 'user'}
          <div class="acp-user-wrap">
          <div class="acp-msg user markdown" class:from-agent={!!item.from}>
            {#if item.from}<div class="acp-from">
                {#if item.from.vault}<OctopusIcon size={12} title={t('acp.chat.fromAnotherVault')} />{/if}
                {t('acp.chat.fromHeader', { agent: item.from.agent, handle: item.from.handle })}
              </div>{/if}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="acp-user-text" onclick={onMessageClick}>{@html renderUserMarkdown(item.text, item.mentions)}</div>
            {#if item.attachments.length}
              <div class="acp-chips sent">
                {#each item.attachments as a, i (i)}
                  {#if a.kind === 'image'}
                    <button type="button" class="acp-image-btn" title={t('acp.chat.viewImage', { name: a.name })} onclick={() => openImage(a)}>
                      <img class="acp-sent-image" src="data:{a.mimeType};base64,{a.data}" alt={a.name} />
                    </button>
                  {:else}
                    <span class="acp-chip" title={a.path}
                      >{#if a.kind === 'selection'}<TextSelect size={11} />{:else if a.name.endsWith('/')}<Folder size={11} />{:else}<FileText size={11} />{/if}{chipLabel(a)}</span
                    >
                  {/if}
                {/each}
              </div>
            {/if}
          </div>
            <div class="acp-msg-actions">
              <button type="button" class="acp-copy-user" title={t('acp.chat.copyUserTitle')} onclick={() => copyMarkdown(String(item.id), item.text)}>
                {#if copied === String(item.id)}<Check size={11} /> {t('replyCopy.done')}{:else}<Copy size={11} /> {t('replyCopy.copy')}{/if}
              </button>
            </div>
          </div>
        {:else if item.type === 'agent'}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div class="acp-agent-wrap">
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <!-- El bloque que se copiaría se marca mientras el menú está abierto -->
            <ContextMenu.Root onOpenChange={(open) => replyMenu?.block?.classList.toggle('acp-copy-target', open)}>
              <ContextMenu.Trigger disabled={replyMenu?.id !== item.id}>
                {#snippet child({ props })}
                  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                  <div {...props} class="acp-msg agent markdown" data-reply={item.id} onclick={onMessageClick} onpointerdowncapture={(e) => aimReplyMenu(e, item.id)}>{@html renderAgentMarkdown(item.text)}</div>
                {/snippet}
              </ContextMenu.Trigger>
              <ContextMenu.Portal>
                <ContextMenu.Content class="acp-context-menu">
                  {#if replyMenu?.block}
                    <ContextMenu.Item class="acp-context-item" onSelect={() => copyBlock(item.text)}>
                      <TextSelect size={13} /> {t('replyCopy.block')}
                    </ContextMenu.Item>
                  {/if}
                  <ContextMenu.Item class="acp-context-item" onSelect={() => copyWholeReply(item.text)}>
                    <Copy size={13} /> {t('replyCopy.reply')}
                  </ContextMenu.Item>
                </ContextMenu.Content>
              </ContextMenu.Portal>
            </ContextMenu.Root>
            <div class="acp-msg-actions">
              <button type="button" class="acp-copy-reply" title={t('replyCopy.hint')} onclick={(e) => copyReply(item.id, item.text, e.currentTarget)}>
                {#if copied === item.id}<Check size={11} /> {t('replyCopy.done')}{:else}<Copy size={11} /> {t('replyCopy.copy')}{/if}
              </button>
            </div>
          </div>
        {:else if item.type === 'thought'}
          <details class="acp-thought">
            <summary title={t('acp.chat.thoughtSummaryTitle')}>
              <span aria-hidden="true">🧠</span>
              <span class="acp-thought-label">{t('acp.chat.thoughtLabel')}<span class="acp-thought-ellipsis" aria-hidden="true">…</span></span>
            </summary>
            <div class="acp-thought-body">{item.text.trim()}</div>
          </details>
        {:else if item.type === 'tool'}
          <ContextMenu.Root>
            <ContextMenu.Trigger>
              {#snippet child({ props })}
                <details {...props} class="acp-tool" data-status={item.status ?? ''} oncontextmenucapture={(e) => openToolMenu(e.currentTarget)}>
                  <summary>
                    <Wrench size={13} />
                    <span class="acp-tool-title">{item.title}</span>
                    {#if item.status}<span class="acp-tool-status">{item.status}</span>{/if}
                  </summary>
                  {#if item.kind}<div class="acp-tool-loc">{t('acp.chat.toolKind', { kind: item.kind })}</div>{/if}
                  {#each item.locations as loc, i (i)}
                    <div class="acp-tool-loc">{relOf(loc.path)}{loc.line ? `:${loc.line}` : ''}</div>
                  {/each}
                  {#if item.rawInput != null}
                    <div class="acp-tool-label">{t('acp.chat.toolInput')}</div>
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
                    <div class="acp-tool-label">{t('acp.chat.toolOutput')}</div>
                    <pre class="acp-tool-text">{formatRaw(item.rawOutput)}</pre>
                  {/if}
                  {#if item.rawInput == null && item.rawOutput == null && !item.content.length && !item.locations.length}
                    <div class="acp-tool-loc empty">{t('acp.chat.toolNoDetails')}</div>
                  {/if}
                </details>
              {/snippet}
            </ContextMenu.Trigger>
            <ContextMenu.Portal>
              <ContextMenu.Content class="acp-context-menu">
                {#if toolMenu?.selection}
                  <ContextMenu.Item class="acp-context-item" onSelect={() => copyPlain(toolMenu?.selection ?? '')}>
                    <TextSelect size={13} /> {t('replyCopy.selection')}
                  </ContextMenu.Item>
                {/if}
                <ContextMenu.Item class="acp-context-item" onSelect={() => copyPlain(toolText(toolMenu?.el))}>
                  <Copy size={13} /> {t('replyCopy.copy')}
                </ContextMenu.Item>
              </ContextMenu.Content>
            </ContextMenu.Portal>
          </ContextMenu.Root>
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
                {item.request.options.find((o) => o.id === item.answered)?.name ?? t('acp.chat.cancelled')}
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
          <div class="acp-compact-marker" role="separator">{t('acp.chat.compactMarker')}</div>
        {:else if item.type === 'context'}
          <details class="acp-context">
            <summary>{t('aside.contextSent')} <span class="acp-incoming-size">{incomingSize(item.text)}</span></summary>
            <pre>{item.text}</pre>
          </details>
        {:else if item.type === 'fork'}
          <div class="acp-compact-marker acp-fork-marker" role="separator" title={item.title}>{t('acp.chat.forkMarker', { label: item.title || item.handle })}</div>
        {/if}
      {/each}
      {#if busy}
        <div class="acp-status acp-thinking"><span class="acp-brain" aria-hidden="true">🧠</span> {t('acp.chat.thinking')}</div>
      {/if}
      <!-- Marca de agua: logo del agente en uso, fija en la esquina inferior derecha -->
      <div class="acp-watermark" aria-hidden="true" style:opacity={appSettings.agentWatermark / 100}><AgentIcon agentId={conv.agentId} size={72} /></div>
    </div>

    {#if conv.session || conv.usage}
      <div class="acp-usage-row">
        {#if conv.session && (conv.session.models.length || conv.session.efforts.length || (!isAside && conv.session.modes.length))}
          <div class="acp-agent-opts">
            <button
              type="button"
              class="acp-opts-btn"
              title={optsTitle}
              aria-label={t('acp.chat.optsAria')}
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
                  <div class="acp-opts-head">{t('acp.chat.modelMenuHead')}</div>
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
                  <div class="acp-opts-head">{t('acp.chat.effortMenuHead')}</div>
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
                {#if !isAside && conv.session.modes.length}
                  <div class="acp-opts-head">{t('acp.chat.modeMenuHead')}</div>
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
            title={t('acp.chat.contextTitle', {
              used: conv.usage.used.toLocaleString(),
              size: conv.usage.size.toLocaleString(),
              compactUsed: compact(conv.usage.used),
              compactSize: compact(conv.usage.size),
              costPart: conv.usage.cost ? ` · ${formatCost(conv.usage.cost)}` : '',
            })}
          >
            <div class="acp-usage-bar" style="width: {usagePct}%"></div>
          </div>
          <span class="acp-usage-text" title="{compact(conv.usage.used)} / {compact(conv.usage.size)} tokens">{usagePct.toFixed(0)}%</span>
        {:else}
          <span class="acp-usage-spacer"></span>
        {/if}
        {#if quota && quotaWindow && !isAside}
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
            title={t('acp.chat.compactTitle')}
            disabled={conv.compacting || busy}
            onclick={() => conv.compact()}
          >
            <Shrink size={11} />
            {conv.compacting ? t('acp.chat.compacting') : t('acp.chat.compact')}
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
      {#if suggestions.length}
        <div class="acp-suggest" role="listbox">
          <!-- Sin clave: dos filas pueden ser iguales (bóvedas con el mismo nombre) y una clave repetida
               deja la lista sin actualizar -->
          {#each suggestions as sug, i}
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
        <div class="acp-queue" aria-label={t('acp.chat.queueAria')}>
          {#each conv.queue as q (q.id)}
            <div class="acp-queued">
              <Clock size={11} />
              <span class="acp-queued-text" title={q.text}>{q.text}</span>
              {#if q.attachments.length}<span class="acp-queued-att">+{q.attachments.length}</span>{/if}
              <button type="button" aria-label={t('acp.chat.unqueueLabel')} title={t('acp.chat.unqueueLabel')} onclick={() => conv.unqueue(q.id)}><X size={10} /></button>
            </div>
          {/each}
          <div class="acp-queue-hint">{t('acp.chat.queueHint')}</div>
        </div>
      {/if}
      {#if conv.pendingAttachments.length || minimizedAsides}
        <div class="acp-chips">
          {#each conv.pendingAttachments as a (a.kind + a.path + (a.kind === 'selection' ? a.startLine : ''))}
            <span class="acp-chip" class:image={a.kind === 'image'} title={a.kind === 'image' ? imageTitle : a.path}>
              {#if a.kind === 'image'}<button type="button" class="acp-image-btn" title={t('acp.chat.viewRawImage')} onclick={() => openImage(a)}><img src="data:{a.mimeType};base64,{a.data}" alt="" /></button><ImageIcon size={11} />{:else if a.kind === 'selection'}<TextSelect size={11} />{:else if a.name.endsWith('/')}<Folder size={11} />{:else}<FileText size={11} />{/if}
              {chipLabel(a)}
              <button type="button" aria-label={t('acp.chat.removeAttachment', { name: chipLabel(a) })} onclick={() => conv.detach(a)}><X size={10} /></button>
            </span>
          {/each}
          {#if minimizedAsides}
            {@const state = asideWorking ? t('aside.answering') : asideUnread ? t('aside.unread') : t('aside.show')}
            <button type="button" class="acp-aside-card" class:unread={asideUnread} title={state} aria-label="{t('aside.card', { count: minimizedAsides })}: {state}" onclick={openAsides}>
              {#if asideWorking}<Loader2 size={12} class="acp-spin" />{:else}<MessageCircleQuestion size={12} />{/if}
              {t('aside.card', { count: minimizedAsides })}
              {#if asideUnread && !asideWorking}<span class="acp-aside-dot" aria-hidden="true"></span>{/if}
            </button>
          {/if}
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
          placeholder={isAside
            ? t('aside.placeholder')
            : busy
            ? t('acp.chat.placeholderBusy', { agent: conv.agent?.name ?? t('acp.store.fallbackAgentName') })
            : ready
            ? t('acp.chat.placeholderReady', { agent: conv.agent?.name ?? t('acp.chat.fallbackAgentLower') })
            : t('acp.chat.placeholderDisconnected')}
          disabled={!ready}
        ></textarea>
        {#if busy}
          <button type="button" class="acp-send" title={t('acp.chat.stopTitle')} onclick={stopTurn}>
            <Square size={14} />
          </button>
        {:else}
          <button type="submit" class="acp-send" title={t('acp.chat.sendTitle')} disabled={!ready || !conv.draft.trim()}>
            <Send size={14} />
          </button>
        {/if}
      </form>
    </div>
    {#if openAside}
      <SideQuestion
        bind:this={sidePanel}
        asides={conv.asides}
        side={openAside}
        onSelect={(id) => conv.showAside(id)}
        onMinimize={() => {
          conv.minimizeAsides();
          textarea?.focus();
        }}
        onRemove={(id) => {
          void conv.removeAside(id);
          textarea?.focus();
        }}
        onPromote={(id) => void conv.promoteAside(id)}
      >
        {#key openAside.id}
          <Self {cwd} {onOpenSettings} {onOpenNote} {onOutsideFile} conv={openAside} />
        {/key}
      </SideQuestion>
    {/if}
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
  /* Último hijo del scroller: margin-top:auto lo baja al fondo y sticky lo mantiene visible al hacer scroll */
  .acp-watermark {
    position: sticky;
    bottom: 0;
    align-self: flex-end;
    margin-top: auto;
    height: 0;
    width: 0;
    overflow: visible;
    pointer-events: none;
    user-select: none;
  }
  .acp-watermark :global(.agent-icon) {
    position: absolute;
    right: 0;
    bottom: 4px;
    color: var(--text-muted);
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
  /* Preguntas aparte minimizadas: a la derecha de los adjuntos, encima del botón Enviar */
  .acp-aside-card {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-left: auto;
    padding: 3px 9px;
    border: 1px solid var(--accent);
    border-radius: 6px;
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    color: var(--accent);
    font: inherit;
    font-size: 11px;
    cursor: pointer;
  }
  .acp-aside-card:hover,
  .acp-aside-card.unread {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
  }
  .acp-aside-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }
  .acp-context {
    font-size: 12px;
    color: var(--text-muted);
    border: 1px dashed var(--border-primary);
    border-radius: 8px;
    padding: 4px 8px;
    flex-shrink: 0;
  }
  .acp-context summary {
    cursor: pointer;
  }
  .acp-context pre {
    margin: 6px 0 0;
    max-height: 240px;
    overflow: auto;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font-size: 11px;
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
  .acp-msg.agent :global(.acp-copy-target) {
    outline: 1px dashed var(--accent, #0969da);
    outline-offset: 2px;
  }
  :global(.acp-context-menu) {
    min-width: 170px;
    padding: 4px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    box-shadow: var(--popover-shadow, 0 8px 24px rgba(0, 0, 0, 0.12));
    z-index: 9999;
    outline: none;
  }
  :global(.acp-context-item) {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    border-radius: 5px;
    font-size: 12px;
    color: var(--text-primary, #1f2328);
    cursor: pointer;
    outline: none;
  }
  :global(.acp-context-item[data-highlighted]) {
    background: var(--bg-secondary, #f6f8fa);
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
