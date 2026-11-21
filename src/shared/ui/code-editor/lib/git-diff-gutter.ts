import { StateEffect, StateField, RangeSetBuilder, type Extension, type Text } from '@codemirror/state';
import { EditorView, GutterMarker, gutter, showTooltip, type Tooltip } from '@codemirror/view';

export type LineDiffKind = 'added' | 'modified' | 'deleted';

export interface LineDiffMark {
  /** 1-based line number in the current document. */
  line: number;
  kind: LineDiffKind;
}

const MAX_DIFF_LINES = 20_000;

function splitLines(text: string): string[] {
  if (text.length === 0) return [];
  return text.replace(/\r\n/g, '\n').split('\n');
}

type DiffOp = { type: 'equal' | 'delete' | 'insert'; count: number };

/**
 * Myers O(ND) line diff returning a sequence of edit operations (equal / delete / insert).
 * Falls back to empty ops if the file exceeds MAX_DIFF_LINES to keep editing responsive.
 */
function myersDiffOps(a: string[], b: string[]): DiffOp[] {
  const n = a.length;
  const m = b.length;
  if (n + m > MAX_DIFF_LINES) return [];
  if (n === 0 && m === 0) return [];
  if (n === 0) return [{ type: 'insert', count: m }];
  if (m === 0) return [{ type: 'delete', count: n }];

  // Trim common prefix and suffix to shrink the Myers search space dramatically.
  let prefix = 0;
  while (prefix < n && prefix < m && a[prefix] === b[prefix]) prefix++;

  let suffix = 0;
  while (suffix < n - prefix && suffix < m - prefix && a[n - 1 - suffix] === b[m - 1 - suffix]) {
    suffix++;
  }

  const midN = n - prefix - suffix;
  const midM = m - prefix - suffix;

  const ops: DiffOp[] = [];
  const pushOp = (type: DiffOp['type'], count: number) => {
    if (count <= 0) return;
    const last = ops[ops.length - 1];
    if (last && last.type === type) {
      last.count += count;
    } else {
      ops.push({ type, count });
    }
  };

  if (prefix > 0) pushOp('equal', prefix);

  if (midN === 0) {
    if (midM > 0) pushOp('insert', midM);
  } else if (midM === 0) {
    pushOp('delete', midN);
  } else {
    const max = midN + midM;
    const offset = max;
    const v = new Int32Array(2 * max + 1);
    const trace: Int32Array[] = [];

    let foundD = -1;
    for (let d = 0; d <= max; d++) {
      trace.push(v.slice());
      for (let k = -d; k <= d; k += 2) {
        let x: number;
        if (k === -d || (k !== d && v[offset + k - 1] < v[offset + k + 1])) {
          x = v[offset + k + 1];
        } else {
          x = v[offset + k - 1] + 1;
        }
        let y = x - k;
        while (x < midN && y < midM && a[prefix + x] === b[prefix + y]) {
          x++;
          y++;
        }
        v[offset + k] = x;
        if (x >= midN && y >= midM) {
          foundD = d;
          break;
        }
      }
      if (foundD !== -1) break;
    }

    // Backtrack edit script
    const midOps: DiffOp[] = [];
    let x = midN;
    let y = midM;
    for (let d = foundD; d > 0; d--) {
      const prevV = trace[d];
      const k = x - y;
      let prevK: number;
      if (k === -d || (k !== d && prevV[offset + k - 1] < prevV[offset + k + 1])) {
        prevK = k + 1;
      } else {
        prevK = k - 1;
      }
      const prevX = prevV[offset + prevK];
      const prevY = prevX - prevK;

      while (x > prevX && y > prevY) {
        midOps.push({ type: 'equal', count: 1 });
        x--;
        y--;
      }
      if (x === prevX) {
        midOps.push({ type: 'insert', count: 1 });
        y--;
      } else {
        midOps.push({ type: 'delete', count: 1 });
        x--;
      }
    }
    while (x > 0 && y > 0) {
      midOps.push({ type: 'equal', count: 1 });
      x--;
      y--;
    }

    for (let i = midOps.length - 1; i >= 0; i--) {
      pushOp(midOps[i].type, midOps[i].count);
    }
  }

  if (suffix > 0) pushOp('equal', suffix);
  return ops;
}

/**
 * Computes 1-based line diff marks for `currentText` compared against `baseText` (e.g. git HEAD).
 */
