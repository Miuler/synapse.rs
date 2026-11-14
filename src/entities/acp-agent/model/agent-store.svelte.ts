import { agentApi, listenAgentEvents, listenConversationRequests } from '../api/agent.api';
import { feedbackPrompt } from './feedback-prompt';
import { replayText } from './replay-text';
import { askConfirm } from '@shared/lib/confirm';
import type {
  AgentCommand,
  AgentConnection,
  AgentProfile,
  AgentSession,
  AgentTranscriptSummary,
  Attachment,
  ChatItem,
  PlanEntry,
  PromptBlock,
} from './types';

type DistributiveOmit<T, K extends PropertyKey> = T extends unknown ? Omit<T, K> : never;

const FLOATING_KEY = 'synapse.agent.floating';
const FEEDBACK_KEY = 'synapse.agent.feedback';
/** Espera de un permiso no de lectura en una conversación entrante antes de rechazarlo. */
const INCOMING_PERMISSION_TIMEOUT = 120_000;

/** Idas y vueltas seguidas entre agentes sin intervención del usuario. */
export const MAX_AGENT_HOPS = 3;

/** Mensaje de otra conversación esperando a que esta termine su turno. */
/** Quien escribe: otra conversación de esta ventana, o de otra bóveda (`vault`). */
interface Sender {
  handle: string;
  agent: string;
  title: string;
  vault?: string;
}

/** Bóveda remota y conversación a la que vuelve una respuesta. */
interface RemoteRef {
  vault: string;
  path: string;
  convId: string;
}

/** Adónde vuelve la respuesta de una pregunta. */
type ReplyTarget = { local: Conversation } | { remote: RemoteRef };

interface InboxMessage {
  from: Sender;
  text: string;
  hops: number;
  /** Adónde vuelve la respuesta (si era una pregunta). */
  replyTo: ReplyTarget | null;
}

/** Origen de un pedido de otra instancia. */
interface PeerFrom extends Sender {
  vault: string;
  path: string;
  convId: string;
}

/** Pedido que llega de otra instancia de Synapse por su canal `/peer`. */
interface PeerRequest {
  action: 'ping' | 'list' | 'send' | 'deliver';
  from: PeerFrom;
  to: number | string | null;
  fresh?: boolean;
  message: string;
  expectsReply?: boolean;
  hops: number;
}

/** Conversaciones desplegadas a la vez en el acordeón. */
export const MAX_EXPANDED = 2;

function load(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function save(key: string, value: string | null) {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, value);
  } catch {
    /* sin almacenamiento */
  }
}

type Usage = { used: number; size: number; cost: { amount: number; currency: string } | null };

/** Una conversación con un agente: su sesión ACP, sus mensajes y lo que lleva el próximo mensaje. */
export class Conversation {
  /** Identificador local (y de su archivo en el historial). */
  readonly id: string;
  agentId: string;
  session = $state<AgentSession | null>(null);
  connection = $state<AgentConnection>('idle');
  items = $state<ChatItem[]>([]);
  plan = $state<PlanEntry[]>([]);
  commands = $state<AgentCommand[]>([]);
  usage = $state<Usage | null>(null);
  currentMode = $state<string | null>(null);
  currentModel = $state<string | null>(null);
  attachments = $state<Attachment[]>([]);
  dismissedActiveNote = $state<string | null>(null);
  /** Hay novedades que el usuario no vio (terminó o pide permiso estando plegada). */
  unseen = $state(false);
  /** El agente está compactando su contexto (`/compact`). */
  compacting = $state(false);
  /** Número corto (#1, #2…) para nombrarla desde otra conversación. */
  number = 0;
  /** Mensajes de otras conversaciones que esperan a que termine su turno. */
  inbox = $state<InboxMessage[]>([]);
  /** Conversaciones a las que preguntó y de las que espera respuesta (#2…). */
  waitingFor = $state<string[]>([]);
  /** Conversación abierta por otra bóveda de Synapse (🐙), con su origen. */
  incoming = $state<{ vault: string; handle: string; title: string } | null>(null);
  /** Saltos entre agentes del turno en curso (0 si lo empezó el usuario). */
  #turnHops = 0;

