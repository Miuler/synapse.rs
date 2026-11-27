import { invokeTauri } from '@shared/api';
import type { BoardApi } from '../model/folder-board';

/** Error de las órdenes de un tablero de carpeta. */
export interface BoardError {
  kind: 'not_folder_board' | 'frontmatter' | 'sync' | 'task_id' | 'path' | 'conflict' | 'not_found' | 'io';
  detail: string;
}

export const boardsApi: BoardApi = {
  read: (path) => invokeTauri('board_read', { path }),
  write: (path, board, hash) => invokeTauri('board_write', { path, board, hash }),
  readTask: (path, id) => invokeTauri('board_task_read', { path, id }),
  writeTask: (path, id, task, hash) => invokeTauri('board_task_write', { path, id, task, hash }),
  deleteTask: (path, id, hash) => invokeTauri('board_task_delete', { path, id, hash }),
};
