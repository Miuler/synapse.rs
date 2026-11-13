import { agentApi, listenAgentEvents } from '../api/agent.api';
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

type AgentCommandList = AgentCommand[];

const FLOATING_KEY = 'synapse.agent.floating';
const FEEDBACK_KEY = 'synapse.agent.feedback';

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

/** Estado de la conversación con el agente activo. */
class AgentStore {
  agents = $state<AgentProfile[]>([]);
  /** Ningún agente elegido al arrancar: el usuario lo elige, o lo fija la conversación que retoma. */
  activeAgentId = $state<string | null>(null);
  session = $state<AgentSession | null>(null);
  connection = $state<AgentConnection>('idle');
  items = $state<ChatItem[]>([]);
  listError = $state<string | null>(null);
  floating = $state(load(FLOATING_KEY) === '1');
  /** Modo feedback: el agente anota lo que le falta al MCP de Synapse (aplica a nuevas sesiones). */
  feedbackMode = $state(load(FEEDBACK_KEY) === '1');
  /** Plan del agente para el turno actual. */
  plan = $state<PlanEntry[]>([]);
  /** Comandos que anuncia el agente (para autocompletar con `/`). */
  commands = $state<AgentCommand[]>([]);
  usage = $state<{ used: number; size: number; cost: { amount: number; currency: string } | null } | null>(null);
  currentMode = $state<string | null>(null);
  currentModel = $state<string | null>(null);
  /** Etiquetas que acompañan al próximo mensaje. */
  attachments = $state<Attachment[]>([]);
  /** Nota activa del editor; se adjunta sola salvo que el usuario la quite. */
  activeNote = $state<{ path: string; name: string } | null>(null);
  dismissedActiveNote = $state<string | null>(null);
  /** Conversación actual en el historial de la bóveda y conversaciones guardadas. */
  transcriptId = $state<string | null>(null);
  history = $state<AgentTranscriptSummary[]>([]);

  activeAgent = $derived(this.agents.find((a) => a.id === this.activeAgentId) ?? null);
  pendingPermissions = $derived(this.items.filter((i) => i.type === 'permission' && i.answered === null).length);

  #nextId = 1;
  #early = new Map<string, AgentCommandList>();
  #listening: Promise<unknown> | null = null;
  #createdAt = 0;
  /** Conversación anterior que el agente no pudo retomar: se le pasa como contexto en el próximo mensaje. */
  #replay: string | null = null;
  #saveTimer: ReturnType<typeof setTimeout> | null = null;

  /** Guarda la conversación en la bóveda (agrupa cambios seguidos). */
  #persist(now = false) {
    if (this.#saveTimer) clearTimeout(this.#saveTimer);
    this.#saveTimer = null;
    const run = () => {
      const cwd = this.session?.cwd ?? this.#cwd;
      const agent = this.activeAgent;
      if (!cwd || !agent || !this.transcriptId || !this.items.some((i) => i.type === 'user')) return;
      const first = this.items.find((i) => i.type === 'user');
      const title = (first && 'text' in first ? first.text : '').replace(/\s+/g, ' ').trim().slice(0, 80);
      const transcript = {
        id: this.transcriptId,
        agent_id: agent.id,
        agent_name: agent.name,
        cwd,
        title,
        acp_session_id: this.session?.id ?? this.#acpSessionId,
        created_at: this.#createdAt,
        updated_at: Date.now(),
        items: $state.snapshot(this.items) as ChatItem[],
      };
      agentApi
        .saveTranscript(transcript)
        .then(() => this.loadHistory(cwd))
        .catch(() => {});
    };
    if (now) run();
    else this.#saveTimer = setTimeout(run, 800);
  }
  #cwd: string | null = null;
  #acpSessionId: string | null = null;

  #push(item: DistributiveOmit<ChatItem, 'id'>) {
    this.items.push({ ...item, id: this.#nextId++ } as ChatItem);
  }

  #error(e: unknown) {
    this.#push({ type: 'error', text: e instanceof Error ? e.message : String(e) });
  }