  readonly #store: AgentStore;
  readonly #cwd: string;
  #nextId = 1;
  #createdAt = Date.now();
  #acpSessionId: string | null = null;
  #replay: string | null = null;
  #saveTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(store: AgentStore, agentId: string, cwd: string, id: string = crypto.randomUUID()) {
    this.#store = store;
    this.agentId = agentId;
    this.#cwd = cwd;
    this.id = id;
  }

  get handle() {
    return `#${this.number}`;
  }

  get cwd() {
    return this.#cwd;
  }
  get agent(): AgentProfile | null {
    return this.#store.agents.find((a) => a.id === this.agentId) ?? null;
  }
  get title(): string {
    const first = this.items.find((i) => i.type === 'user');
    return (first && 'text' in first ? first.text : '').replace(/\s+/g, ' ').trim().slice(0, 80);
  }
  get pendingPermissions(): number {
    return this.items.filter((i) => i.type === 'permission' && i.answered === null).length;
  }
  /** Estado resumido para el punto de color de la fila. */
  get status(): 'working' | 'permission' | 'idle' | 'error' {
    if (this.pendingPermissions) return 'permission';
    if (this.connection === 'busy' || this.connection === 'starting' || this.connection === 'installing') return 'working';
    if (this.connection === 'error') return 'error';
    return 'idle';
  }

  push(item: DistributiveOmit<ChatItem, 'id'>) {
    this.items.push({ ...item, id: this.#nextId++ } as ChatItem);
  }

  error(e: unknown) {
    this.push({ type: 'error', text: e instanceof Error ? e.message : String(e) });
  }

  /** Guarda la conversación en la bóveda (agrupa cambios seguidos). */
  persist(now = false) {
    if (this.#saveTimer) clearTimeout(this.#saveTimer);
    this.#saveTimer = null;
    const run = () => {
      const agent = this.agent;
      if (!agent || !this.items.some((i) => i.type === 'user')) return;
      agentApi
        .saveTranscript({
          id: this.id,
          agent_id: agent.id,
          agent_name: agent.name,
          cwd: this.#cwd,
          title: this.title,
          acp_session_id: this.session?.id ?? this.#acpSessionId,
          created_at: this.#createdAt,
          updated_at: Date.now(),
          items: $state.snapshot(this.items) as ChatItem[],
        })
        .then(() => this.#store.loadHistory(this.#cwd))
        .catch(() => {});
    };
    if (now) run();
    else this.#saveTimer = setTimeout(run, 800);
  }

  /** Carga una conversación guardada (sus mensajes) antes de conectar. */
  async restore(transcriptId: string): Promise<string | null> {
    const t = await agentApi.loadTranscript(this.#cwd, transcriptId);
    if (!this.#store.agents.some((a) => a.id === t.agent_id)) {
      throw new Error(`El agente «${t.agent_name || t.agent_id}» de esta conversación ya no está configurado.`);
    }
    this.agentId = t.agent_id;
    this.items = Array.isArray(t.items) ? t.items : [];
    this.#nextId = this.items.reduce((m, i) => Math.max(m, i.id), 0) + 1;
    this.#createdAt = t.created_at;
    this.#acpSessionId = t.acp_session_id;
    return t.acp_session_id;
  }

  /** Lanza el agente (instalándolo si hace falta) y abre o retoma la sesión. */
  async connect(resume: string | null = null) {
    const agent = this.agent;
    if (!agent || this.session) return;
    try {
      if (!agent.installed) {
        this.connection = 'installing';
        await agentApi.install(agent.id);
        await this.#store.refresh();
      }
      this.connection = 'starting';
      this.session = await agentApi.startSession(agent.id, this.#cwd, this.#store.feedbackMode, resume, this.id);
      if (resume && !this.session.resumed && this.items.length) this.#replay = replayText(this.items);
      this.currentMode = this.session.current_mode;
      this.currentModel = this.session.current_model;
      this.commands = this.#store.takeEarlyCommands(this.session.id);
      this.connection = 'ready';
      // Modelo recordado para este agente: no hay que elegirlo en cada conversación nueva
      const savedModel = load(`synapse.agent.model.${agent.id}`);
      if (savedModel && savedModel !== this.currentModel && this.session.models.some((m) => m.id === savedModel)) {
        await this.setModel(savedModel);
      }
      if (resume) this.persist(true);
      this.deliverNext();
    } catch (e) {
      this.connection = 'error';
      this.error(e);
    }
  }

  /** Deja un mensaje de otra conversación; entra ya si está libre, o al terminar su turno. */
  receive(msg: InboxMessage): 'delivered' | 'queued' {
    this.inbox.push(msg);
    const now = this.connection === 'ready';
    this.deliverNext();
    return now ? 'delivered' : 'queued';
  }

  /** Entrega el siguiente mensaje del buzón si la conversación está libre. */
  deliverNext() {
    if (this.connection !== 'ready' || !this.inbox.length) return;
    const msg = this.inbox.shift()!;
    const from = msg.from;
    const origin = from.vault ? `otra bóveda de Synapse («${from.vault}»)` : 'otra conversación de agente';
    const prompt =
      `[Mensaje de ${origin}: ${from.agent} (${from.handle}${from.title ? ` · ${from.title}` : ''})]\n` +
      (msg.replyTo
        ? 'Te hace una pregunta; tu respuesta se le enviará automáticamente, así que respóndela directamente.\n\n'
        : 'Es un aviso; no hace falta que le respondas.\n\n') +
      msg.text;
    void this.send(msg.text, { prompt, from, hops: msg.hops, replyTo: msg.replyTo });
  }

  /** Etiquetas que viajarán con el próximo mensaje (incluida la nota activa si no se quitó). */
  get pendingAttachments(): Attachment[] {
    const note = this.#store.activeNote;
    const auto =
      note && note.path !== this.dismissedActiveNote && !this.attachments.some((a) => a.kind === 'file' && a.path === note.path)
        ? [{ kind: 'file' as const, path: note.path, name: note.name }]
        : [];
    return [...auto, ...this.attachments];
  }

  attach(a: Attachment) {
    if (a.kind === 'file' && this.attachments.some((x) => x.kind === 'file' && x.path === a.path)) return;
    this.attachments.push(a);
  }

  /** Adjunta el archivo de la pestaña visible (`@this`), aunque se hubiera quitado antes. */
  attachActiveNote() {
    const note = this.#store.activeNote;
    if (!note) return;
    if (this.dismissedActiveNote === note.path) this.dismissedActiveNote = null;
    this.attach({ kind: 'file', path: note.path, name: note.name });
  }

  detach(a: Attachment) {
    if (a.kind === 'file' && this.#store.activeNote?.path === a.path && !this.attachments.includes(a)) {
      this.dismissedActiveNote = a.path;
      return;
    }
    this.attachments = this.attachments.filter((x) => x !== a);
  }

  async send(
    text: string,
    opts: {
      prompt?: string;
      from?: Sender;
      hops?: number;
      replyTo?: ReplyTarget | null;
    } = {},
  ) {
    const session = this.session;
    if (!session || this.connection === 'busy' || !text.trim()) return;
    this.#turnHops = opts.hops ?? 0;
    const turnStart = this.items.length;
    if (/(^|\s)@this\b/.test(text)) this.attachActiveNote();
    // `/feedback [indicaciones]`: lo resuelve Synapse, no el agente
    const feedback = this.#store.feedbackAvailable ? /^\/feedback(?:\s+([\s\S]*))?$/.exec(text.trim()) : null;
    const attachments = this.pendingAttachments;
    const blocks: PromptBlock[] = [];
    if (this.#replay) {
      blocks.push({ type: 'text', text: this.#replay });
      this.#replay = null;
    }
    blocks.push({ type: 'text', text: opts.prompt ?? (feedback ? feedbackPrompt(feedback[1]?.trim() ?? '') : text) });
    for (const a of attachments) {
      blocks.push(
        a.kind === 'file'
          ? { type: 'file', path: a.path, name: a.name }
          : a.kind === 'image'
            ? { type: 'image', mime_type: a.mimeType, data: a.data }
            : { type: 'selection', path: a.path, text: a.text, start_line: a.startLine, end_line: a.endLine },
      );
    }
    // `/compact`: el agente resume su contexto; la marca separa lo que ya no recuerda en detalle
    const compact = /^\/compact\b/.test(text.trim());
    if (compact) {
      this.push({ type: 'compact' });
      this.compacting = true;
    }
    this.push(opts.from ? { type: 'user', text, attachments, from: opts.from } : { type: 'user', text, attachments });
    this.attachments = [];
    // Tras enviarla, la nota activa no se repite hasta que cambie
    const note = this.#store.activeNote;
    if (note) this.dismissedActiveNote = note.path;
    this.plan = [];
    this.connection = 'busy';
    this.persist(true);
    try {
      const stop = await agentApi.sendPrompt(session.id, blocks);
      if (stop === 'refusal') this.push({ type: 'error', text: 'El agente rechazó la petición.' });
    } catch (e) {
      this.error(e);
    } finally {
      if (this.session?.id === session.id) this.connection = 'ready';
      if (compact) this.compacting = false;
      if (!this.#store.isVisible(this)) this.unseen = true;
      this.persist(true);
      // Era una pregunta de otra conversación: su respuesta vuelve sola
      if (opts.replyTo) {
        const answer = this.items
          .slice(turnStart)
          .filter((i) => i.type === 'agent')
          .map((i) => ('text' in i ? i.text : ''))
          .join('\n\n')
          .trim();
        const target = opts.replyTo;
        const text = answer || '(no respondió nada)';
        const hops = (opts.hops ?? 0) + 1;
        if ('local' in target) {
          const asker = target.local;
          asker.waitingFor = asker.waitingFor.filter((h) => h !== this.handle);
          if (this.#store.byId(asker.id)) asker.receive({ from: this.#store.senderOf(this), text, hops, replyTo: null });
        } else {
          void this.#store.replyRemote(target.remote, this, text, hops);
        }
      }
      this.#turnHops = 0;
      this.deliverNext();
    }
  }

  /** Saltos entre agentes que lleva el turno en curso. */
  get turnHops() {
    return this.#turnHops;
  }

  async setMode(modeId: string) {
    if (!this.session) return;
    const previous = this.currentMode;
    this.currentMode = modeId;
    try {
      await agentApi.setMode(this.session.id, modeId);
    } catch (e) {
      this.currentMode = previous;
      this.error(e);
    }
  }

  async setModel(modelId: string) {
    if (!this.session) return;
    save(`synapse.agent.model.${this.session.agent_id}`, modelId);
    const previous = this.currentModel;
    this.currentModel = modelId;
    try {
      await agentApi.setModel(this.session.id, modelId);
    } catch (e) {
      this.currentModel = previous;
      this.error(e);
    }
  }

  async cancel() {
    if (this.session) await agentApi.cancelPrompt(this.session.id).catch((e) => this.error(e));
  }

  async answer(itemId: number, optionId: string | null) {
    const item = this.items.find((i) => i.id === itemId);
    if (!item || item.type !== 'permission' || item.answered !== null) return;
    item.answered = optionId ?? '';
    this.persist();
    try {
      await agentApi.respondPermission(item.request.sessionId, item.request.requestId, optionId);
    } catch (e) {
      this.error(e);
    }
  }

  /** Si el agente ofrece `/compact` (lo anuncia entre sus comandos). */
  get canCompact(): boolean {
    return this.commands.some((c) => c.name === 'compact');
  }

  /** Compacta el contexto del agente con su comando `/compact`. */
  compact() {
    return this.send('/compact');
  }

  /** Abre el archivo crudo (JSON) de la conversación con la aplicación por defecto. */
  async openRaw() {
    this.persist(true);
    try {
      await new Promise((r) => setTimeout(r, 150));
      await agentApi.openTranscript(this.#cwd, this.id);
    } catch (e) {
      this.error(e);
    }
  }

  /** Cierra la sesión del agente; la conversación queda guardada en el historial. */
  async stop() {
    const session = this.session;
    if (session) {
      this.#acpSessionId = session.id;
      this.persist(true);
    }
    this.session = null;
    this.connection = 'idle';
    if (session) await agentApi.stopSession(session.id).catch(() => {});
  }

  /** Eventos del agente para esta conversación. */
  onStream(kind: string, text: string) {
    if (kind === 'user') return;
    const type = kind === 'thought' ? 'thought' : 'agent';
    const last = this.items.at(-1);
    if (last && last.type === type) last.text += text;
    else this.push({ type, text });
  }
}

/** Conversaciones abiertas con los agentes, en un acordeón (como mucho {@link MAX_EXPANDED} desplegadas) y una fijada al costado. */
class AgentStore {
  agents = $state<AgentProfile[]>([]);
  listError = $state<string | null>(null);
  floating = $state(load(FLOATING_KEY) === '1');
  /** Modo feedback: el agente anota lo que le falta al MCP de Synapse (aplica a nuevas sesiones). */
  feedbackMode = $state(load(FEEDBACK_KEY) === '1');
  /** Esta compilación trae el feedback del MCP (feature `develop`): habilita `/feedback`. */
  feedbackAvailable = $state(false);
  /** Nota activa del editor; se adjunta sola salvo que el usuario la quite. */
  activeNote = $state<{ path: string; name: string } | null>(null);
  history = $state<AgentTranscriptSummary[]>([]);
  /** Conversaciones abiertas, la más reciente primero. */
  conversations = $state<Conversation[]>([]);
  /** Desplegadas en el acordeón, la última abierta al final. */
  expanded = $state<string[]>([]);
  /** Conversación fijada al costado del panel. */
  pinned = $state<string | null>(null);
  /** Último agente usado, para «＋ Nueva conversación». */
  lastAgentId = $state<string | null>(null);
  /** Agente que atiende las conversaciones que llegan de otra bóveda (por bóveda; `null` = el último usado). */
  incomingAgentId = $state<string | null>(null);
  /** Panel visible; se recuerda por bóveda junto con sus conversaciones. */
  panelOpen = $state(false);
  /** Estado guardado de la bóveda, pendiente de restaurar cuando se abra el panel. */
  #toRestore: VaultAgentState | null = null;

  #cwd: string | null = null;
  #nextNumber = 1;
  /** Bóveda de esta instancia, para presentarse ante otras. */
  #identity: { vault: string; path: string } | null = null;
  #early = new Map<string, AgentCommand[]>();
  #listening: Promise<unknown> | null = null;

  pinnedConversation = $derived(this.conversations.find((c) => c.id === this.pinned) ?? null);
  pendingPermissions = $derived(this.conversations.reduce((n, c) => n + c.pendingPermissions, 0));
  working = $derived(this.conversations.filter((c) => c.status === 'working').length);

  byId(id: string) {
    return this.conversations.find((c) => c.id === id) ?? null;
  }

  #bySession(sessionId: string) {
    return this.conversations.find((c) => c.session?.id === sessionId) ?? null;
  }

  isVisible(c: Conversation) {
    return this.pinned === c.id || this.expanded.includes(c.id);
  }

  takeEarlyCommands(sessionId: string): AgentCommand[] {
    const commands = this.#early.get(sessionId) ?? [];
    this.#early.delete(sessionId);
    return commands;
  }

  #listen() {
    this.#listening ??= (listenConversationRequests((req) => this.handleConversationRequest(req)), listenAgentEvents({
      onPlan: (e) => {
        const c = this.#bySession(e.sessionId);
        if (c) c.plan = e.entries;
      },
      onCommands: (e) => {
        // Puede llegar antes de que termine `startSession`: se guarda hasta conocer la sesión
        const c = this.#bySession(e.sessionId);
        if (c) c.commands = e.commands;
        else this.#early.set(e.sessionId, e.commands);
      },
      onUsage: (e) => {
        const c = this.#bySession(e.sessionId);
        if (c && e.used != null && e.size) c.usage = { used: e.used, size: e.size, cost: e.cost ?? null };
      },
      onMode: (e) => {
        const c = this.#bySession(e.sessionId);
        if (c) c.currentMode = e.modeId;
      },
      onStream: (e) => this.#bySession(e.sessionId)?.onStream(e.kind, e.text),
      onToolCall: (e) => {
        const c = this.#bySession(e.sessionId);
        if (!c) return;
        const existing = c.items.find((i) => i.type === 'tool' && i.toolCallId === e.toolCallId);
        if (existing && existing.type === 'tool') {
          if (e.title) existing.title = e.title;
          if (e.kind) existing.kind = e.kind;
          if (e.status) existing.status = e.status;
          if (e.locations?.length) existing.locations = e.locations;
          if (e.content?.length) existing.content = e.content;
          if (e.rawInput != null) existing.rawInput = e.rawInput;
          if (e.rawOutput != null) existing.rawOutput = e.rawOutput;
        } else {
          c.push({
            type: 'tool',
            toolCallId: e.toolCallId,
            title: e.title ?? 'Herramienta',
            kind: e.kind ?? null,
            status: e.status ?? null,
            locations: e.locations ?? [],
            content: e.content ?? [],
            rawInput: e.rawInput ?? undefined,
            rawOutput: e.rawOutput ?? undefined,
          });
        }
      },
      onPermission: (e) => {
        const c = this.#bySession(e.sessionId);
        if (!c) return;
        c.push({ type: 'permission', request: e, answered: null });
        const item = c.items.at(-1)!;
        if (!this.isVisible(c)) c.unseen = true;
        if (c.incoming) this.#incomingPermission(c, item.id, e);
      },
      onStatus: (e) => {
        const c = this.#bySession(e.sessionId);
        if (!c) return;
        c.session = null;
        c.connection = 'idle';
        c.push({ type: 'error', text: 'El agente terminó.' });
      },
    }));
    return this.#listening;
  }

  /** Guarda el estado del panel de la bóveda actual (solo si esa bóveda ya usó agentes o abrió el panel). */
  #saveVaultState() {
    const cwd = this.#cwd;
    if (!cwd || this.#toRestore) return;
    const state: VaultAgentState = {
      open: this.panelOpen,
      lastAgentId: this.lastAgentId,
      conversations: this.conversations.map((c) => c.id),
      expanded: this.expanded,
      pinned: this.pinned,
    };
    save(vaultKey(cwd), JSON.stringify(state));
  }

  setIncomingAgent(agentId: string | null) {
    this.incomingAgentId = agentId;
    if (this.#cwd) save(`synapse.agent.incoming.${this.#cwd}`, agentId);
  }

  /** Empieza a atender los eventos de los agentes y los pedidos del MCP y de otras bóvedas. */
  startListening() {
    return this.#listen();
  }

  setPanelOpen(open: boolean) {
    this.panelOpen = open;
    this.#saveVaultState();
  }

  /**
   * Restaura las conversaciones que la bóveda tenía abiertas (y su agente). Si no queda ninguna
   * guardada pero sí un último agente, abre una conversación nueva con él.
   */
  async restoreVaultState() {
    const state = this.#toRestore;
    const cwd = this.#cwd;
    if (!state || !cwd || this.conversations.length) {
      this.#toRestore = null;
      return;
    }
    if (!this.agents.length) await this.refresh();
    await this.loadHistory(cwd);
    const saved = state.conversations.filter((id) => this.history.some((h) => h.id === id));
    try {
      for (const id of [...saved].reverse()) await this.openFromHistory(cwd, id);
      if (!saved.length && state.lastAgentId && this.agents.some((a) => a.id === state.lastAgentId)) {
        await this.newConversation(cwd, state.lastAgentId);
      }
      // Lo desplegado al cerrar la bóveda, sin replegar lo que el usuario abrió mientras se restauraba
      const opened = this.expanded.filter((id) => !saved.includes(id));
      this.expanded = [...state.expanded.filter((id) => saved.includes(id) && this.byId(id)), ...opened].slice(-MAX_EXPANDED);
      if (state.pinned && this.byId(state.pinned)) {
        this.pinned = state.pinned;
        this.collapse(state.pinned);
      }
      if (state.lastAgentId) this.lastAgentId = state.lastAgentId;
    } finally {
      this.#toRestore = null;
      this.#saveVaultState();
    }
  }

  /** Cómo se presenta una conversación de esta ventana ante otra. */
  senderOf(c: Conversation): Sender {
    return { handle: c.handle, agent: c.agent?.name ?? c.agentId, title: c.title };
  }

  async #me() {
    this.#identity ??= await agentApi.peerIdentity().catch(() => null);
    return this.#identity;
  }

  /** Peticiones del MCP (`list_conversations`, `send_to_conversation`) y de otras instancias (`peer`). */
  async handleConversationRequest(req: {
    action: string;
    caller: string | null;
    to?: string;
    message?: string;
    expectsReply?: boolean;
    request?: unknown;
  }): Promise<unknown> {
    if (req.action === 'peer') return this.#handlePeer(req.request as PeerRequest);
    const caller = req.caller ? this.byId(req.caller) : null;
    if (req.action === 'list') {
      const local = this.#localSummaries(caller);
      // Las conversaciones de las otras bóvedas abiertas, con el nombre de su bóveda delante
      const peers = await agentApi.peerList({ action: 'list' }).catch(() => []);
      const remote = peers.flatMap((p) =>
        Array.isArray(p.result)
          ? (p.result as { handle: string }[]).map((c) => ({ ...c, handle: `${p.vault}${c.handle}`, vault: p.vault, window: 'otra ventana' }))
          : [],
      );
      const me = await this.#me();
      return [
        ...local.map((c) => ({ ...c, vault: me?.vault ?? null, window: 'esta ventana' })),
        ...remote,
        ...peers
          .filter((p) => !Array.isArray(p.result) || !(p.result as unknown[]).length)
          .map((p) => ({ handle: `${p.vault}#nueva`, vault: p.vault, window: 'otra ventana', note: 'Bóveda abierta sin conversaciones: escribirle abre una nueva' })),
      ];
    }
    if (req.action !== 'send') throw new Error(`Acción desconocida: ${req.action}`);
    if (!caller) throw new Error('No se sabe desde qué conversación escribes; usa el MCP de tu conversación.');
    const to = (req.to ?? '').trim();
    const hops = caller.turnHops + 1;
    if (hops > MAX_AGENT_HOPS) {
      throw new Error(
        `Límite de ${MAX_AGENT_HOPS} mensajes seguidos entre agentes sin intervención del usuario. Resume lo que tengas y deja que el usuario decida.`,
      );
    }
    const expectsReply = !!req.expectsReply;
    // «mainframe», «mainframe#2» o «mainframe#nueva»: otra bóveda
    const remote = await this.#remoteTarget(to);
    if (remote) return this.#sendRemote(caller, remote, req.message ?? '', expectsReply, hops);
    const target = this.#resolveTarget(to, caller);
    if (expectsReply && !caller.waitingFor.includes(target.handle)) caller.waitingFor = [...caller.waitingFor, target.handle];
    const status = target.receive({
      from: this.senderOf(caller),
      text: req.message ?? '',
      hops,
      replyTo: expectsReply ? { local: caller } : null,
    });
    return {
      status,
      to: `${target.handle} · ${target.agent?.name ?? target.agentId}`,
      note:
        (status === 'queued' ? 'Está trabajando: lo recibirá al terminar su turno. ' : 'Entregado. ') +
        (expectsReply ? 'Su respuesta te llegará como un mensaje nuevo en esta conversación; no la esperes.' : ''),
    };
  }

  #localSummaries(caller: Conversation | null) {
    return this.conversations.map((c) => ({
      handle: c.handle,
      agent: c.agent?.name ?? c.agentId,
      title: c.title || 'Conversación nueva',
      status: c.status,
      pending_messages: c.inbox.length,
      is_you: c.id === caller?.id,
      incoming_from: c.incoming ? `${c.incoming.vault}${c.incoming.handle}` : undefined,
    }));
  }

  /** `to` apunta a otra bóveda abierta: su nombre, con `#n` o `#nueva` opcional. */
  async #remoteTarget(to: string): Promise<{ vault: string; number: number | null; fresh: boolean } | null> {
    const m = /^(.*?)#(\d+|nueva)?$/i.exec(to) ?? [to, to, undefined];
    const vault = (m[1] ?? '').trim();
    if (!vault) return null;
    const me = await this.#me();
    if (me && vault.toLowerCase() === me.vault.toLowerCase()) return null;
    const peers = await agentApi.peerList({ action: 'ping' }).catch(() => []);
    const peer = peers.find((p) => p.vault.toLowerCase() === vault.toLowerCase() || p.path === vault);
    if (!peer) {
      // Sin «#» puede ser el título de una conversación local; con «#» es una bóveda que no está abierta
      if (to.includes('#')) throw new Error(`La bóveda «${vault}» no está abierta en otra ventana de Synapse.`);
      return null;
    }
    const n = m[2];
    return { vault: peer.path, number: n && /^\d+$/.test(n) ? Number(n) : null, fresh: /^nueva$/i.test(n ?? '') };
  }

  async #sendRemote(
    caller: Conversation,
    target: { vault: string; number: number | null; fresh: boolean },
    message: string,
    expectsReply: boolean,
    hops: number,
  ) {
    const me = await this.#me();
    if (!me) throw new Error('Esta ventana no tiene una bóveda abierta.');
    const result = (await agentApi.peerSend(target.vault, {
      action: 'send',
      from: { ...this.senderOf(caller), vault: me.vault, path: me.path, convId: caller.id },
      to: target.number,
      fresh: target.fresh,
      message,
      expectsReply,
      hops,
    })) as { status: string; to: string };
    if (expectsReply && !caller.waitingFor.includes(result.to)) caller.waitingFor = [...caller.waitingFor, result.to];
    return {
      ...result,
      note:
        `Enviado a la bóveda de otra ventana (${result.to}). ` +
        (expectsReply ? 'Su respuesta te llegará como un mensaje nuevo en esta conversación; no la esperes.' : ''),
    };
  }

  /** Devuelve la respuesta a la conversación de otra bóveda que preguntó. */
  async replyRemote(remote: RemoteRef, from: Conversation, text: string, hops: number) {
    const me = await this.#me();
    try {
      await agentApi.peerSend(remote.path, {
        action: 'deliver',
        to: remote.convId,
        from: { ...this.senderOf(from), vault: me?.vault ?? '', path: me?.path ?? '' },
        message: text,
        hops,
      });
    } catch (e) {
      from.error(`No se pudo devolver la respuesta a «${remote.vault}»: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  /** Pedidos de otra instancia: listar, recibir un mensaje (en una conversación entrante) o una respuesta. */
  async #handlePeer(req: PeerRequest): Promise<unknown> {
    if (!req || req.action === 'ping') return [];
    if (req.action === 'list') return this.#localSummaries(null);
    const cwd = this.#cwd;
    if (!cwd) throw new Error('Esa ventana de Synapse no tiene una bóveda abierta.');
    const from = req.from;
    if (req.hops > MAX_AGENT_HOPS) throw new Error(`Límite de ${MAX_AGENT_HOPS} mensajes seguidos entre agentes.`);
    const sender: Sender = { handle: `${from.vault}${from.handle}`, agent: from.agent, title: from.title, vault: from.vault };
    if (req.action === 'deliver') {
      const c = this.byId(req.to as string);
      if (!c) throw new Error('La conversación que preguntó ya no está abierta.');
      c.waitingFor = c.waitingFor.filter((h) => !h.startsWith(from.vault));
      c.receive({ from: sender, text: req.message, hops: req.hops, replyTo: null });
      return { status: 'delivered' };
    }
    if (req.action !== 'send') throw new Error(`Pedido desconocido: ${req.action}`);
    if (!(await this.#acceptsFrom(cwd, from))) throw new Error(`La bóveda «${(await this.#me())?.vault}» no aceptó la conversación.`);
    const c = await this.#incomingConversation(cwd, req);
    const me = await this.#me();
    const status = c.receive({
      from: sender,
      text: req.message,
      hops: req.hops,
      replyTo: req.expectsReply ? { remote: { vault: from.vault, path: from.path, convId: from.convId } } : null,
    });
    return { status, to: `${me?.vault ?? ''}${c.handle}` };
  }

  /**
   * Permisos en una conversación abierta por otra bóveda, donde quizá nadie está mirando: las
   * lecturas se permiten solas; lo demás espera al usuario y, sin respuesta, se rechaza.
   */
  #incomingPermission(c: Conversation, itemId: number, e: { toolCallId?: string | null; options: { id: string; kind: string }[] }) {
    const tool = c.items.find((i) => i.type === 'tool' && i.toolCallId === e.toolCallId);
    const kind = tool && tool.type === 'tool' ? (tool.kind ?? '') : '';
    const allow = e.options.find((o) => o.kind === 'allow_once') ?? e.options.find((o) => o.kind.startsWith('allow'));
    const reject = e.options.find((o) => o.kind === 'reject_once') ?? e.options.find((o) => o.kind.startsWith('reject'));
    if (['read', 'search', 'fetch', 'think'].includes(kind) && allow) {
      void c.answer(itemId, allow.id);
      return;
    }
    setTimeout(() => {
      const item = c.items.find((i) => i.id === itemId);
      if (item && item.type === 'permission' && item.answered === null) {
        c.push({ type: 'error', text: 'Nadie respondió el permiso en 2 minutos: se rechazó.' });
        void c.answer(itemId, reject?.id ?? null);
      }
    }, INCOMING_PERMISSION_TIMEOUT);
  }

  /** Permiso de esta bóveda para conversar con `from` (se pregunta la primera vez y se recuerda). */
  async #acceptsFrom(cwd: string, from: PeerFrom): Promise<boolean> {
    const key = `synapse.agent.accept.${cwd}`;
    const saved = JSON.parse(load(key) ?? '{}') as Record<string, boolean>;
    if (from.path in saved) return saved[from.path];
    const agentId = this.incomingAgentId ?? this.lastAgentId;
    const agent = agentId ? (this.agents.find((a) => a.id === agentId)?.name ?? '') : '';
    const allowed = await askConfirm(
      '🐙 Conversación desde otra bóveda',
      `Un agente de «${from.vault}» quiere conversar con un agente de esta bóveda` +
        (agent ? ` (se usará ${agent})` : '') +
        '. Si no hay una conversación abierta para él, se abrirá una nueva. ¿Permitir?',
    );
    saved[from.path] = allowed;
    save(key, JSON.stringify(saved));
    return allowed;
  }

  /** Conversación que atiende a `from`: la pedida, la de su hilo, o una nueva y limpia. */
  async #incomingConversation(cwd: string, req: PeerRequest): Promise<Conversation> {
    const from = req.from;
    if (typeof req.to === 'number') {
      const c = this.conversations.find((x) => x.number === req.to);
      if (!c) throw new Error(`No hay una conversación #${req.to} en esa bóveda.`);
      return c;
    }
    const threadsKey = `synapse.agent.threads.${cwd}`;
    const threads = JSON.parse(load(threadsKey) ?? '{}') as Record<string, string>;
    const thread = `${from.path}|${from.convId}`;
    const known = req.fresh ? null : threads[thread];
    if (known) {
      const open = this.byId(known);
      if (open) return open;
      if (this.history.some((h) => h.id === known)) {
        const c = await this.openFromHistory(cwd, known);
        c.incoming = { vault: from.vault, handle: from.handle, title: from.title };
        return c;
      }
    }
    if (!this.agents.length) await this.refresh();
    const configured = this.incomingAgentId && this.agents.some((a) => a.id === this.incomingAgentId) ? this.incomingAgentId : null;
    const agentId = configured ?? this.lastAgentId ?? this.agents.find((a) => a.installed)?.id;
    if (!agentId) throw new Error('Esa bóveda no tiene un agente configurado para atender conversaciones.');
    const c = new Conversation(this, agentId, cwd);
    c.incoming = { vault: from.vault, handle: from.handle, title: from.title };
    this.#add(c);
    this.panelOpen = true;
    await c.connect();
    threads[thread] = c.id;
    save(threadsKey, JSON.stringify(threads));
    this.loadHistory(cwd);
    return c;
  }

  /** Destino por número (#2), id, o parte de su título o del nombre del agente. */
  #resolveTarget(to: string, caller: Conversation): Conversation {
    const q = to.trim().toLowerCase();
    const others = this.conversations.filter((c) => c.id !== caller.id);
    const byNumber = /^#?(\d+)$/.exec(q);
    const exact = byNumber ? others.find((c) => c.number === Number(byNumber[1])) : others.find((c) => c.id === to);
    if (exact) return exact;
    const matches = others.filter((c) => `${c.agent?.name ?? c.agentId} ${c.title}`.toLowerCase().includes(q));
    if (matches.length === 1) return matches[0];
    const list = others.map((c) => `${c.handle} ${c.agent?.name ?? c.agentId} · ${c.title || 'nueva'}`).join('; ');
    throw new Error(
      matches.length
        ? `«${to}» coincide con varias conversaciones (${matches.map((c) => c.handle).join(', ')}); usa su número. Abiertas: ${list}`
        : `No hay otra conversación «${to}». Abiertas: ${list || 'ninguna'}`,
    );
  }

  async refresh() {
    this.listError = null;
    try {
      this.agents = await agentApi.list();
      this.feedbackAvailable = await agentApi.feedbackAvailable().catch(() => false);
    } catch (e) {
      this.listError = String(e);
    }
  }

  setFeedbackMode(value: boolean) {
    this.feedbackMode = value;
    save(FEEDBACK_KEY, value ? '1' : null);
  }

  setFloating(value: boolean) {
    this.floating = value;
    save(FLOATING_KEY, value ? '1' : null);
  }

  setActiveNote(note: { path: string; name: string } | null) {
    if (note?.path === this.activeNote?.path) return;
    this.activeNote = note;
  }

  /** Despliega una conversación; si ya hay {@link MAX_EXPANDED}, se pliega la que lleva más tiempo abierta. */
  expand(id: string) {
    if (this.pinned === id) return;
    const rest = this.expanded.filter((x) => x !== id);
    while (rest.length >= MAX_EXPANDED) rest.shift();
    this.expanded = [...rest, id];
    const c = this.byId(id);
    if (c) c.unseen = false;
    this.#saveVaultState();
  }

  collapse(id: string) {
    this.expanded = this.expanded.filter((x) => x !== id);
    this.#saveVaultState();
  }

  toggle(id: string) {
    if (this.expanded.includes(id)) this.collapse(id);
    else this.expand(id);
  }

  /** Fija la conversación al costado del panel (solo una); volver a fijarla la devuelve al acordeón. */
  togglePin(id: string) {
    if (this.pinned === id) {
      this.pinned = null;
      this.expand(id);
      return;
    }
    this.pinned = id;
    this.collapse(id);
    this.#saveVaultState();
    const c = this.byId(id);
    if (c) c.unseen = false;
  }

  /** Conversación a la que van los adjuntos externos: la última desplegada, o la fijada. */
  get focused(): Conversation | null {
    const id = this.expanded.at(-1) ?? this.pinned;
    return (id && this.byId(id)) || this.conversations[0] || null;
  }

  /** Adjuntos que llegan sin conversación abierta: viajan con la próxima que se abra. */
  #waiting: Attachment[] = [];

  /** Adjunta (por ejemplo, la selección del editor) a la conversación enfocada. */
  attach(a: Attachment) {
    const c = this.focused;
    if (c) c.attach(a);
    else this.#waiting.push(a);
  }

  #add(c: Conversation) {
    c.number = this.#nextNumber++;
    for (const a of this.#waiting.splice(0)) c.attach(a);
    this.conversations = [c, ...this.conversations];
    this.expand(c.id);
  }

  /** Abre una conversación nueva con `agentId` y la despliega arriba del acordeón. */
  async newConversation(cwd: string, agentId: string) {
    await this.#listen();
    this.#cwd = cwd;
    this.lastAgentId = agentId;
    const c = new Conversation(this, agentId, cwd);
    this.#add(c);
    await c.connect();
    this.loadHistory(cwd);
    return c;
  }

  /** Abre una conversación del historial (o enfoca la que ya esté abierta) y retoma su sesión. */
  async openFromHistory(cwd: string, transcriptId: string) {
    const open = this.byId(transcriptId);
    if (open) {
      if (this.pinned !== open.id) this.expand(open.id);
      return open;
    }
    await this.#listen();
    this.#cwd = cwd;
    const summary = this.history.find((h) => h.id === transcriptId);
    const c = new Conversation(this, summary?.agent_id ?? '', cwd, transcriptId);
    this.#add(c);
    try {
      const resume = await c.restore(transcriptId);
      this.lastAgentId = c.agentId;
      await c.connect(resume);
    } catch (e) {
      c.error(e);
    }
    return c;
  }

  /** Cierra la conversación (la sesión termina) y pasa al historial. */
  async close(id: string) {
    const c = this.byId(id);
    if (!c) return;
    this.conversations = this.conversations.filter((x) => x.id !== id);
    this.collapse(id);
    if (this.pinned === id) this.pinned = null;
    this.#saveVaultState();
    await c.stop();
    if (this.#cwd) this.loadHistory(this.#cwd);
  }

  async loadHistory(cwd: string) {
    try {
      this.history = await agentApi.listTranscripts(cwd);
    } catch {
      this.history = [];
    }
  }

  /** Conversaciones guardadas que no están abiertas. */
  get closedHistory(): AgentTranscriptSummary[] {
    return this.history.filter((h) => !this.byId(h.id));
  }

  async deleteTranscript(cwd: string, id: string) {
    await this.close(id);
    await agentApi.deleteTranscript(cwd, id).catch(() => {});
    await this.loadHistory(cwd);
  }

  /** Borra las conversaciones guardadas que no están abiertas. */
  async deleteAllTranscripts(cwd: string) {
    for (const h of this.closedHistory) await agentApi.deleteTranscript(cwd, h.id).catch(() => {});
    await this.loadHistory(cwd);
  }

  /**
   * La ventana abrió otra bóveda: se cierran las conversaciones (quedan guardadas en la bóveda
   * anterior) y el panel vuelve a empezar, con el historial de la nueva bóveda.
   */
  async changeVault(cwd: string | null) {
    if (this.#cwd === cwd) return;
    const open = this.conversations;
    this.conversations = [];
    this.expanded = [];
    this.pinned = null;
    this.activeNote = null;
    this.history = [];
    this.lastAgentId = null;
    this.#cwd = cwd;
    // Lo que la bóveda tenía: panel abierto o cerrado, último agente y conversaciones abiertas.
    // Una bóveda que nunca usó agentes empieza con el panel cerrado y sin agente.
    const saved = cwd ? loadVaultState(cwd) : null;
    this.#toRestore = saved;
    this.panelOpen = saved?.open ?? false;
    this.incomingAgentId = cwd ? load(`synapse.agent.incoming.${cwd}`) : null;
    this.lastAgentId = saved?.lastAgentId ?? null;
    await Promise.all(open.map((c) => c.stop()));
    if (cwd) await this.loadHistory(cwd);
    if (saved?.open) await this.restoreVaultState();
  }
}


/** Estado del panel de agentes guardado por bóveda. */
interface VaultAgentState {
  open: boolean;
  lastAgentId: string | null;
  conversations: string[];
  expanded: string[];
  pinned: string | null;
}

const vaultKey = (cwd: string) => `synapse.agent.vault.${cwd}`;

function loadVaultState(cwd: string): VaultAgentState | null {
  try {
    const raw = load(vaultKey(cwd));
    if (!raw) return null;
    const s = JSON.parse(raw) as Partial<VaultAgentState>;
    return {
      open: !!s.open,
      lastAgentId: s.lastAgentId ?? null,
      conversations: Array.isArray(s.conversations) ? s.conversations : [],
      expanded: Array.isArray(s.expanded) ? s.expanded : [],
      pinned: s.pinned ?? null,
    };
  } catch {
    return null;
  }
}

export type { AgentStore };
export const agentStore = new AgentStore();