export function computeLineDiff(baseText: string, currentText: string): LineDiffMark[] {
  if (baseText === currentText) return [];

  const baseLines = splitLines(baseText);
  const currLines = splitLines(currentText);
  if (currLines.length === 0) return [];

  const ops = myersDiffOps(baseLines, currLines);
  const marks = new Map<number, LineDiffKind>();

  let currLine = 1; // 1-based line number in currentText
  let i = 0;

  while (i < ops.length) {
    const op = ops[i];
    if (op.type === 'equal') {
      currLine += op.count;
      i++;
      continue;
    }

    // Group adjacent delete + insert operations into a single hunk
    let delCount = 0;
    let insCount = 0;
    while (i < ops.length && ops[i].type !== 'equal') {
      if (ops[i].type === 'delete') delCount += ops[i].count;
      else insCount += ops[i].count;
      i++;
    }

    if (delCount > 0 && insCount > 0) {
      const modCount = Math.min(delCount, insCount);
      for (let j = 0; j < modCount; j++) {
        marks.set(currLine + j, 'modified');
      }
      for (let j = modCount; j < insCount; j++) {
        marks.set(currLine + j, 'added');
      }
      currLine += insCount;
    } else if (insCount > 0) {
      for (let j = 0; j < insCount; j++) {
        marks.set(currLine + j, 'added');
      }
      currLine += insCount;
    } else if (delCount > 0) {
      // Pure deletion: attach marker to the line just before the deletion (or line 1 if at top)
      const targetLine = Math.min(Math.max(1, currLine - 1), currLines.length);
      if (!marks.has(targetLine)) {
        marks.set(targetLine, 'deleted');
      }
    }
  }

  return Array.from(marks.entries())
    .sort((a, b) => a[0] - b[0])
    .map(([line, kind]) => ({ line, kind }));
}

/** Bloque de líneas seguidas que cambian respecto a Git, con el texto de antes y el de ahora. */
export interface DiffHunk {
  kind: LineDiffKind;
  /** Primera línea (1-based) del documento actual; en un borrado puro, la línea anterior al hueco */
  startLine: number;
  /** Líneas que ocupa en el documento actual (0 en un borrado puro) */
  newCount: number;
  oldLines: string[];
  newLines: string[];
}

/** Bloques de cambio de `currentText` respecto a `baseText` (para mostrar qué había antes). */
export function computeHunks(baseText: string, currentText: string): DiffHunk[] {
  if (baseText === currentText) return [];
  const baseLines = splitLines(baseText);
  const currLines = splitLines(currentText);
  if (currLines.length === 0) return [];
  const ops = myersDiffOps(baseLines, currLines);
  const hunks: DiffHunk[] = [];
  let oldLine = 0; // índice 0-based en baseLines
  let currLine = 1; // 1-based en currentText
  let i = 0;
  while (i < ops.length) {
    const op = ops[i];
    if (op.type === 'equal') {
      oldLine += op.count;
      currLine += op.count;
      i++;
      continue;
    }
    let delCount = 0;
    let insCount = 0;
    while (i < ops.length && ops[i].type !== 'equal') {
      if (ops[i].type === 'delete') delCount += ops[i].count;
      else insCount += ops[i].count;
      i++;
    }
    const oldLines = baseLines.slice(oldLine, oldLine + delCount);
    const newLines = currLines.slice(currLine - 1, currLine - 1 + insCount);
    if (insCount > 0) {
      hunks.push({ kind: delCount > 0 ? 'modified' : 'added', startLine: currLine, newCount: insCount, oldLines, newLines });
    } else {
      hunks.push({ kind: 'deleted', startLine: Math.min(Math.max(1, currLine - 1), currLines.length), newCount: 0, oldLines, newLines });
    }
    oldLine += delCount;
    currLine += insCount;
  }
  return hunks;
}

/** Tramo de una línea: igual o cambiado respecto a la otra versión */
export interface CharSegment {
  text: string;
  changed: boolean;
}

/**
 * Diferencias por carácter entre la línea de antes y la de ahora (para ver si cambió una sola letra).
 * Líneas muy largas: solo se separa el prefijo y el sufijo comunes.
 */