  #listen() {
    this.#listening ??= listenAgentEvents({
      onPlan: (e) => {
        if (e.sessionId === this.session?.id) this.plan = e.entries;
      },
      onCommands: (e) => {
        // Puede llegar antes de que termine `startSession`: se guarda hasta conocer la sesión
        if (e.sessionId === this.session?.id) this.commands = e.commands;
        else if (!this.session) this.#early.set(e.sessionId, e.commands);
      },
      onUsage: (e) => {
        if (e.sessionId === this.session?.id && e.used != null && e.size) this.usage = { used: e.used, size: e.size, cost: e.cost ?? null };
      },
      onMode: (e) => {
        if (e.sessionId === this.session?.id) this.currentMode = e.modeId;
      },
      onStream: (e) => {
        if (e.sessionId !== this.session?.id || e.kind === 'user') return;
        const type = e.kind === 'thought' ? 'thought' : 'agent';
        const last = this.items.at(-1);
        if (last && last.type === type) last.text += e.text;
        else this.#push({ type, text: e.text });
      },
      onToolCall: (e) => {
        if (e.sessionId !== this.session?.id) return;
        const existing = this.items.find((i) => i.type === 'tool' && i.toolCallId === e.toolCallId);
        if (existing && existing.type === 'tool') {
          if (e.title) existing.title = e.title;
          if (e.kind) existing.kind = e.kind;
          if (e.status) existing.status = e.status;
          if (e.locations?.length) existing.locations = e.locations;
          if (e.content?.length) existing.content = e.content;
          if (e.rawInput != null) existing.rawInput = e.rawInput;
          if (e.rawOutput != null) existing.rawOutput = e.rawOutput;
        } else {
          this.#push({
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
        if (e.sessionId !== this.session?.id) return;
        this.#push({ type: 'permission', request: e, answered: null });
      },
      onStatus: (e) => {
        if (e.sessionId !== this.session?.id) return;
        this.session = null;
        this.connection = 'idle';
        this.#push({ type: 'error', text: 'El agente terminó.' });
      },
    });
    return this.#listening;
  }

  async refresh() {
    this.listError = null;
    try {
      this.agents = await agentApi.list();
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

  /** Cambia de agente: cierra la sesión anterior y abre otra sobre `cwd`. */
  async select(agentId: string, cwd: string) {
    if (this.session) await this.stop();
    this.activeAgentId = agentId;
    await this.connectLatest(cwd);
  }

  /** Abre una conversación nueva, o retoma la guardada `resumeId` de esta bóveda. */
  async connect(cwd: string, resumeId: string | null = null) {
    if (this.session) return;
    await this.#listen();
    this.#cwd = cwd;
    this.items = [];
    this.plan = [];
    this.commands = [];
    this.usage = null;
    this.#replay = null;
    this.#acpSessionId = null;
    this.transcriptId = crypto.randomUUID();
    this.#createdAt = Date.now();
    let resume: string | null = null;
    if (resumeId) {
      try {
        const t = await agentApi.loadTranscript(cwd, resumeId);
        // La conversación recuerda su agente: retomarla cambia a ese agente
        if (!this.agents.some((a) => a.id === t.agent_id)) {
          throw new Error(`El agente «${t.agent_name || t.agent_id}» de esta conversación ya no está configurado.`);
        }
        this.activeAgentId = t.agent_id;
        this.items = Array.isArray(t.items) ? t.items : [];
        this.#nextId = this.items.reduce((m, i) => Math.max(m, i.id), 0) + 1;
        this.transcriptId = t.id;
        this.#createdAt = t.created_at;
        this.#acpSessionId = t.acp_session_id;
        resume = t.acp_session_id;
      } catch (e) {
        this.#error(e);
      }
    }
    const agent = this.activeAgent;
    if (!agent) return;
    try {
      if (!agent.installed) {
        this.connection = 'installing';
        await agentApi.install(agent.id);
        await this.refresh();
      }
      this.connection = 'starting';
      this.session = await agentApi.startSession(agent.id, cwd, this.feedbackMode, resume);
      if (resumeId && !this.session.resumed && this.items.length) this.#replay = replayText(this.items);
      // Modelo recordado para este agente: no hay que elegirlo en cada conversación nueva
      const savedModel = load(`synapse.agent.model.${agent.id}`);
      if (savedModel && savedModel !== this.currentModel && this.session.models.some((m) => m.id === savedModel)) {
        await this.setModel(savedModel);
      }
      this.currentMode = this.session.current_mode;
      this.currentModel = this.session.current_model;
      this.commands = this.#early.get(this.session.id) ?? [];
      this.#early.clear();
      this.connection = 'ready';
      if (resumeId) this.#persist(true);
    } catch (e) {
      this.connection = 'error';
      this.#error(e);
    }
    this.loadHistory(cwd);
  }

  /** Conecta retomando la última conversación guardada de la bóveda, si hay alguna. */
  async connectLatest(cwd: string) {
    if (this.session) return;
    await this.loadHistory(cwd);
    const last = this.history.find((h) => h.agent_id === this.activeAgentId) ?? null;
    await this.connect(cwd, last?.id ?? null);
  }

  /** Cierra la conversación actual y abre otra vacía. */
  async newSession(cwd: string) {
    await this.stop();
    await this.connect(cwd);
  }

  /** Retoma una conversación guardada. */
  async resume(cwd: string, id: string) {
    if (id === this.transcriptId && this.session) return;
    await this.stop();
    await this.connect(cwd, id);
  }

  async loadHistory(cwd: string) {
    try {
      this.history = await agentApi.listTranscripts(cwd);
    } catch {
      this.history = [];
    }
  }

  async deleteTranscript(cwd: string, id: string) {
    try {
      await agentApi.deleteTranscript(cwd, id);
      if (id === this.transcriptId) this.transcriptId = crypto.randomUUID();
    } catch (e) {
      this.#error(e);
    }
    await this.loadHistory(cwd);
  }

  async deleteAllTranscripts(cwd: string) {
    try {
      await agentApi.deleteAllTranscripts(cwd);
      this.transcriptId = crypto.randomUUID();
    } catch (e) {
      this.#error(e);
    }
    await this.loadHistory(cwd);
  }

  /** Abre el archivo crudo (JSON) de la conversación actual con la aplicación por defecto. */
  async openRaw() {
    const cwd = this.session?.cwd ?? this.#cwd;
    if (!cwd || !this.transcriptId) return;
    this.#persist(true);
    try {
      await new Promise((r) => setTimeout(r, 150));
      await agentApi.openTranscript(cwd, this.transcriptId);
    } catch (e) {
      this.#error(e);
    }
  }

  /** Etiquetas que viajarán con el próximo mensaje (incluida la nota activa si no se quitó). */
  get pendingAttachments(): Attachment[] {
    const note = this.activeNote;
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
    const note = this.activeNote;
    if (!note) return;
    if (this.dismissedActiveNote === note.path) this.dismissedActiveNote = null;
    this.attach({ kind: 'file', path: note.path, name: note.name });
  }

  detach(a: Attachment) {
    if (a.kind === 'file' && this.activeNote?.path === a.path && !this.attachments.includes(a)) {
      this.dismissedActiveNote = a.path;
      return;
    }
    this.attachments = this.attachments.filter((x) => x !== a);
  }

  async send(text: string) {
    const session = this.session;
    if (!session || this.connection === 'busy' || !text.trim()) return;
    if (/(^|\s)@this\b/.test(text)) this.attachActiveNote();
    const attachments = this.pendingAttachments;
    const blocks: PromptBlock[] = [];
    if (this.#replay) {
      blocks.push({ type: 'text', text: this.#replay });
      this.#replay = null;
    }
    blocks.push({ type: 'text', text });
    for (const a of attachments) {
      blocks.push(
        a.kind === 'file'
          ? { type: 'file', path: a.path, name: a.name }
          : a.kind === 'image'
            ? { type: 'image', mime_type: a.mimeType, data: a.data }
            : { type: 'selection', path: a.path, text: a.text, start_line: a.startLine, end_line: a.endLine },
      );
    }
    this.#push({ type: 'user', text, attachments });
    this.attachments = [];
    // Tras enviarla, la nota activa no se repite hasta que cambie
    if (this.activeNote) this.dismissedActiveNote = this.activeNote.path;
    this.plan = [];
    this.connection = 'busy';
    this.#persist(true);
    try {
      const stop = await agentApi.sendPrompt(session.id, blocks);
      if (stop === 'refusal') this.#push({ type: 'error', text: 'El agente rechazó la petición.' });
    } catch (e) {
      this.#error(e);
    } finally {
      if (this.session?.id === session.id) this.connection = 'ready';
      this.#persist(true);
    }
  }

  setActiveNote(note: { path: string; name: string } | null) {
    if (note?.path === this.activeNote?.path) return;
    this.activeNote = note;
  }

  async setMode(modeId: string) {
    if (!this.session) return;
    const previous = this.currentMode;
    this.currentMode = modeId;
    try {
      await agentApi.setMode(this.session.id, modeId);
    } catch (e) {
      this.currentMode = previous;
      this.#error(e);
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
      this.#error(e);
    }
  }

  async cancel() {
    if (this.session) await agentApi.cancelPrompt(this.session.id).catch((e) => this.#error(e));
  }

  async answer(itemId: number, optionId: string | null) {
    const item = this.items.find((i) => i.id === itemId);
    if (!item || item.type !== 'permission' || item.answered !== null) return;
    item.answered = optionId ?? '';
    this.#persist();
    try {
      await agentApi.respondPermission(item.request.sessionId, item.request.requestId, optionId);
    } catch (e) {
      this.#error(e);
    }
  }

  /**
   * La ventana abrió otra bóveda: se cierra la conversación (queda guardada en la bóveda anterior)
   * y el panel vuelve a empezar sin agente elegido, con el historial de la nueva bóveda.
   */
  async changeVault(cwd: string | null) {
    const current = this.session?.cwd ?? this.#cwd;
    if (current === cwd) return;
    if (this.session) await this.stop();
    this.#cwd = cwd;
    this.activeAgentId = null;
    this.items = [];
    this.plan = [];
    this.commands = [];
    this.usage = null;
    this.attachments = [];
    this.activeNote = null;
    this.dismissedActiveNote = null;
    this.transcriptId = null;
    this.#replay = null;
    this.#acpSessionId = null;
    this.history = [];
    this.connection = 'idle';
    if (cwd) await this.loadHistory(cwd);
  }

  async stop() {
    const session = this.session;
    if (session) {
      this.#acpSessionId = session.id;
      this.#persist(true);
    }
    this.session = null;
    this.connection = 'idle';
    if (session) await agentApi.stopSession(session.id).catch(() => {});
  }
}

/** Conversación anterior en texto, para un agente que no puede retomarla por sí mismo. */
function replayText(items: ChatItem[]): string {
  const lines: string[] = [];
  for (const i of items) {
    if (i.type === 'user') lines.push(`Usuario: ${i.text}`);
    else if (i.type === 'agent') lines.push(`Agente: ${i.text}`);
    else if (i.type === 'tool') lines.push(`(herramienta: ${i.title})`);
  }
  const body = lines.join('\n\n').slice(-20000);
  return `Contexto: esta conversación continúa una anterior. Resumen de lo dicho hasta ahora:\n\n${body}\n\n— Fin del contexto anterior —`;
}

export const agentStore = new AgentStore();
