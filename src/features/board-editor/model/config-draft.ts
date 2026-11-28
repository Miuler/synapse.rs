// La configuración que el agente puede ver y cambiar: la última que se abrió en esta ventana. Solo
// la lee y la cambia en pantalla; guardarla es cosa del usuario.
import { t } from '@shared/lib/i18n';
import { listenDraftChanges, publishDraft, replyDraftChange } from '../api/config-draft.api';
import type { Criterion, IssueFilter } from './issue-sync';

export type QueryMode = 'mine' | 'criteria' | 'custom';

/** El borrador tal como lo entiende Rust (`SourceDraft`). */
export interface SourceDraft {
  provider: string;
  connection: string;
  scope: string;
  mode: QueryMode;
  query: string;
  filter: IssueFilter;
  states: Record<string, string[]>;
  columns: { id: string; title: string }[];
  /** Ids de las tareas vinculadas en este proyecto. */
  linked: string[];
  /** `AAAA-MM-DD`: decide la iteración en curso. */
  today: string;
}

export interface DraftChange {
  query?: string;
  filter?: Partial<Record<Criterion, string[]>>;
  states?: Record<string, string[]>;
}

interface OpenDraft {
  /** Cambia lo que se ve y devuelve cómo quedó. */
  apply: (change: DraftChange) => SourceDraft;
  draft: SourceDraft | null;
}

const open: OpenDraft[] = [];
let listening: Promise<unknown> | null = null;

const send = () =>
  publishDraft(open.at(-1)?.draft ?? null).catch((error) => console.warn('No se pudo anunciar la configuración abierta', error));

function listen() {
  listening ??= listenDraftChanges((requestId, change) => {
    const current = open.at(-1);
    if (!current) return void replyDraftChange(requestId, null, t('boardDraft.closedError'));
    try {
      current.draft = current.apply(change);
      void replyDraftChange(requestId, current.draft, null);
    } catch (error) {
      void replyDraftChange(requestId, null, String(error));
    }
  }).catch(() => (listening = null));
}

/** Registra una configuración abierta: se publica cada vez que cambia, hasta cerrarla. */
export function openConfigDraft(apply: OpenDraft['apply']) {
  const entry: OpenDraft = { apply, draft: null };
  open.push(entry);
  listen();
  return {
    publish(draft: SourceDraft) {
      entry.draft = draft;
      if (open.at(-1) === entry) void send();
    },
    close() {
      const at = open.indexOf(entry);
      if (at < 0) return;
      open.splice(at, 1);
      void send();
    },
  };
}

/** Al cargar la ventana: lo que quedara anunciado de antes de recargarla ya no está abierto. */
export function resetConfigDraft() {
  if (!open.length) void send();
}