export function charDiff(before: string, after: string): { before: CharSegment[]; after: CharSegment[] } {
  const a = [...before];
  const b = [...after];
  const segs = () => [] as CharSegment[];
  const push = (out: CharSegment[], text: string, changed: boolean) => {
    if (!text) return;
    const last = out[out.length - 1];
    if (last && last.changed === changed) last.text += text;
    else out.push({ text, changed });
  };
  const outA = segs();
  const outB = segs();
  const ops = a.length + b.length <= 2_000 ? myersDiffOps(a, b) : [];
  if (!ops.length) {
    let p = 0;
    while (p < a.length && p < b.length && a[p] === b[p]) p++;
    let q = 0;
    while (q < a.length - p && q < b.length - p && a[a.length - 1 - q] === b[b.length - 1 - q]) q++;
    push(outA, a.slice(0, p).join(''), false);
    push(outB, b.slice(0, p).join(''), false);
    push(outA, a.slice(p, a.length - q).join(''), true);
    push(outB, b.slice(p, b.length - q).join(''), true);
    push(outA, a.slice(a.length - q).join(''), false);
    push(outB, b.slice(b.length - q).join(''), false);
    return { before: outA, after: outB };
  }
  let ia = 0;
  let ib = 0;
  for (const op of ops) {
    if (op.type === 'equal') {
      push(outA, a.slice(ia, ia + op.count).join(''), false);
      push(outB, b.slice(ib, ib + op.count).join(''), false);
      ia += op.count;
      ib += op.count;
    } else if (op.type === 'delete') {
      push(outA, a.slice(ia, ia + op.count).join(''), true);
      ia += op.count;
    } else {
      push(outB, b.slice(ib, ib + op.count).join(''), true);
      ib += op.count;
    }
  }
  return { before: outA, after: outB };
}

class GitDiffMarker extends GutterMarker {
  readonly kind: LineDiffKind;

  constructor(kind: LineDiffKind) {
    super();
    this.kind = kind;
  }

  eq(other: GutterMarker): boolean {
    return other instanceof GitDiffMarker && other.kind === this.kind;
  }

  toDOM(): HTMLElement {
    const el = document.createElement('div');
    el.className = `cm-git-diff-marker cm-git-diff-${this.kind}`;
    el.title =
      this.kind === 'added'
        ? 'Línea añadida (Git)'
        : this.kind === 'modified'
          ? 'Línea modificada (Git)'
          : 'Líneas eliminadas (Git)';
    return el;
  }
}

const addedMarker = new GitDiffMarker('added');
const modifiedMarker = new GitDiffMarker('modified');
const deletedMarker = new GitDiffMarker('deleted');

function buildMarkers(doc: Text, base: string | null) {
  const builder = new RangeSetBuilder<GutterMarker>();
  if (base === null) return builder.finish();

  const marks = computeLineDiff(base, doc.toString());
  for (const mark of marks) {
    if (mark.line >= 1 && mark.line <= doc.lines) {
      const lineObj = doc.line(mark.line);
      const marker =
        mark.kind === 'added'
          ? addedMarker
          : mark.kind === 'modified'
            ? modifiedMarker
            : deletedMarker;
      builder.add(lineObj.from, lineObj.from, marker);
    }
  }
  return builder.finish();
}

/** Dispatches a new Git base content (e.g. HEAD) to update the gutter diff markers. */
export const setGitBaseContent = StateEffect.define<string | null>();

const gitBaseField = StateField.define<string | null>({
  create: () => null,
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setGitBaseContent)) return effect.value;
    }
    return value;
  },
});

interface DiffState {
  markers: ReturnType<typeof buildMarkers>;
  hunks: DiffHunk[];
}

function buildState(doc: Text, base: string | null): DiffState {
  return { markers: buildMarkers(doc, base), hunks: base === null ? [] : computeHunks(base, doc.toString()) };
}

const gitDiffMarkersField = StateField.define<DiffState>({
  create(state) {
    return buildState(state.doc, state.field(gitBaseField));
  },
  update(value, tr) {
    let baseChanged = false;
    for (const effect of tr.effects) {
      if (effect.is(setGitBaseContent)) {
        baseChanged = true;
        break;
      }
    }
    if (!baseChanged && !tr.docChanged) return value;
    return buildState(tr.state.doc, tr.state.field(gitBaseField));
  },
});

/** Bloque bajo el ratón en el margen (índice en `hunks`), para el tooltip */
const setHoverHunk = StateEffect.define<{ hunk: number; line: number } | null>();

