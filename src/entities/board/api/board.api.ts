import { invokeTauri } from '@shared/api';
import type { ReferencedBoard } from '../model/query';

export const referencedBoards = {
  /** Los datos guardados de un tablero de carpeta; nunca se pide nada al proveedor. */
  read: (path: string) => invokeTauri<ReferencedBoard>('board_read', { path }),
};
