// Texto de una línea al marcar o desmarcar su casilla de tarea (sin dependencias, para probarlo aislado).

const TASK = /^(\s*(?:[-*+]|\d+[.)])\s+)\[([ xX])\]/;
const ITEM = /^(\s*(?:[-*+]|\d+[.)])\s+)/;

/**
 * Nuevo texto de la línea: `- [ ]` ↔ `- [x]`; un elemento de lista sin casilla la gana; una línea
 * normal pasa a ser una tarea (`- [ ] texto`, conservando la sangría).
 */
export function toggleTaskLine(line: string): string {
  const task = TASK.exec(line);
  if (task) {
    const mark = task[2] === ' ' ? 'x' : ' ';
    return `${task[1]}[${mark}]${line.slice(task[0].length)}`;
  }
  const item = ITEM.exec(line);
  if (item) return `${item[1]}[ ] ${line.slice(item[0].length)}`;
  const indent = /^\s*/.exec(line)![0];
  return `${indent}- [ ] ${line.slice(indent.length)}`;
}