/** Bloque al que pertenece la línea `n` (1-based) */
function hunkAtLine(hunks: DiffHunk[], n: number): number | null {
  const i = hunks.findIndex((h) => n >= h.startLine && n < h.startLine + Math.max(h.newCount, 1));
  return i < 0 ? null : i;
}

const MAX_TOOLTIP_LINES = 20;

function renderSegments(line: HTMLElement, segs: CharSegment[], cls: string) {
  for (const seg of segs) {
    const span = document.createElement('span');
    span.textContent = seg.text;
    if (seg.changed) span.className = cls;
    line.appendChild(span);
  }
}

function lineEl(kind: 'old' | 'new', content: string | CharSegment[]): HTMLElement {
  const el = document.createElement('div');
  el.className = `cm-git-diff-tip-line cm-git-diff-tip-${kind}`;
  const sign = document.createElement('span');
  sign.className = 'cm-git-diff-tip-sign';
  sign.textContent = kind === 'old' ? '−' : '+';
  el.appendChild(sign);
  if (typeof content === 'string') el.appendChild(document.createTextNode(content || ' '));
  else renderSegments(el, content, kind === 'old' ? 'cm-git-diff-tip-del' : 'cm-git-diff-tip-ins');
  return el;
}

/** Contenido del tooltip: qué había en Git y qué hay ahora, con las letras cambiadas resaltadas */
function renderHunk(h: DiffHunk): HTMLElement {
  const dom = document.createElement('div');
  dom.className = 'cm-git-diff-tip';
  const head = document.createElement('div');
  head.className = 'cm-git-diff-tip-head';
  const n = (k: number, one: string, many: string) => `${k} ${k === 1 ? one : many}`;
  head.textContent =
    h.kind === 'added'
      ? `Añadido: ${n(h.newLines.length, 'línea nueva', 'líneas nuevas')} (no estaban en Git)`
      : h.kind === 'deleted'
        ? `Borrado: ${n(h.oldLines.length, 'línea', 'líneas')} que había en Git`
        : `Modificado: así estaba en Git (−) y así está ahora (+)`;
  dom.appendChild(head);
  const body = document.createElement('div');
  body.className = 'cm-git-diff-tip-body';
  let shown = 0;
  const more = (left: number) => {
    const el = document.createElement('div');
    el.className = 'cm-git-diff-tip-more';
    el.textContent = `… y ${left} más`;
    body.appendChild(el);
  };
  if (h.kind === 'deleted') {
    for (const l of h.oldLines.slice(0, MAX_TOOLTIP_LINES)) body.appendChild(lineEl('old', l));
    if (h.oldLines.length > MAX_TOOLTIP_LINES) more(h.oldLines.length - MAX_TOOLTIP_LINES);
  } else if (h.kind === 'modified') {
    const pairs = Math.min(h.oldLines.length, h.newLines.length);
    for (let j = 0; j < pairs && shown < MAX_TOOLTIP_LINES; j++, shown++) {
      const d = charDiff(h.oldLines[j], h.newLines[j]);
      body.appendChild(lineEl('old', d.before));
      body.appendChild(lineEl('new', d.after));
    }
    for (const l of h.oldLines.slice(pairs, pairs + Math.max(0, MAX_TOOLTIP_LINES - shown))) body.appendChild(lineEl('old', l));
    for (const l of h.newLines.slice(pairs, pairs + Math.max(0, MAX_TOOLTIP_LINES - shown))) body.appendChild(lineEl('new', l));
    const total = Math.max(h.oldLines.length, h.newLines.length);
    if (total > MAX_TOOLTIP_LINES) more(total - MAX_TOOLTIP_LINES);
  }
  if (body.childNodes.length) dom.appendChild(body);
  return dom;
}

const hoverHunkField = StateField.define<{ hunk: number; line: number } | null>({
  create: () => null,
  update(value, tr) {
    for (const e of tr.effects) if (e.is(setHoverHunk)) return e.value;
    return tr.docChanged ? null : value;
  },
  provide: (f) =>
    showTooltip.compute([f, gitDiffMarkersField], (state): Tooltip | null => {
      const hover = state.field(f);
      const hunk = hover === null ? undefined : state.field(gitDiffMarkersField).hunks[hover.hunk];
      if (!hover || !hunk) return null;
      // Justo debajo de la línea bajo el ratón, al principio (pegado al margen)
      const line = state.doc.line(Math.min(Math.max(1, hover.line), state.doc.lines));
      return { pos: line.from, above: false, create: () => ({ dom: renderHunk(hunk) }) };
    }),
});

