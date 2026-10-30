import { EditorState, Prec, type ChangeSpec } from '@codemirror/state';
import { EditorView, keymap } from '@codemirror/view';
import { StreamLanguage } from '@codemirror/language';

interface CommentTokens {
  line?: string;
  block?: { open: string; close: string };
}

/** Tokens de comentario del lenguaje en `pos`; si no hay, usa `<!-- -->` como en Markdown. */
function tokensAt(state: EditorState, pos: number): CommentTokens {
  const found = state.languageDataAt<CommentTokens>('commentTokens', pos)[0];
  return found && (found.line || found.block)
    ? found
    : { block: { open: '<!--', close: '-->' } };
}

/** Define los tokens de comentario de un StreamLanguage (que no los trae por defecto). */
export function withCommentTokens(language: StreamLanguage<unknown>, tokens: CommentTokens) {
  return [language, language.data.of({ commentTokens: tokens })];
}

/**
 * Comenta/descomenta todas las líneas tocadas por la selección (o la línea del cursor).
 * Si todas las líneas con contenido ya están comentadas las descomenta; si no, comenta todas.
 */
export function toggleLinesComment(view: EditorView): boolean {
  const { state } = view;
  if (state.readOnly) return false;

  const lineNumbers = new Set<number>();
  for (const r of state.selection.ranges) {
    const first = state.doc.lineAt(r.from).number;
    let last = state.doc.lineAt(r.to).number;
    // Selección que termina justo al inicio de una línea: esa línea no cuenta
    if (!r.empty && last > first && state.doc.line(last).from === r.to) last--;
    for (let n = first; n <= last; n++) lineNumbers.add(n);
  }
  const lines = [...lineNumbers].sort((a, b) => a - b).map((n) => state.doc.line(n));
  const withText = lines.filter((l) => l.text.trim().length > 0);
  const targets = withText.length ? withText : lines;

  const tokens = tokensAt(state, targets[0].from);
  const open = tokens.line ?? tokens.block!.open;
  const close = tokens.line ? '' : tokens.block!.close;

  const isCommented = (text: string) => {
    const t = text.trim();
    return t.startsWith(open) && (!close || t.endsWith(close));
  };
  const uncomment = withText.length > 0 && withText.every((l) => isCommented(l.text));
  const changes: ChangeSpec[] = [];

  if (uncomment) {
    for (const l of withText) {
      const start = l.from + (l.text.length - l.text.trimStart().length);
      const openLen = open.length + (l.text.startsWith(' ', start - l.from + open.length) ? 1 : 0);
      changes.push({ from: start, to: start + openLen });
      if (close) {
        const end = l.from + l.text.trimEnd().length;
        const closeLen = close.length + (l.text[end - l.from - close.length - 1] === ' ' ? 1 : 0);
        changes.push({ from: end - closeLen, to: end });
      }
    }
  } else {
    // Los comentarios de línea se alinean a la sangría mínima del bloque
    const indent = Math.min(...targets.map((l) => l.text.length - l.text.trimStart().length));
    for (const l of targets) {
      if (!l.text.trim() && targets.length > 1 && withText.length) continue;
      if (close) {
        const start = l.from + (l.text.length - l.text.trimStart().length);
        changes.push({ from: start, insert: `${open} ` });
        changes.push({ from: l.from + l.text.trimEnd().length, insert: ` ${close}` });
      } else {
        changes.push({ from: l.from + indent, insert: `${open} ` });
      }
    }
  }

  view.dispatch({ changes, userEvent: 'input.comment' });
  return true;
}

/** Atajo reutilizable Ctrl+/ (Cmd+/ en macOS) para cualquier editor CodeMirror. */
export function commentExtension() {
  return Prec.highest(keymap.of([{ key: 'Mod-/', run: toggleLinesComment, preventDefault: true }]));
}
