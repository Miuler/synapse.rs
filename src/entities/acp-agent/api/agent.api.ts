import { invokeTauri } from '@shared/api';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  AgentProfile,
  AgentSession,
  AgentTranscript,
  AgentTranscriptSummary,
  CommandsEvent,
  ModeEvent,
  PermissionRequestEvent,
  PlanEvent,
  PromptBlock,
  UsageEvent,
  StatusEvent,
  StreamEvent,
  ToolCallEvent,
} from '../model/types';

export const agentApi = {
  list: () => invokeTauri<AgentProfile[]>('agent_list'),
  install: (agentId: string) => invokeTauri<void>('agent_install', { agentId }),
  saveProfile: (p: { id: string; name: string; command: string; args: string[]; env: Record<string, string> }) =>
    invokeTauri<AgentProfile>('agent_save_profile', p),
  deleteProfile: (id: string) => invokeTauri<void>('agent_delete_profile', { id }),
  startSession: (agentId: string, cwd: string, feedback: boolean, resume: string | null = null, conversationId: string | null = null) =>
    invokeTauri<AgentSession>('agent_start_session', { agentId, cwd, feedback, resume, conversationId }),
  saveTranscript: (transcript: AgentTranscript) => invokeTauri<void>('agent_save_transcript', { transcript }),
  listTranscripts: (cwd: string) => invokeTauri<AgentTranscriptSummary[]>('agent_list_transcripts', { cwd }),
  loadTranscript: (cwd: string, id: string) => invokeTauri<AgentTranscript>('agent_load_transcript', { cwd, id }),
  deleteTranscript: (cwd: string, id: string) => invokeTauri<void>('agent_delete_transcript', { cwd, id }),
  deleteAllTranscripts: (cwd: string) => invokeTauri<void>('agent_delete_all_transcripts', { cwd }),
  openTranscript: (cwd: string, id: string) => invokeTauri<void>('agent_open_transcript', { cwd, id }),
  sendPrompt: (sessionId: string, blocks: PromptBlock[]) => invokeTauri<string>('agent_send_prompt', { sessionId, blocks }),
  setMode: (sessionId: string, modeId: string) => invokeTauri<void>('agent_set_mode', { sessionId, modeId }),
  setModel: (sessionId: string, modelId: string) => invokeTauri<void>('agent_set_model', { sessionId, modelId }),
  cancelPrompt: (sessionId: string) => invokeTauri<void>('agent_cancel_prompt', { sessionId }),
  respondPermission: (sessionId: string, requestId: string, optionId: string | null) =>
    invokeTauri<void>('agent_respond_permission', { sessionId, requestId, optionId }),
  /** Conversaciones de las otras instancias de Synapse (otras bóvedas abiertas). */
  peerList: (request: unknown) =>
    invokeTauri<{ vault: string; path: string; result: unknown; error: string | null }[]>('peer_list', { request }),
  /** Pedido a la instancia de otra bóveda (por nombre o ruta). */
  peerSend: (vault: string, request: unknown) => invokeTauri<unknown>('peer_send', { vault, request }),
  /** Bóveda de esta instancia, para presentarse ante las demás. */
  peerIdentity: () => invokeTauri<{ vault: string; path: string } | null>('peer_identity'),
  setProcessLimit: (agentId: string, limit: number) => invokeTauri<void>('agent_set_process_limit', { agentId, limit }),
  feedbackAvailable: () => invokeTauri<boolean>('agent_feedback_available'),
  /** Activa o desactiva el registro de depuración de ACP; devuelve la ruta del archivo. */
  setDebugLog: (enabled: boolean) => invokeTauri<string>('agent_set_debug_log', { enabled }),
  stopSession: (sessionId: string) => invokeTauri<void>('agent_stop_session', { sessionId }),
};

export interface AgentEventHandlers {
  onStream: (e: StreamEvent) => void;
  onToolCall: (e: ToolCallEvent) => void;
  onPermission: (e: PermissionRequestEvent) => void;
  onStatus: (e: StatusEvent) => void;
  onPlan: (e: PlanEvent) => void;
  onCommands: (e: CommandsEvent) => void;
  onUsage: (e: UsageEvent) => void;
  onMode: (e: ModeEvent) => void;
}

/** Escucha los eventos de los agentes; devuelve la función para dejar de escuchar. */
export async function listenAgentEvents(h: AgentEventHandlers): Promise<UnlistenFn> {
  const offs = await Promise.all([
    listen<StreamEvent>('acp://stream', (e) => h.onStream(e.payload)),
    listen<ToolCallEvent>('acp://tool-call', (e) => h.onToolCall(e.payload)),
    listen<PermissionRequestEvent>('acp://permission-request', (e) => h.onPermission(e.payload)),
    listen<StatusEvent>('acp://status', (e) => h.onStatus(e.payload)),
    listen<PlanEvent>('acp://plan', (e) => h.onPlan(e.payload)),
    listen<CommandsEvent>('acp://commands', (e) => h.onCommands(e.payload)),
    listen<UsageEvent>('acp://usage', (e) => h.onUsage(e.payload)),
    listen<ModeEvent>('acp://mode', (e) => h.onMode(e.payload)),
  ]);
  return () => offs.forEach((off) => off());
}

export interface ConversationRequest {
  requestId: string;
  action: string;
  caller: string | null;
  to?: string;
  message?: string;
  expectsReply?: boolean;
  /** Pedido de otra instancia (`action: "peer"`). */
  request?: unknown;
}

/** Atiende las peticiones del MCP sobre las conversaciones abiertas y devuelve el resultado al backend. */
export function listenConversationRequests(handle: (req: ConversationRequest) => unknown): Promise<UnlistenFn> {
  return listen<ConversationRequest>('mcp://conversations', async ({ payload }) => {
    let result: unknown = null;
    let error: string | null = null;
    try {
      result = await handle(payload);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
    invokeTauri('mcp_conversations_reply', { requestId: payload.requestId, result, error }).catch(() => {});
  });
}
