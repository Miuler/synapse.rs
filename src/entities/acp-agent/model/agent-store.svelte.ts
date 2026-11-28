import { t } from '@shared/lib/i18n';
import { quotaMarkdown } from '../lib/quota';
import { agentNoticeContent, notifyIfUnfocused, onNotificationClicked, requestAttention } from '@shared/lib/attention';
import { playAgentCompletionSound } from '@shared/lib/sound';
import { completionNotice } from './completion-notice';
import { PeerLink, type PeerRequest, type RemoteRef } from './peer-link';
import { load, save } from './storage';
import { agentApi, listenAgentEvents, listenConversationRequests } from '../api/agent.api';
import { feedbackPrompt } from './feedback-prompt';
import { savedModelChoice } from './legacy-model';
import { replayText } from './replay-text';
import { asideBase, btwPrompt, conversationTitle, forkItems, PROMOTED_NOTE, readOnlyMode, rejectOption } from './side-question';
import type {
  AgentCommand,
  AgentConfig,
  AgentConnection,
  AgentProfile,
  AgentQuota,
  AgentSession,
  AgentTranscriptSummary,
  Attachment,
  ChatItem,
  ConversationSummary,
  Mention,
  PermissionRequestEvent,
  PlanEntry,
  PromptBlock,
  QueuedMessage,
  SavedAside,
} from './types';

type DistributiveOmit<T, K extends PropertyKey> = T extends unknown ? Omit<T, K> : never;

const FLOATING_KEY = 'synapse.agent.floating';
const FEEDBACK_KEY = 'synapse.agent.feedback';
const LIMITS_KEY = 'synapse.agent.processLimits';
const DEFAULT_AGENT_KEY = 'synapse.agent.default';
const DEBUG_LOG_KEY = 'synapse.agent.debugLog';
/** Conversaciones por proceso si el usuario no eligió otro número. */
export const DEFAULT_PROCESS_LIMIT = 4;
/** Idas y vueltas seguidas entre agentes sin intervención del usuario. */
export const MAX_AGENT_HOPS = 3;

/** Mensaje de otra conversación esperando a que esta termine su turno. */
/** Quien escribe: otra conversación de esta ventana, o de otra bóveda (`vault`). */
export interface Sender {
  handle: string;
  agent: string;
  title: string;
  vault?: string;
}


/** Adónde vuelve la respuesta de una pregunta. */
export type ReplyTarget = { local: Conversation } | { remote: RemoteRef } | { external: true };

export interface InboxMessage {
  from: Sender;
  text: string;
  hops: number;
  /** Adónde vuelve la respuesta (si era una pregunta). */
  replyTo: ReplyTarget | null;
}