const gitDiffGutterTheme = EditorView.baseTheme({
  '.cm-tooltip:has(> .cm-git-diff-tip)': {
    border: '1px solid var(--border-primary, #444)',
    borderRadius: '6px',
    overflow: 'hidden',
  },
  '.cm-git-diff-tip': {
    maxWidth: 'min(80vw, 900px)',
    fontSize: '12px',
    background: 'var(--bg-primary, #1e1e1e)',
    color: 'var(--text-primary, #ddd)',
  },
  '.cm-git-diff-tip-head': {
    padding: '4px 8px',
    fontFamily: 'var(--font-ui, sans-serif)',
    color: 'var(--text-secondary, #aaa)',
    borderBottom: '1px solid var(--border-primary, #444)',
  },
  '.cm-git-diff-tip-body': {
    maxHeight: '40vh',
    overflow: 'auto',
    fontFamily: 'var(--font-mono, monospace)',
    whiteSpace: 'pre',
    padding: '2px 0',
  },
  '.cm-git-diff-tip-line': { padding: '0 8px 0 4px' },
  '.cm-git-diff-tip-old': { background: 'rgba(248, 81, 73, 0.12)' },
  '.cm-git-diff-tip-new': { background: 'rgba(46, 160, 67, 0.12)' },
  '.cm-git-diff-tip-sign': { display: 'inline-block', width: '14px', opacity: '0.7', userSelect: 'none' },
  '.cm-git-diff-tip-del': { background: 'rgba(248, 81, 73, 0.45)', borderRadius: '2px' },
  '.cm-git-diff-tip-ins': { background: 'rgba(46, 160, 67, 0.45)', borderRadius: '2px' },
  '.cm-git-diff-tip-more': { padding: '2px 8px', fontStyle: 'italic', opacity: '0.7' },
  '.cm-git-diff-gutter .cm-gutterElement:has(.cm-git-diff-marker)': { cursor: 'help' },
  '.cm-git-diff-gutter': {
    width: '6px',
    padding: '0',
    marginLeft: '1px',
  },
  '.cm-git-diff-gutter .cm-gutterElement': {
    padding: '0',
    display: 'flex',
    alignItems: 'stretch',
    justifyContent: 'center',
  },
  '.cm-git-diff-marker': {
    width: '3px',
    minHeight: '100%',
    borderRadius: '1px',
  },
  '.cm-git-diff-added': {
    backgroundColor: '#2ea043',
  },
  '.cm-git-diff-modified': {
    backgroundColor: '#2f81f7',
  },
  '.cm-git-diff-deleted': {
    width: '0',
    height: '0',
    minHeight: '0',
    alignSelf: 'flex-end',
    borderRadius: '0',
    borderTop: '4px solid transparent',
    borderBottom: '4px solid transparent',
    borderLeft: '5px solid #f85149',
  },
});

/**
 * CodeMirror 6 extension that displays Git line-diff indicators (added / modified / deleted)
 * in a slim gutter next to line numbers.
 */
export function gitDiffGutter(initialBase: string | null = null): Extension {
  return [
    gitBaseField.init(() => initialBase),
    gitDiffMarkersField,
    hoverHunkField,
    gutter({
      class: 'cm-git-diff-gutter',
      markers: (view) => view.state.field(gitDiffMarkersField).markers,
      // Al pasar por una marca: tooltip con lo que había en Git (y qué letras cambiaron)
      domEventHandlers: {
        mousemove(view, line) {
          const n = view.state.doc.lineAt(line.from).number;
          const i = hunkAtLine(view.state.field(gitDiffMarkersField).hunks, n);
          const cur = view.state.field(hoverHunkField);
          const next = i === null ? null : { hunk: i, line: n };
          if (cur?.hunk !== next?.hunk || cur?.line !== next?.line) view.dispatch({ effects: setHoverHunk.of(next) });
          return false;
        },
        mouseleave(view) {
          if (view.state.field(hoverHunkField) !== null) view.dispatch({ effects: setHoverHunk.of(null) });
          return false;
        },
      },
    }),
    gitDiffGutterTheme,
  ];
}
