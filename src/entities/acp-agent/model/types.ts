export type AgentSource = 'verified' | 'custom';

export interface AgentLaunch {
  command: string;
  args: string[];
  env: Record<string, string>;
}

export interface AgentProfile {
  id: string;
  name: string;
  source: AgentSource;
  description?: string | null;
  version?: string | null;
  installed: boolean;
  launch?: AgentLaunch | null;
}

export interface AgentCapabilities {
  image: boolean;
  embedded_context: boolean;
  tools_over_http: boolean;
  load_session: boolean;
}

export interface AgentOption {
  id: string;
  name: string;
  description?: string | null;
}

export interface AgentSession {
  id: string;
  agent_id: string;
  cwd: string;
  capabilities: AgentCapabilities;
  modes: AgentOption[];
  current_mode: string | null;
  models: AgentOption[];
  current_model: string | null;
  tool_servers: string[];
  /** El agente retomó la sesión anterior y conserva su historial. */
  resumed: boolean;
}

/** Conversación guardada en `<bóveda>/.synapse/agent-sessions/`. */
export interface AgentTranscriptSummary {
  id: string;
  agent_id: string;
  agent_name: string;
  cwd: string;
  title: string;
  acp_session_id: string | null;
  created_at: number;
  updated_at: number;
}

export interface AgentTranscript extends AgentTranscriptSummary {
  items: ChatItem[];
}

export interface ToolCallTelemetry {
  tool_call_id: string;
  title: string;
  kind?: string | null;
  status?: string | null;
  started_at?: number | null;
  ended_at?: number | null;
  duration_ms?: number | null;
}

export interface IterationTelemetry {
  id: string;
  turn_index: number;
  model?: string | null;
  mode?: string | null;
  started_at: number;
  ended_at: number;
  duration_ms: number;
  stop_reason: string;
  usage_used?: number | null;
  usage_size?: number | null;
  cost_amount?: number | null;
  cost_delta?: number | null;
  cost_currency?: string | null;
  tool_calls: ToolCallTelemetry[];
  mcp_call_ids: string[];
}

export interface RecordIterationPayload {
  cwd: string;
  conversation_id: string;
  agent_id: string;
  agent_name: string;
  acp_session_id: string | null;
  iteration: IterationTelemetry;
}

/** Parte de un mensaje al agente (coincide con `PromptBlock` del backend). */
export type PromptBlock =
  | { type: 'text'; text: string }
  | { type: 'file'; path: string; name: string }
  | { type: 'selection'; path: string; text: string; start_line: number; end_line: number }
  | { type: 'image'; mime_type: string; data: string };

/** Adjunto visible como etiqueta en el cuadro de texto. */
export type Attachment =
  | { kind: 'file'; path: string; name: string }
  | { kind: 'selection'; path: string; name: string; text: string; startLine: number; endLine: number }
  /** Imagen pegada (`path` es un identificador local, `data` va en base64 sin prefijo). */
  | { kind: 'image'; path: string; name: string; mimeType: string; data: string };

export interface PlanEntry {
  content: string;
  priority?: string;
  status: 'pending' | 'in_progress' | 'completed' | string;
}

export interface AgentCommand {
  name: string;
  description?: string;
}

export interface ToolContent {
  type: 'diff' | 'text';
  path?: string;
  oldText?: string | null;
  newText?: string;
  text?: string;
}

export interface PermissionOption {
  id: string;
  name: string;
  kind: string;
}

export interface PermissionRequestEvent {
  sessionId: string;
  requestId: string;
  title: string;
  toolCallId?: string | null;
  options: PermissionOption[];
}

export interface StreamEvent {
  sessionId: string;
  kind: 'message' | 'thought' | 'user';
  text: string;
}

export interface ToolCallEvent {
  sessionId: string;
  toolCallId: string;
  title?: string | null;
  kind?: string | null;
  status?: string | null;
  locations?: { path: string; line?: number }[] | null;
  content?: ToolContent[] | null;
  /** Entrada y salida crudas de la herramienta, tal como las envía el agente. */
  rawInput?: unknown;
  rawOutput?: unknown;
}

export interface PlanEvent {
  sessionId: string;
  entries: PlanEntry[];
}

export interface CommandsEvent {
  sessionId: string;
  commands: AgentCommand[];
}

export interface UsageEvent {
  sessionId: string;
  used: number | null;
  size: number | null;
  /** Costo acumulado de la sesión, si el agente lo informa. */
  cost?: { amount: number; currency: string } | null;
}

export interface ModeEvent {
  sessionId: string;
  modeId: string;
}

export interface StatusEvent {
  sessionId: string;
  agentId: string;
  status: 'stopped';
}

/** Resumen de una conversación abierta (propia o de otra bóveda, por `/peer`). */
export interface ConversationSummary {
  handle: string;
  agent: string;
  model?: string;
  title: string;
  status: 'working' | 'permission' | 'idle' | 'error';
  pending_messages?: number;
  number?: number;
  last_reply?: string;
  last_question?: string;
  last_activity?: number;
}

/** Mención en un mensaje: `#2` (conversación de esta ventana), `@Bóveda` o `@Bóveda#2`. */
/** Mensaje del usuario en cola (escrito mientras el agente trabajaba). */
export interface QueuedMessage {
  id: number;
  text: string;
  /** Texto que se envía al agente si difiere del visible (menciones resueltas) */
  prompt?: string;
  mentions: Mention[];
  attachments: Attachment[];
}

export interface Mention {
  token: string;
  /** Qué es, para el tooltip y para explicárselo al agente. */
  title: string;
  /** Valor de `to` para `send_to_conversation`. */
  to: string;
}

/** Elemento de la conversación tal como se muestra. */
export type ChatItem =
  | {
      type: 'user';
      id: number;
      text: string;
      attachments: Attachment[];
      /** Mensaje de otra conversación de agente (no del usuario). */
      from?: { handle: string; agent: string; title: string; vault?: string };
      /** Menciones a otras conversaciones o bóvedas (`#2`, `@Bóveda`), con lo que eran al enviarlas. */
      mentions?: Mention[];
    }
  | { type: 'agent'; id: number; text: string }
  | { type: 'thought'; id: number; text: string }
  | {
      type: 'tool';
      id: number;
      toolCallId: string;
      title: string;
      kind: string | null;
      status: string | null;
      locations: { path: string; line?: number }[];
      content: ToolContent[];
      rawInput?: unknown;
      rawOutput?: unknown;
    }
  | { type: 'permission'; id: number; request: PermissionRequestEvent; answered: string | null }
  | { type: 'error'; id: number; text: string }
  /** El usuario compactó el contexto del agente (`/compact`): lo anterior ya no está en su memoria. */
  | { type: 'compact'; id: number }
  /** Información de Synapse en el chat (no la escribe el agente), por ejemplo `/usage`. */
  | { type: 'info'; id: number; text: string };

export type AgentConnection = 'idle' | 'installing' | 'starting' | 'ready' | 'busy' | 'error';
