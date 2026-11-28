// «Actualizar» desde una nota: el mismo «Traer» sobre un tablero que puede no estar abierto.
import { t } from '@shared/lib/i18n';
import { boardsApi, type BoardError } from '../api/boards.api';
import { issueProviders } from '../api/issues.api';
import { FolderBoard } from './folder-board-store.svelte';
import { issueErrorText, pullIssues } from './pull';

const boardError = (error: unknown) => {
  const { kind, detail } = error as Partial<BoardError>;
  return new Error(kind ? t(`board.errors.${kind}`, { detail: detail ?? '' }) : String(error));
};

/** Trae las tareas del tablero y devuelve qué cambió; los fallos llegan ya redactados. */
export async function refreshBoard(path: string): Promise<string> {
  const board = new FolderBoard(boardsApi, path);
  await board.load();
  if (board.error || !board.doc) throw boardError(board.error);
  const source = board.doc.source;
  let name = source?.provider ?? '';
  try {
    const declared = (await issueProviders()).find((p) => p.id === source?.provider);
    if (!source?.scope || !declared) throw { kind: 'unknown_provider', detail: name };
    name = declared.name;
    const messages = await pullIssues(source, declared, {
      read: () => board.doc!,
      write: (next) => board.write(next),
      readTask: (id) => board.readTask(id),
    });
    await board.settled();
    if (board.error) throw boardError(board.error);
    return messages.join(' ');
  } catch (error) {
    throw new Error(issueErrorText(error, name));
  }
}
