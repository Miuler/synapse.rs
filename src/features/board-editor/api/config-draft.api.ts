import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { invokeTauri } from '@shared/api';
import type { DraftChange, SourceDraft } from '../model/config-draft';

/** La configuración abierta en esta ventana, para las herramientas `config_` del agente. */
export const publishDraft = (draft: SourceDraft | null) => invokeTauri<void>('mcp_config_draft', { draft });

export const replyDraftChange = (requestId: string, draft: SourceDraft | null, error: string | null) =>
  invokeTauri<void>('mcp_config_changed', { requestId, draft, error });

/** Los cambios que pide el agente a la configuración de esta ventana. */
export const listenDraftChanges = (handle: (requestId: string, change: DraftChange) => void) =>
  getCurrentWebviewWindow().listen<{ requestId: string; change: DraftChange }>('mcp://config-change', ({ payload }) =>
    handle(payload.requestId, payload.change),
  );
