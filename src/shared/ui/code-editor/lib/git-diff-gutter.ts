import { StateEffect, StateField, RangeSetBuilder, type Extension, type Text } from '@codemirror/state';
import { EditorView, GutterMarker, gutter } from '@codemirror/view';

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

const gitDiffMarkersField = StateField.define({
  create(state) {
    return buildMarkers(state.doc, state.field(gitBaseField));
  },
  update(markers, tr) {
    let baseChanged = false;
    for (const effect of tr.effects) {
      if (effect.is(setGitBaseContent)) {
        baseChanged = true;
        break;
      }
    }
    if (!baseChanged && !tr.docChanged) return markers;
    const base = tr.state.field(gitBaseField);
    if (base === null) return buildMarkers(tr.state.doc, null);
    return buildMarkers(tr.state.doc, base);
  },
});

const gitDiffGutterTheme = EditorView.baseTheme({
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
    gutter({
      class: 'cm-git-diff-gutter',
      markers: (view) => view.state.field(gitDiffMarkersField),
    }),
    gitDiffGutterTheme,
  ];
}
