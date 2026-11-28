// Dónde hay bloques `board` en una nota. Se siguen todas las vallas de código: un ejemplo escrito
// dentro de otro bloque no es un tablero.

export interface BoardBlock {
  /** Posición del primer carácter de la valla que abre y del último de la que cierra. */
  from: number;
  to: number;
  /** Desde `from` hasta el contenido: ahí va el cursor al editar. */
  codeOffset: number;
  code: string;
}

const FENCE = /^ {0,3}(`{3,}|~{3,})\s*([^`]*)$/;

export function findBoardBlocks(text: string): BoardBlock[] {
  const blocks: BoardBlock[] = [];
  let open: { mark: string; board: boolean; from: number; codeFrom: number; lines: string[] } | null = null;
  let from = 0;
  for (const line of text.split('\n')) {
    const to = from + line.length;
    const fence = line.match(FENCE);
    if (!open) {
      if (fence) {
        const board = /^board(\s|$)/i.test(fence[2].trim());
        open = { mark: fence[1], board, from, codeFrom: to + 1, lines: [] };
      }
    } else if (fence && fence[1][0] === open.mark[0] && fence[1].length >= open.mark.length && !fence[2].trim()) {
      if (open.board) {
        const codeOffset = Math.min(open.codeFrom, from) - open.from;
        blocks.push({ from: open.from, to, codeOffset, code: open.lines.join('\n') });
      }
      open = null;
    } else {
      open.lines.push(line);
    }
    from = to + 1;
  }
  return blocks;
}