/** Conversaciones desplegadas a la vez en el acordeón. */
export const MAX_EXPANDED = 2;

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
  /** Nivel de esfuerzo, si el agente lo ofrece para el modelo actual. */
  currentEffort = $state<string | null>(null);
  attachments = $state<Attachment[]>([]);
  dismissedActiveNote = $state<string | null>(null);
  /** Lo escrito sin enviar (y las menciones elegidas en ello): sobrevive a cerrar el panel. */
  draft = $state('');
  pickedMentions = $state<Mention[]>([]);
  /** Hay novedades que el usuario no vio (terminó o pide permiso estando plegada). */
  unseen = $state(false);
  /** El agente está compactando su contexto (`/compact`). */
  compacting = $state(false);
  /** Número corto (#1, #2…) para nombrarla desde otra conversación. */
  number = 0;
  /** Mensajes de otras conversaciones que esperan a que termine su turno. */
  inbox = $state<InboxMessage[]>([]);
  /** Mensajes del usuario escritos mientras el agente trabajaba: se envían al terminar el turno, en orden. */
  queue = $state<QueuedMessage[]>([]);
  #queueId = 1;
  /** Conversaciones a las que preguntó y de las que espera respuesta (#2…). */
  waitingFor = $state<string[]>([]);
  /** Conversación abierta por otra bóveda de Synapse (🐙), con su origen. */
  incoming = $state<{ vault: string; handle: string; title: string } | null>(null);
  /** Preguntas aparte (`/btw`) de esta conversación: se guardan con ella hasta que se borren. */
  asides = $state<Conversation[]>([]);
  /** La pregunta aparte a la vista; `null` = minimizadas en su tarjeta. */
  openAside = $state<string | null>(null);
  /**
   * Conversación de la que es una pregunta aparte. Siendo aparte no tiene archivo propio (se guarda
   * dentro de la otra), no suena, no cuenta en la telemetría y no concede permisos.
   */
  #parent = $state<Conversation | null>(null);
  /** El agente bifurcó la sesión de la original: tiene lo heredado, y la pregunta aparte lo muestra. */
  #nativeHistory = $state(false);
  /** Último mensaje de la original cuando se bifurcó. */
  #baseUntil = 0;
  /** Nombre que el agente le da a la sesión (manda sobre la primera pregunta). */
  nativeTitle = $state<string | null>(null);
  /** Una pregunta aparte preparándose para enviar: su sesión no se cierra aunque deje de verse. */
  #preparing = false;
  /** Cierre en curso de su sesión: se espera antes de volver a conectarla. */
  #closing: Promise<void> | null = null;
  /** Ya está en el historial: se sigue guardando aunque se borren sus preguntas aparte. */
  #saved = false;
  /** Borrada: lo que aún termine no debe volver a crear su archivo. */
  #discarded = false;
  /** Saltos entre agentes del turno en curso (0 si lo empezó el usuario). */
  #turnHops = 0;
  /** Contador de iteraciones (turnos) completados en esta conversación. */
  #turnIndex = 0;
  /** Tiempos de inicio y fin de cada `toolCallId` para la telemetría de iteración. */
  toolTimings = new Map<string, { startedAt: number; endedAt?: number }>();

  readonly #store: AgentStore;
  readonly #cwd: string;
  #nextId = 1;
  #createdAt = Date.now();
  /** Último mensaje, para la lista de menciones. */
  lastActivity = $state(Date.now());
  #acpSessionId = $state<string | null>(null);
  #replay: string | null = null;
  /** Mensajes heredados de la original, para convertir la pregunta aparte en conversación. */
  #base: ChatItem[] = [];
  /** Sesión del agente que se bifurca al conectar, y lo que llevaba del turno en curso. */
  #forkOf: string | null = null;
  #turnReplay: string | null = null;
  /**
   * Modelo y esfuerzo propios (mandan sobre los recordados para el agente): los de la conversación de
   * origen y, en una pregunta aparte, los que se elijan en ella.
   */
  #inheritedModel: string | null = null;
  #inheritedEffort: string | null = null;
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
    return this.nativeTitle || conversationTitle(this.items);
  }
  /** Id de la sesión en el agente: la abierta o la última que tuvo. */
  get sessionId(): string | null {
    return this.session?.id ?? this.#acpSessionId;
  }
  get isAside(): boolean {
    return this.#parent !== null;
  }
  /** La conversación de la que es pregunta aparte. */
  get parent(): Conversation | null {
    return this.#parent;
  }
  /** Lo heredado que se muestra antes de sus mensajes (con el historial bifurcado por el agente). */
  get history(): ChatItem[] {
    return this.#nativeHistory && this.#parent ? this.#inheritedItems() : [];
  }
  /** Id que se guarda para retomarla: con contexto sin enviar la sesión está vacía y retomarla lo perdería. */
  get #resumableId(): string | null {
    return this.#replay && this.#replay !== PROMOTED_NOTE ? null : this.sessionId;
  }
  get pendingPermissions(): number {
    return this.items.filter(
      (i) => (i.type === 'permission' || i.type === 'elicitation') && i.answered === null,
    ).length;
  }
  /** Estado resumido para el punto de color de la fila. */
  /** Primeras líneas del último mensaje del agente (`agent`) o del usuario (`user`). */
  lastText(kind: 'agent' | 'user'): string {
    for (let i = this.items.length - 1; i >= 0; i--) {
      const it = this.items[i];
      if (it.type === kind && !(it.type === 'user' && it.from)) return it.text.trim().slice(0, 200);
    }
    return '';
  }

  get status(): 'working' | 'permission' | 'idle' | 'error' {
    if (this.pendingPermissions) return 'permission';
    if (this.connection === 'busy' || this.connection === 'starting' || this.connection === 'installing') return 'working';
    if (this.connection === 'error') return 'error';
    return 'idle';
  }

  push(item: DistributiveOmit<ChatItem, 'id'>) {
    this.items.push({ ...item, id: this.#nextId++ } as ChatItem);
    this.lastActivity = Date.now();
  }

  error(e: unknown) {
    this.push({ type: 'error', text: e instanceof Error ? e.message : String(e) });
  }

  /** Guarda la conversación en la bóveda (agrupa cambios seguidos). */
  persist(now = false): void {
    if (this.#parent) return this.#parent.persist(now);
    if (this.#saveTimer) clearTimeout(this.#saveTimer);
    this.#saveTimer = null;
    const run = () => {
      const agent = this.agent;
      if (!agent || this.#discarded) return;
      const asides = this.asides.flatMap((a) => a.#asSaved() ?? []);
      if (!this.#saved && !asides.length && !this.items.some((i) => i.type === 'user')) return;
      this.#saved = true;
      agentApi
        .saveTranscript({
          id: this.id,
          agent_id: agent.id,
          agent_name: agent.name,
          cwd: this.#cwd,
          title: this.title,
          number: this.number,
          acp_session_id: this.#resumableId,
          native_title: this.nativeTitle,
          asides,
          created_at: this.#createdAt,
          updated_at: Date.now(),
          usage: $state.snapshot(this.usage),
          items: $state.snapshot(this.items) as ChatItem[],
        })
        .then(() => this.#store.loadHistory(this.#cwd))
        .catch(() => {});
    };
    if (now) run();
    else this.#saveTimer = setTimeout(run, 800);
  }

  /** Cómo se guarda como pregunta aparte; `null` si aún no hay nada que guardar. */
  #asSaved(): SavedAside | null {
    if (!this.draft && !this.items.some((i) => i.type === 'user')) return null;
    return {
      id: this.id,
      items: $state.snapshot(this.items) as ChatItem[],
      acp_session_id: this.#resumableId,
      native_title: this.nativeTitle,
      usage: $state.snapshot(this.usage),
      created_at: this.#createdAt,
      base_until: this.#baseUntil,
      native: this.#nativeHistory || undefined,
      draft: this.draft || undefined,
      model: this.#inheritedModel,
      effort: this.#inheritedEffort,
    };
  }

  /** Una pregunta aparte guardada: sin conectar hasta que se le vuelva a preguntar. */
  #restoreAside(saved: SavedAside): Conversation {
    const c = new Conversation(this.#store, this.agentId, this.#cwd, saved.id);
    c.#parent = this;
    c.items = Array.isArray(saved.items) ? saved.items : [];
    c.#baseUntil = saved.base_until ?? 0;
    c.#nativeHistory = !!saved.native;
    c.#nextId = c.items.reduce((m, i) => Math.max(m, i.id), c.#baseUntil + 1) + 1;
    c.#createdAt = saved.created_at;
    c.#acpSessionId = saved.acp_session_id;
    c.nativeTitle = saved.native_title ?? null;
    c.usage = saved.usage ?? null;
    c.draft = saved.draft ?? '';
    c.#inheritedModel = saved.model ?? null;
    c.#inheritedEffort = saved.effort ?? null;
    return c;
  }

  /** Lo heredado de la conversación de origen (solo para una pregunta aparte). */
  #inheritedItems(): ChatItem[] {
    const parent = this.#parent;
    // Una restaurada lo reconstruye una sola vez
    if (!this.#base.length && parent) {
      this.#base = asideBase($state.snapshot(parent.items) as ChatItem[], this.#baseUntil, parent.handle, parent.title);
    }
    return this.#base;
  }

  /** Carga una conversación guardada (sus mensajes) antes de conectar. */
  async restore(transcriptId: string): Promise<string | null> {
    const transcript = await agentApi.loadTranscript(this.#cwd, transcriptId);
    if (!this.#store.agents.some((a) => a.id === transcript.agent_id)) {
      throw new Error(t('acp.store.agentMissingForTranscript', { agent: transcript.agent_name || transcript.agent_id }));
    }
    this.agentId = transcript.agent_id;
    this.items = Array.isArray(transcript.items) ? transcript.items : [];
    this.#nextId = this.items.reduce((m, i) => Math.max(m, i.id), 0) + 1;
    this.#createdAt = transcript.created_at;
    this.lastActivity = transcript.updated_at ?? transcript.created_at;
    this.#acpSessionId = transcript.acp_session_id;
    this.nativeTitle = transcript.native_title ?? null;
    this.asides = (Array.isArray(transcript.asides) ? transcript.asides : []).map((a) => this.#restoreAside(a));
    this.#saved = true;
    // El agente no repite el contexto hasta responder: mientras tanto, el de la última vez
    if (transcript.acp_session_id && transcript.usage?.size) this.usage = transcript.usage;
    if (!transcript.acp_session_id && this.items.some((i) => i.type === 'user')) this.#replay = replayText(this.items);
    return transcript.acp_session_id;
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
      this.session = await agentApi.startSession(agent.id, this.#cwd, this.#store.feedbackMode, resume ?? this.#forkOf, this.id, !resume && !!this.#forkOf);
      // Bifurcada por el agente: ya tiene el contexto, salvo lo que la original lleve del turno en curso
      if (!resume && this.session.resumed) this.#replay = this.#turnReplay;
      if (resume && !this.session.resumed) {
        // Sesión nueva en el agente: el contexto guardado ya no es el suyo. El nombre se conserva
        // hasta que el agente le ponga otro
        this.usage = null;
      }
      if (this.#parent) {
        if (!resume && this.session.resumed) this.#nativeHistory = true;
        else if (!this.session.resumed) this.#nativeHistory = false;
      }
      // Sin su contexto en el agente, viaja con el próximo mensaje (a una pregunta aparte, con lo heredado)
      if (!this.session.resumed && (resume || (this.#parent && !this.#replay)) && this.items.length) {
        this.#replay = replayText([...(this.#parent ? this.#inheritedItems() : []), ...this.items]);
      }
      this.#acpSessionId = this.session.id;
      const title = this.#store.takeEarlyTitle(this.session.id);
      if (title) this.nativeTitle = title;
      else void this.refreshTitle();
      this.currentMode = this.session.current_mode;
      this.currentModel = this.session.current_model;
      this.currentEffort = this.session.current_effort;
      this.commands = this.#store.takeEarlyCommands(this.session.id);
      this.connection = 'ready';
      void this.#store.refreshQuota(this.agentId);
      // Modelo recordado para este agente: no hay que elegirlo en cada conversación nueva
      const savedModel = this.#inheritedModel ?? load(`synapse.agent.model.${agent.id}`);
      const choice = savedModel ? savedModelChoice(savedModel, this.session.models, this.session.efforts) : null;
      const effort = this.#inheritedEffort ?? load(`synapse.agent.effort.${agent.id}`) ?? choice?.effort;
      if (choice && choice.model !== this.currentModel) await this.setModel(choice.model, !this.#inheritedModel);
      // El esfuerzo va después: los niveles dependen del modelo
      if (effort && effort !== this.currentEffort && this.session?.efforts.some((e) => e.id === effort)) {
        await this.setEffort(effort, !this.#inheritedEffort);
      }
      if (this.isAside) this.#keepConfig();
      if (resume) this.persist(true);
      this.deliverNext();
    } catch (e) {
      this.connection = 'error';
      this.error(e);
    }
  }

  /** El agente puso o cambió el nombre de la sesión. */
  setNativeTitle(title: string) {
    if (!title || title === this.nativeTitle) return;
    this.nativeTitle = title;
    this.persist();
  }

  /** Pide el nombre de la sesión a un agente que lista las suyas. */
  async refreshTitle() {
    const session = this.session;
    if (!session?.capabilities.list_sessions) return;
    const title = await agentApi.sessionTitle(session.id).catch(() => null);
    if (title && this.session?.id === session.id) this.setNativeTitle(title);
  }

  /** Renombra la sesión en el propio agente. */
  async rename(title: string) {
    const session = this.session;
    if (!session?.capabilities.session_title || !title.trim()) return;
    try {
      this.setNativeTitle(await agentApi.setTitle(session.id, title.trim()));
    } catch (e) {
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

  /**
   * Encola un mensaje escrito mientras el agente trabaja, con sus adjuntos y menciones (la caja de
   * texto queda libre). Se envía al terminar el turno.
   */
  enqueue(text: string, opts: { prompt?: string; mentions?: Mention[] } = {}) {
    if (!text.trim()) return;
    const prompt = this.isAside ? this.#asidePrompt(text, opts.prompt) : opts.prompt;
    this.queue.push({ id: this.#queueId++, text, prompt, mentions: opts.mentions ?? [], attachments: this.pendingAttachments });
    this.attachments = [];
    const note = this.#store.activeNote;
    if (note) this.dismissedActiveNote = note.path;
    this.deliverNext();
  }

  /** Saca un mensaje de la cola (para editarlo o cancelarlo); sus adjuntos vuelven a la caja si `restore` */
  unqueue(id: number, restore = false): QueuedMessage | null {
    const i = this.queue.findIndex((q) => q.id === id);
    if (i < 0) return null;
    const [item] = this.queue.splice(i, 1);
    if (restore) for (const a of item.attachments) this.attach(a);
    return item;
  }

  /** Vacía la cola devolviendo sus mensajes (al detener el turno vuelven a la caja de texto) */
  takeQueue(): QueuedMessage[] {
    const all = [...this.queue];
    this.queue = [];
    for (const item of all) for (const a of item.attachments) this.attach(a);
    return all;
  }

  /** Entrega lo siguiente si la conversación está libre: primero lo encolado por el usuario, luego el buzón. */
  deliverNext() {
    if (this.connection !== 'ready') return;
    if (this.queue.length) {
      const q = this.queue.shift()!;
      void this.send(q.text, { prompt: q.prompt, mentions: q.mentions, attachments: q.attachments });
      return;
    }
    if (!this.inbox.length) return;
    const msg = this.inbox.shift()!;
    const from = msg.from;
    const origin = from.vault ? t('acp.store.originOtherVault', { vault: from.vault }) : t('acp.store.originOtherConv');
    const prompt =
      t('acp.store.incomingHeader', {
        origin,
        agent: from.agent,
        handle: from.handle,
        titlePart: from.title ? ` · ${from.title}` : '',
      }) +
      (msg.replyTo ? t('acp.store.incomingQuestionNote') : t('acp.store.incomingNoticeNote')) +
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

  /** `/usage`: el resumen sale ya; la cuota de la cuenta se añade al mismo mensaje cuando llega. */
  async #reportUsage() {
    const report = this.usageReport();
    const name = this.agent?.name ?? this.agentId;
    const pending = this.#store.knowsQuota(this.agentId);
    this.push({ type: 'info', text: pending ? `${report}\n\n${t('quota.loading')}` : report });
    const item = this.items[this.items.length - 1];
    const set = (extra: string) => {
      if (item.type === 'info') item.text = `${report}\n\n${extra}`;
    };
    try {
      const quota = (await this.#store.refreshQuota(this.agentId, { report: true })) ?? null;
      if (quota) set(quotaMarkdown(quota, name));
      else if (pending && item.type === 'info') item.text = report;
    } catch (e) {
      const last = this.#store.quotas[this.agentId];
      const failed = t('quota.failed', { error: e instanceof Error ? e.message : String(e) });
      set(last ? `${quotaMarkdown(last, name)}\n\n${failed}` : failed);
    }
    this.persist();
  }

  /** Resumen de uso de la conversación (`/usage`), en markdown. */
  usageReport(): string {
    const fmt = (n: number) => n.toLocaleString();
    const u = this.usage;
    const model = this.session?.models.find((m) => m.id === this.currentModel)?.name ?? this.currentModel;
    const mode = this.session?.modes.find((m) => m.id === this.currentMode)?.name ?? this.currentMode;
    const users = this.items.filter((i) => i.type === 'user').length;
    const replies = this.items.filter((i) => i.type === 'agent').length;
    const tools = this.items.filter((i) => i.type === 'tool');
    const byKind = new Map<string, number>();
    const otherKind = t('acp.store.usageToolOther');
    for (const tool of tools) if (tool.type === 'tool') byKind.set(tool.kind ?? otherKind, (byKind.get(tool.kind ?? otherKind) ?? 0) + 1);
    const compacts = this.items.filter((i) => i.type === 'compact').length;
    const minutes = Math.round((Date.now() - this.#createdAt) / 60_000);
    const rows = [
      [t('acp.store.usageAgent'), this.agent?.name ?? this.agentId],
      [t('acp.store.usageModel'), model ?? '—'],
      ...(this.currentEffort
        ? [[t('acp.store.usageEffort'), this.session?.efforts.find((e) => e.id === this.currentEffort)?.name ?? this.currentEffort]]
        : []),
      [t('acp.store.usageMode'), mode ?? '—'],
      [
        t('acp.store.usageContext'),
        u
          ? t('acp.store.usageContextValue', {
              used: fmt(u.used),
              size: fmt(u.size),
              percent: ((u.used / u.size) * 100).toFixed(1),
            })
          : t('acp.store.usageNotReported'),
      ],
      [t('acp.store.usageCost'), u?.cost ? `${u.cost.amount} ${u.cost.currency}` : t('acp.store.usageNotReported')],
      [t('acp.store.usageMessages'), t('acp.store.usageMessagesValue', { users, replies })],
      [
        t('acp.store.usageTools'),
        tools.length ? `${tools.length} (${[...byKind].map(([k, n]) => `${k}: ${n}`).join(', ')})` : t('acp.store.usageNone'),
      ],
      [
        t('acp.store.usageCompacted'),
        compacts ? t('acp.store.usageCompactedValue', { count: compacts }) : t('acp.store.usageNever'),
      ],
      [t('acp.store.usageDuration'), minutes < 1 ? t('acp.store.usageLessThanMinute') : `${minutes} min`],
    ];
    return `${t('acp.store.usageTitle')}\n\n| | |\n|---|---|\n${rows.map(([k, v]) => `| ${k} | ${v} |`).join('\n')}`;
  }

  async send(
    text: string,
    opts: {
      prompt?: string;
      from?: Sender;
      hops?: number;
      replyTo?: ReplyTarget | null;
      mentions?: Mention[];
      /** Adjuntos ya elegidos (los de un mensaje encolado); si no, los pendientes */
      attachments?: Attachment[];
    } = {},
  ) {
    const session = this.session;
    if (!session || this.connection === 'busy' || !text.trim()) return;
    // `/usage` lo responde Synapse (salvo que el agente tenga su propio `/usage`)
    if (/^\/usage\s*$/.test(text.trim()) && !this.commands.some((c) => c.name === 'usage')) {
      this.push({ type: 'user', text, attachments: [] });
      void this.#reportUsage();
      return;
    }
    this.#turnHops = opts.hops ?? 0;
    const turnStart = this.items.length;
    if (/(^|\s)@this\b/.test(text)) this.attachActiveNote();
    // `/feedback [indicaciones]`: lo resuelve Synapse, no el agente
    const feedback = this.#store.feedbackAvailable ? /^\/feedback(?:\s+([\s\S]*))?$/.exec(text.trim()) : null;
    const attachments = opts.attachments ?? this.pendingAttachments;
    const blocks: PromptBlock[] = [];
    const replay = this.#replay;
    if (replay) {
      blocks.push({ type: 'text', text: replay });
      this.#replay = null;
    }
    const prompt = opts.prompt ?? (feedback ? feedbackPrompt(feedback[1]?.trim() ?? '') : text);
    blocks.push({ type: 'text', text: prompt });
    // En una pregunta aparte se ve (plegado) lo que se le manda para empezarla: su primer envío y,
    // si se abre otra sesión, el contexto que se le reenvía
    if (this.isAside && (replay || !this.items.some((i) => i.type === 'user'))) {
      this.push({ type: 'context', text: replay ? `${replay}\n\n${prompt}` : prompt });
    }
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
    this.push(
      opts.from
        ? { type: 'user', text, attachments, from: opts.from }
        : { type: 'user', text, attachments, ...(opts.mentions?.length ? { mentions: opts.mentions } : {}) },
    );
    this.attachments = [];
    // Tras enviarla, la nota activa no se repite hasta que cambie
    const note = this.#store.activeNote;
    if (note) this.dismissedActiveNote = note.path;
    this.plan = [];
    this.connection = 'busy';
    this.persist(true);
    const turnStartedAt = Date.now();
    const prevCostAmount = this.usage?.cost?.amount ?? null;
    const turnIndex = ++this.#turnIndex;
    let stopReason = 'error';
    try {
      const stop = await agentApi.sendPrompt(session.id, blocks);
      stopReason = stop;
      if (stop === 'refusal') this.push({ type: 'error', text: t('acp.store.refusalError') });
    } catch (e) {
      this.error(e);
    } finally {
      const turnEndedAt = Date.now();
      if (this.session?.id === session.id) this.connection = 'ready';
      if (compact) this.compacting = false;
      if (!this.#store.isVisible(this)) this.unseen = true;
      this.persist(true);
      const agent = this.agent;
      if (agent && !this.isAside) {
        const toolCalls = this.items
          .slice(turnStart)
          .filter((i): i is Extract<ChatItem, { type: 'tool' }> => i.type === 'tool')
          .map((t) => {
            const timing = this.toolTimings.get(t.toolCallId);
            const started_at = timing?.startedAt ?? null;
            const ended_at = timing?.endedAt ?? null;
            return {
              tool_call_id: t.toolCallId,
              title: t.title,
              kind: t.kind,
              status: t.status,
              started_at,
              ended_at,
              duration_ms: started_at != null && ended_at != null ? Math.max(0, ended_at - started_at) : null,
            };
          });
        const costAmount = this.usage?.cost?.amount ?? null;
        const costDelta =
          costAmount != null ? Math.max(0, costAmount - (prevCostAmount ?? 0)) : null;
        void agentApi
          .recordIteration({
            cwd: this.#cwd,
            conversation_id: this.id,
            agent_id: agent.id,
            agent_name: agent.name,
            acp_session_id: this.session?.id ?? this.#acpSessionId,
            iteration: {
              id: `iter-${turnStartedAt}-${turnIndex}`,
              turn_index: turnIndex,
              model: this.currentModel,
              mode: this.currentMode,
              effort: this.currentEffort === 'default' ? null : this.currentEffort,
              started_at: turnStartedAt,
              ended_at: turnEndedAt,
              duration_ms: Math.max(0, turnEndedAt - turnStartedAt),
              stop_reason: stopReason,
              usage_used: this.usage?.used ?? null,
              usage_size: this.usage?.size ?? null,
              cost_amount: costAmount,
              cost_delta: costDelta,
              cost_currency: this.usage?.cost?.currency ?? null,
              tool_calls: toolCalls,
              mcp_call_ids: [],
            },
          })
          .catch(() => {});
      }
      // Era una pregunta de otra conversación: su respuesta vuelve sola
      if (opts.replyTo) {
        const answer = this.items
          .slice(turnStart)
          .filter((i) => i.type === 'agent')
          .map((i) => ('text' in i ? i.text : ''))
          .join('\n\n')
          .trim();
        const target = opts.replyTo;
        const text = answer || t('acp.store.noAnswerFallback');
        const hops = (opts.hops ?? 0) + 1;
        if ('local' in target) {
          const asker = target.local;
          asker.waitingFor = asker.waitingFor.filter((h) => h !== this.handle);
          if (this.#store.byId(asker.id)) asker.receive({ from: this.#store.senderOf(this), text, hops, replyTo: null });
        } else if ('remote' in target) {
          void this.#store.peers.replyRemote(target.remote, this, text, hops);
        } else {
          // Un cliente MCP externo la recoge con `read_replies`
          this.#store.externalReplies.push({ from: `${this.handle} · ${this.agent?.name ?? this.agentId}`, text, at: Date.now() });
        }
      }
      this.#turnHops = 0;
      void this.#store.refreshQuota(this.agentId);
      // Un agente que no avisa de su nombre lo tiene tras el primer turno
      if (!this.nativeTitle && !this.session?.capabilities.session_title) void this.refreshTitle();
      if (stopReason !== 'cancelled' && !opts.replyTo && !this.isAside && this.queue.length === 0) {
        void playAgentCompletionSound();
        // La respuesta final es el último mensaje del turno (los anteriores acompañan a sus herramientas)
        const last = this.items.slice(turnStart).findLast((i) => i.type === 'agent');
        const answer = agentNoticeContent() === 'answer' && last?.type === 'agent' ? last.text : undefined;
        const notice = completionNotice(this.agent?.name ?? t('acp.store.fallbackAgentName'), this.handle, text, stopReason === 'error', answer);
        notifyIfUnfocused(notice.title, notice.body, this.id);
      }
      this.deliverNext();
      // Una pregunta aparte que respondió sin estar a la vista ya no necesita su sesión
      this.#releaseIfHidden();
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

  async setModel(modelId: string, remember = true) {
    if (!this.session) return;
    // Lo elegido en una pregunta aparte es solo suyo: no cambia el recordado para el agente
    if (remember && !this.isAside) save(`synapse.agent.model.${this.session.agent_id}`, modelId);
    const previous = this.currentModel;
    this.currentModel = modelId;
    try {
      const option = this.session.model_option;
      if (option) this.applyConfig(await agentApi.setOption(this.session.id, option, modelId));
      else await agentApi.setModel(this.session.id, modelId);
      if (remember && this.isAside) this.#keepConfig();
    } catch (e) {
      this.currentModel = previous;
      this.error(e);
    }
  }

  async setEffort(effortId: string, remember = true) {
    const option = this.session?.effort_option;
    if (!this.session || !option) return;
    if (remember && !this.isAside) save(`synapse.agent.effort.${this.session.agent_id}`, effortId);
    const previous = this.currentEffort;
    this.currentEffort = effortId;
    try {
      this.applyConfig(await agentApi.setOption(this.session.id, option, effortId));
      if (remember && this.isAside) this.#keepConfig();
    } catch (e) {
      this.currentEffort = previous;
      this.error(e);
    }
  }

  /** Conserva el modelo y el esfuerzo que tiene ahora, para volver a ellos al reconectar. */
  #keepConfig() {
    this.#inheritedModel = this.currentModel;
    this.#inheritedEffort = this.currentEffort;
    this.persist();
  }

  /** Modelos y esfuerzos tal como los deja el agente (cambiar de modelo puede cambiar los niveles). */
  applyConfig(config: AgentConfig) {
    if (!this.session) return;
    // Una actualización sin modelos (opciones ajenas) no borra los que ya se conocían
    if (config.models.length) {
      this.session.models = config.models;
      this.session.model_option = config.model_option;
      this.currentModel = config.current_model;
    }
    this.session.efforts = config.efforts;
    this.session.effort_option = config.effort_option;
    this.currentEffort = config.current_effort;
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

  async answerElicitation(
    itemId: number,
    action: 'accept' | 'decline' | 'cancel',
    content?: Record<string, unknown> | null,
  ) {
    const item = this.items.find((i) => i.id === itemId);
    if (!item || item.type !== 'elicitation' || item.answered !== null) return;
    item.answered = { action, content: content ?? null };
    this.persist();
    const sessionId = item.request.sessionId || this.session?.id || '';
    try {
      await agentApi.respondElicitation(sessionId, item.request.requestId, action, content ?? null);
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

  /** Toma el contexto de `parent`: sus mensajes hasta ahora viajan con el primer envío. */
  #inherit(parent: Conversation) {
    const items = forkItems($state.snapshot(parent.items) as ChatItem[]);
    this.#baseUntil = items.reduce((m, i) => Math.max(m, i.id), 0);
    this.#nextId = this.#baseUntil + 1;
    this.#base = items.length ? [...items, { type: 'fork', id: this.#nextId++, handle: parent.handle, title: parent.title }] : [];
    if (items.some((i) => i.type === 'user')) this.#replay = replayText(items);
    this.#inheritedModel = parent.currentModel;
    this.#inheritedEffort = parent.currentEffort;
    if (!this.#replay) return;
    // Si la original aún no le envió su contexto al agente, bifurcar su sesión lo perdería
    this.#forkOf = parent.#replay ? null : (parent.session?.id ?? null);
    if (parent.connection === 'busy') this.#turnReplay = replayText(items.slice(items.findLastIndex((i) => i.type === 'user')));
  }

  /** `/fork`: otra conversación con el contexto de esta (también con el turno en curso). */
  async fork(text = ''): Promise<Conversation> {
    const c = new Conversation(this.#store, this.agentId, this.#cwd);
    c.#inherit(this);
    c.items = c.#base;
    this.#store.addConversation(c);
    c.enqueue(text);
    await c.connect();
    c.persist(true);
    return c;
  }

  /** `/btw`: pregunta aparte, sin tocar el turno en curso ni el contexto de esta conversación. */
  async ask(question: string) {
    const c = new Conversation(this.#store, this.agentId, this.#cwd);
    c.#parent = this;
    c.#inherit(this);
    c.draft = question;
    this.asides = [...this.asides, c];
    this.openAside = c.id;
    this.#releaseHiddenAsides();
    // La primera pregunta viene de la caja principal: sus adjuntos se quedan allí
    await c.askAside(question, { attachments: [] });
  }

  /** Sigue siendo suya: ni se borró ni se cerró su conversación. */
  get #attached(): boolean {
    const parent = this.#parent;
    return !!parent && parent.asides.includes(this) && !!this.#store.byId(parent.id);
  }

  /** Lo que recibe una pregunta aparte: con la indicación de no actuar, salvo un `/comando`. */
  #asidePrompt(text: string, prompt?: string): string | undefined {
    return text.startsWith('/') ? prompt : btwPrompt(prompt ?? text);
  }

  /**
   * Pregunta (o repregunta) en la misma sesión aparte, conectándola si hace falta. Sin `attachments`
   * van los de su caja.
   */
  async askAside(question: string, opts: { prompt?: string; mentions?: Mention[]; attachments?: Attachment[] } = {}) {
    if (!question.trim() || !['idle', 'ready', 'error'].includes(this.connection)) return;
    // Si no llega a enviarse, queda escrita para reintentar
    this.draft = question;
    this.#preparing = true;
    try {
      await this.#reconnect();
      if (!this.#attached) return this.stop();
      if (!this.session) return this.persist();
      const mode = readOnlyMode(this.session.modes);
      if (mode && mode !== this.currentMode) await this.setMode(mode);
      if (!this.#attached) return this.stop();
    } finally {
      this.#preparing = false;
    }
    this.draft = '';
    await this.send(question, { ...opts, prompt: this.#asidePrompt(question, opts.prompt) });
  }

  /** Conecta una pregunta aparte que no tiene sesión, retomando la que tuvo. */
  async #reconnect() {
    if (this.session) return;
    if (this.#closing) {
      // Ocupada desde ya: mientras se cierra la anterior no admite otra pregunta
      this.connection = 'starting';
      await this.#closing;
    }
    await this.connect(this.#acpSessionId);
  }

  /**
   * Una pregunta aparte que dejó de verse cierra su sesión, si está libre y el agente sabe cerrarla y
   * retomarla; se vuelve a conectar al preguntarle otra vez. Plegar la conversación o el panel no cuenta.
   */
  #releaseIfHidden() {
    const parent = this.#parent;
    const capabilities = this.session?.capabilities;
    if (!parent || parent.openAside === this.id || this.#preparing) return;
    if (this.connection !== 'ready' || this.queue.length) return;
    if (!capabilities?.load_session || !capabilities.close_session) return;
    // Al retomarla sigue con el modelo y el esfuerzo que tenía
    this.#keepConfig();
    // Como una recién restaurada: sin comandos hasta que el agente los vuelva a anunciar
    this.commands = [];
    this.#closing = this.stop().finally(() => (this.#closing = null));
  }

  #releaseHiddenAsides() {
    for (const aside of this.asides) aside.#releaseIfHidden();
  }

  showAside(id: string) {
    const aside = this.asides.find((a) => a.id === id);
    if (!aside) return;
    this.openAside = id;
    aside.unseen = false;
    this.#releaseHiddenAsides();
  }

  /** Recoge las preguntas aparte en su tarjeta, sin cerrar nada. */
  minimizeAsides() {
    const open = this.asides.find((a) => a.id === this.openAside);
    if (open) open.unseen = false;
    this.openAside = null;
    this.#releaseHiddenAsides();
  }

  /** Borra una pregunta aparte (lo único que la quita). */
  async removeAside(id: string) {
    const i = this.asides.findIndex((a) => a.id === id);
    if (i < 0) return;
    const aside = this.asides[i];
    this.asides = this.asides.filter((a) => a !== aside);
    if (this.openAside === id) this.openAside = this.asides[Math.min(i, this.asides.length - 1)]?.id ?? null;
    await aside.stop();
    this.persist(true);
  }

  /** Convierte una pregunta aparte, ya respondida, en una conversación normal. */
  async promoteAside(id: string) {
    const c = this.asides.find((a) => a.id === id);
    if (!c || c.status === 'working') return;
    c.#preparing = true;
    try {
      await c.#reconnect();
    } finally {
      c.#preparing = false;
    }
    if (!c.#attached) return c.stop();
    const session = c.session;
    if (!session) return;
    const base = c.#inheritedItems();
    this.asides = this.asides.filter((a) => a !== c);
    if (this.openAside === id) this.openAside = null;
    c.#parent = null;
    c.items = [...base, ...c.items];
    c.#replay = c.#replay ? `${c.#replay}\n\n${PROMOTED_NOTE}` : PROMOTED_NOTE;
    // Fuera del modo de solo lectura: el de la sesión al abrirse o, si era ese, el de la original
    const readOnly = readOnlyMode(session.modes);
    const mode = session.current_mode !== readOnly ? session.current_mode : this.currentMode;
    this.#store.addConversation(c);
    if (mode && mode !== c.currentMode) await c.setMode(mode);
    c.persist(true);
    this.persist(true);
  }

  /** Deja de guardarse: se va a borrar. */
  discard() {
    this.#discarded = true;
    if (this.#saveTimer) clearTimeout(this.#saveTimer);
  }

  /** Una pregunta aparte no concede nada: rechaza y lo deja dicho en el panel. */
  rejectPermission(request: PermissionRequestEvent) {
    this.push({ type: 'info', text: t('acp.store.permissionRejected', { title: request.title }) });
    void agentApi
      .respondPermission(request.sessionId, request.requestId, rejectOption(request.options))
      .catch((e) => this.error(e));
  }

  /** Cierra la sesión del agente; la conversación queda guardada en el historial. */
  async stop() {
    const asides = this.asides.map((a) => a.stop());
    const session = this.session;
    if (session) {
      this.#acpSessionId = session.id;
      this.persist(true);
    }
    this.session = null;
    this.connection = 'idle';
    if (session) await agentApi.stopSession(session.id).catch(() => {});
    await Promise.all(asides);
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
const clampQuotaMinutes = (n: number) =>
  Number.isFinite(n) ? Math.min(MAX_QUOTA_MINUTES, Math.max(1, Math.round(n))) : DEFAULT_QUOTA_MINUTES;

/** Agentes cuya cuota se pregunta aparte; Claude la adjunta a la conversación. */
const QUOTA_AGENTS = ['antigravity'];
const QUOTA_MINUTES_KEY = 'synapse.agent.quotaMinutes';
export const DEFAULT_QUOTA_MINUTES = 3;
export const MAX_QUOTA_MINUTES = 60;
/** Al pedirla con un clic: lo justo para no lanzar otra consulta con cada clic. */
const QUOTA_CLICK_AGE = 10_000;

export class AgentStore {
  agents = $state<AgentProfile[]>([]);
  listError = $state<string | null>(null);
  /** Minutos que vale la última cuota antes de volver a preguntarla. */
  quotaMinutes = $state(clampQuotaMinutes(Number(load(QUOTA_MINUTES_KEY) ?? DEFAULT_QUOTA_MINUTES)));
  setQuotaMinutes(minutes: number) {
    this.quotaMinutes = clampQuotaMinutes(minutes);
    save(QUOTA_MINUTES_KEY, String(this.quotaMinutes));
  }
  /** Última cuota conocida de la cuenta de cada agente. */
  quotas = $state<Record<string, AgentQuota>>({});

  /** De este agente se puede preguntar la cuota (los que la adjuntan solos no hace falta). */
  knowsQuota(agentId: string): boolean {
    return QUOTA_AGENTS.includes(agentId) && this.agents.find((a) => a.id === agentId)?.source === 'verified';
  }

  /**
   * Renueva la cuota si la última respuesta tiene más de `quotaMinutes`; con `now`, casi siempre. Los
   * fallos se callan (se sigue viendo la última), salvo con `report`.
   */
  async refreshQuota(agentId: string, { report = false, now = false } = {}): Promise<AgentQuota | null> {
    if (!this.knowsQuota(agentId)) return this.quotas[agentId] ?? null;
    try {
      const quota = await agentApi.quota(agentId, now ? QUOTA_CLICK_AGE : this.quotaMinutes * 60_000);
      // Una respuesta guardada no pisa un dato más nuevo que haya adjuntado el agente
      if (quota && quota.fetched_at >= (this.quotas[agentId]?.fetched_at ?? 0)) this.quotas[agentId] = quota;
      return this.quotas[agentId] ?? quota;
    } catch (e) {
      if (report) throw e;
      return null;
    }
  }
  floating = $state(load(FLOATING_KEY) === '1');
  /** Modo feedback: el agente anota lo que le falta al MCP de Synapse (aplica a nuevas sesiones). */
  feedbackMode = $state(load(FEEDBACK_KEY) === '1');
  /** Registro de depuración: cada mensaje ACP crudo en un archivo (Ajustes). */
  debugLog = $state(load(DEBUG_LOG_KEY) === '1');
  debugLogPath = $state<string | null>(null);
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
  /** Agente por defecto de todas las bóvedas: el de una bóveda que nunca usó ninguno (p. ej. cuando
   *  otra bóveda le escribe por primera vez) y el de «＋ Nueva conversación». */
  defaultAgentId = $state<string | null>(load(DEFAULT_AGENT_KEY));
  /** Conversaciones por proceso de cada agente (Ajustes); sin valor, el del backend (4). */
  processLimits = $state<Record<string, number>>(JSON.parse(load(LIMITS_KEY) ?? '{}'));
  /** Panel visible; se recuerda por bóveda junto con sus conversaciones. */
  panelOpen = $state(false);
  /** Estado guardado de la bóveda, pendiente de restaurar cuando se abra el panel. */
  #toRestore: VaultAgentState | null = null;

  #cwd: string | null = null;
  #nextNumber = 1;
  /** Respuestas para clientes MCP externos (sin conversación en el panel), hasta que las recojan. */
  externalReplies: { from: string; text: string; at: number }[] = [];
  /** Bóveda de esta instancia, para presentarse ante otras. */
  /** Conversaciones con otras bóvedas de Synapse (canal `/peer`). */
  readonly peers = new PeerLink(this);
  #early = new Map<string, AgentCommand[]>();
  #earlyTitles = new Map<string, string>();
  #listening: Promise<unknown> | null = null;

  pinnedConversation = $derived(this.conversations.find((c) => c.id === this.pinned) ?? null);
  pendingPermissions = $derived(this.conversations.reduce((n, c) => n + c.pendingPermissions, 0));
  working = $derived(this.conversations.filter((c) => c.status === 'working').length);

  /** Bóveda abierta en esta ventana. */
  get cwd(): string | null {
    return this.#cwd;
  }

  byId(id: string) {
    return this.conversations.find((c) => c.id === id) ?? null;
  }

  #bySession(sessionId: string) {
    for (const c of this.conversations) {
      if (c.session?.id === sessionId) return c;
      const aside = c.asides.find((a) => a.session?.id === sessionId);
      if (aside) return aside;
    }
    return null;
  }

  isVisible(c: Conversation): boolean {
    if (c.parent) return c.parent.openAside === c.id && this.isVisible(c.parent);
    return this.pinned === c.id || this.expanded.includes(c.id);
  }

  takeEarlyCommands(sessionId: string): AgentCommand[] {
    const commands = this.#early.get(sessionId) ?? [];
    this.#early.delete(sessionId);
    return commands;
  }

  takeEarlyTitle(sessionId: string): string | null {
    const title = this.#earlyTitles.get(sessionId) ?? null;
    this.#earlyTitles.delete(sessionId);
    return title;
  }

  #listen() {
    this.#listening ??= (void onNotificationClicked((id) => this.reveal(id)), listenConversationRequests((req) => this.handleConversationRequest(req)), listenAgentEvents({
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
        if (c && e.quota) this.quotas[c.agentId] = e.quota;
        if (c && e.used != null && e.size) c.usage = { used: e.used, size: e.size, cost: e.cost ?? null };
      },
      onMode: (e) => {
        const c = this.#bySession(e.sessionId);
        if (c) c.currentMode = e.modeId;
      },
      onSessionInfo: (e) => {
        // Como los comandos: puede llegar antes de conocer la sesión
        const c = this.#bySession(e.sessionId);
        if (c) c.setNativeTitle(e.title);
        else this.#earlyTitles.set(e.sessionId, e.title);
      },
      onConfig: (e) => this.#bySession(e.sessionId)?.applyConfig(e.config),
      onStream: (e) => this.#bySession(e.sessionId)?.onStream(e.kind, e.text),
      onToolCall: (e) => {
        const c = this.#bySession(e.sessionId);
        if (!c) return;
        const now = Date.now();
        let timing = c.toolTimings.get(e.toolCallId);
        if (!timing) {
          timing = { startedAt: now };
          c.toolTimings.set(e.toolCallId, timing);
        }
        if (e.status === 'completed' || e.status === 'failed' || e.status === 'error') {
          timing.endedAt = now;
        }
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
        if (c.isAside) return c.rejectPermission(e);
        c.push({ type: 'permission', request: e, answered: null });
        const item = c.items.at(-1)!;
        if (!this.isVisible(c)) c.unseen = true;
        if (c.incoming) this.peers.incomingPermission(c, item.id, e);
        const tool = c.items.find((i) => i.type === 'tool' && i.toolCallId === e.toolCallId);
        requestAttention(
          t('acp.store.asksPermission', { agent: c.agent?.name ?? t('acp.store.fallbackAgentName') }),
          `${c.handle}${tool && tool.type === 'tool' ? ` · ${tool.title}` : ''}`,
          c.id,
        );
      },
      onElicitation: (e) => {
        const c =
          (e.sessionId ? this.#bySession(e.sessionId) : undefined) ??
          this.conversations.find((x) => x.connection === 'busy');
        if (!c) return;
        if (c.isAside) {
          c.push({ type: 'info', text: t('acp.store.asideAskedInfo', { message: e.message }) });
          void agentApi.respondElicitation(c.session?.id ?? '', e.requestId, 'cancel', null).catch(() => {});
          return;
        }
        c.push({ type: 'elicitation', request: e, answered: null });
        if (!this.isVisible(c)) c.unseen = true;
        requestAttention(
          t('acp.store.asksInfo', { agent: c.agent?.name ?? t('acp.store.fallbackAgentName') }),
          `${c.handle} · ${e.message}`,
          c.id,
        );
      },
      onStatus: (e) => {
        const c = this.#bySession(e.sessionId);
        if (!c) return;
        c.session = null;
        c.connection = 'idle';
        c.push({ type: 'error', text: t('acp.store.agentTerminated') });
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

  /** Cuántas conversaciones comparte un proceso del agente (1 si atiende una a la vez). */
  setProcessLimit(agentId: string, limit: number) {
    this.processLimits = { ...this.processLimits, [agentId]: limit };
    save(LIMITS_KEY, JSON.stringify(this.processLimits));
    void agentApi.setProcessLimit(agentId, limit).catch(() => {});
  }

  setDefaultAgent(agentId: string | null) {
    this.defaultAgentId = agentId;
    save(DEFAULT_AGENT_KEY, agentId);
  }

  /** Agente para abrir una conversación sin elegirlo: el último usado en la bóveda, el agente por
   *  defecto, el primero instalado o, si no hay ninguno, el primero de la lista (se descarga solo). */
  get fallbackAgentId(): string | null {
    const known = (id: string | null) => (id && this.agents.some((a) => a.id === id) ? id : null);
    return (
      known(this.lastAgentId) ??
      known(this.defaultAgentId) ??
      this.agents.find((a) => a.installed)?.id ??
      this.agents[0]?.id ??
      null
    );
  }

  setIncomingAgent(agentId: string | null) {
    this.incomingAgentId = agentId;
    if (this.#cwd) save(`synapse.agent.incoming.${this.#cwd}`, agentId);
  }

  /** Empieza a atender los eventos de los agentes y los pedidos del MCP y de otras bóvedas. */
  startListening() {
    this.setDebugLog(this.debugLog);
    return this.#listen();
  }

  /** Conversación en la que se pidió buscar (Ctrl+F); la consume su chat. */
  searchRequest = $state<string | null>(null);

  /** Abre la búsqueda de una conversación, desplegándola si hace falta. */
  /** Clic en una notificación: abre el panel con esa conversación a la vista. */
  reveal(id: string) {
    if (!this.byId(id)) return;
    this.setPanelOpen(true);
    this.expand(id);
  }

  searchIn(id: string) {
    const aside = this.conversations.some((c) => c.asides.some((a) => a.id === id));
    const c = this.byId(id);
    if (!c && !aside) return;
    if (c && !this.isVisible(c)) this.expand(id);
    this.searchRequest = id;
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


  /** Peticiones del MCP (`list_conversations`, `send_to_conversation`) y de otras instancias (`peer`). */
  async handleConversationRequest(req: {
    action: string;
    caller: string | null;
    to?: string;
    message?: string;
    expectsReply?: boolean;
    request?: unknown;
  }): Promise<unknown> {
    if (req.action === 'peer') return this.peers.handlePeer(req.request as PeerRequest);
    if (req.action === 'send' && this.conversations.some((c) => c.asides.some((a) => a.id === req.caller))) {
      throw new Error(t('acp.store.asideCannotSend'));
    }
    const caller = req.caller ? this.byId(req.caller) : null;
    if (req.action === 'list') {
      const local = this.localSummaries(caller);
      // Las conversaciones de las otras bóvedas abiertas, con el nombre de su bóveda delante
      const peers = await agentApi.peerList({ action: 'list' }).catch(() => []);
      const remote = peers.flatMap((p) =>
        Array.isArray(p.result)
          ? (p.result as { handle: string }[]).map((c) => ({ ...c, handle: `${p.vault}${c.handle}`, vault: p.vault, window: t('acp.store.otherWindow') }))
          : [],
      );
      const me = await this.peers.me();
      return [
        ...local.map((c) => ({ ...c, vault: me?.vault ?? null, window: t('acp.store.thisWindow') })),
        ...remote,
        ...peers
          .filter((p) => !Array.isArray(p.result) || !(p.result as unknown[]).length)
          .map((p) => ({ handle: `${p.vault}#${t('acp.mentions.newTag')}`, vault: p.vault, window: t('acp.store.otherWindow'), note: t('acp.store.emptyRemoteVaultNote') })),
      ];
    }
    if (req.action === 'replies') return this.externalReplies.splice(0);
    if (req.action !== 'send') throw new Error(t('acp.store.unknownAction', { action: req.action }));
    if (!caller) return this.#sendFromExternal(req.to ?? '', req.message ?? '', !!req.expectsReply);
    const to = (req.to ?? '').trim();
    const hops = caller.turnHops + 1;
    if (hops > MAX_AGENT_HOPS) {
      throw new Error(t('acp.store.hopLimitDetail', { max: MAX_AGENT_HOPS }));
    }
    const expectsReply = !!req.expectsReply;
    // «mainframe», «mainframe#2» o «mainframe#nueva»: otra bóveda
    const remote = await this.peers.remoteTarget(to);
    if (remote) return this.peers.sendRemote(caller, remote, req.message ?? '', expectsReply, hops);
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
        (status === 'queued' ? t('acp.store.queuedPrefix') : t('acp.store.deliveredPrefix')) +
        (expectsReply ? t('acp.peer.replyWaitAsync') : ''),
    };
  }

  /**
   * Mensaje de un cliente MCP externo (por ejemplo, Claude Code usando el MCP de Synapse): solo a
   * conversaciones de esta ventana; si pregunta, la respuesta queda para `read_replies`.
   */
  #sendFromExternal(to: string, message: string, expectsReply: boolean) {
    const target = this.#resolveTarget(to.trim(), null);
    const status = target.receive({
      from: { handle: t('acp.store.externalAgentHandle'), agent: t('acp.store.externalMcpClient'), title: '' },
      text: message,
      hops: 1,
      replyTo: expectsReply ? { external: true } : null,
    });
    return {
      status,
      to: `${target.handle} · ${target.agent?.name ?? target.agentId}`,
      note:
        (status === 'queued' ? t('acp.store.queuedPrefix') : t('acp.store.deliveredPrefix')) +
        (expectsReply ? t('acp.store.externalReplyHint') : ''),
    };
  }

  /** Resumen de las conversaciones abiertas (para `list_conversations` y para otras bóvedas). */
  localSummaries(caller: Conversation | null) {
    return this.conversations.map((c) => ({
      handle: c.handle,
      agent: c.agent?.name ?? c.agentId,
      model: c.session?.models.find((m) => m.id === c.currentModel)?.name ?? c.currentModel ?? undefined,
      title: c.title || t('acp.store.newConversation'),
      status: c.status,
      pending_messages: c.inbox.length,
      is_you: c.id === caller?.id,
      incoming_from: c.incoming ? `${c.incoming.vault}${c.incoming.handle}` : undefined,
      number: c.number,
      last_reply: c.lastText('agent'),
      last_question: c.lastText('user'),
      last_activity: c.lastActivity,
    }));
  }









  /** Destino por número (#2), id, o parte de su título o del nombre del agente. */
  #resolveTarget(to: string, caller: Conversation | null): Conversation {
    const q = to.trim().toLowerCase();
    const others = this.conversations.filter((c) => c.id !== caller?.id);
    const byNumber = /^#?(\d+)$/.exec(q);
    const exact = byNumber ? others.find((c) => c.number === Number(byNumber[1])) : others.find((c) => c.id === to);
    if (exact) return exact;
    const matches = others.filter((c) => `${c.agent?.name ?? c.agentId} ${c.title}`.toLowerCase().includes(q));
    if (matches.length === 1) return matches[0];
    const list = others.map((c) => `${c.handle} ${c.agent?.name ?? c.agentId} · ${c.title || t('acp.store.newShort')}`).join('; ');
    throw new Error(
      matches.length
        ? t('acp.store.ambiguousTarget', { to, handles: matches.map((c) => c.handle).join(', '), list })
        : t('acp.store.missingTarget', { to, list: list || t('acp.store.usageNone') }),
    );
  }

  async refresh() {
    this.listError = null;
    try {
      this.agents = await agentApi.list();
      this.feedbackAvailable = await agentApi.feedbackAvailable().catch(() => false);
      // El backend no recuerda los límites: se le aplican al arrancar
      for (const [id, n] of Object.entries(this.processLimits)) void agentApi.setProcessLimit(id, n).catch(() => {});
    } catch (e) {
      this.listError = String(e);
    }
  }

  setDebugLog(value: boolean) {
    this.debugLog = value;
    save(DEBUG_LOG_KEY, value ? '1' : null);
    void agentApi
      .setDebugLog(value)
      .then((p) => (this.debugLogPath = p))
      .catch(() => {});
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

  /** Texto que se deja escrito (sin enviar) en la conversación enfocada, o en la próxima que se abra. */
  pendingDraft = $state<string | null>(null);

  /**
   * Prepara una pregunta al agente sobre lo que cambió en un archivo entre dos commits: abre el
   * panel, adjunta el archivo y deja la pregunta escrita para que el usuario la revise y la envíe.
   */
  askAboutRevisions(file: { path: string; name: string }, from: string, to: string, message?: string) {
    this.setPanelOpen(true);
    this.attach({ kind: 'file', path: file.path, name: file.name });
    const s = (h: string) => h.slice(0, 7);
    const how = t('acp.store.askRevisionsHow', { from, to });
    // Un solo commit: `from` es su padre (`<hash>^`); si el archivo nació en él, el diff es contra vacío
    this.pendingDraft =
      from === `${to}^`
        ? t('acp.store.askCommit', {
            name: file.name,
            commit: s(to),
            msgPart: message ? ` («${message}»)` : '',
            how,
          })
        : t('acp.store.askRevisions', {
            name: file.name,
            from: s(from),
            to: s(to),
            how,
          });
  }

  /** Adjunta (por ejemplo, la selección del editor) a la conversación enfocada. */
  attach(a: Attachment) {
    const c = this.focused;
    if (c) c.attach(a);
    else this.#waiting.push(a);
  }

  addConversation(c: Conversation) {
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
    this.addConversation(c);
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
    this.addConversation(c);
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
    this.byId(id)?.discard();
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

export const agentStore = new AgentStore();
