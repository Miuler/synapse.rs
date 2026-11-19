// Referencias `$ref` de YAML y JSON (OpenAPI, JSON Schema): se subrayan como enlaces y con
// Ctrl+clic o Ctrl+Enter se abre el archivo al que apuntan y se salta a la clave del puntero `#/...`.
import { Decoration, type DecorationSet, EditorView, ViewPlugin, type ViewUpdate, keymap } from '@codemirror/view';
import { type Extension, RangeSetBuilder, Prec } from '@codemirror/state';

export interface RefMatch {
  /** Rango del valor de la referencia (sin comillas). */
  from: number;
  to: number;
  target: string;
}

// `$ref: ./a.yaml`, `$ref: '#/x'`, `- $ref: "b.yaml#/y"` y JSON `"$ref": "..."`
const REF_RE = /(?:^|[\s{,-])["']?\$ref["']?\s*:\s*(["']?)([^"'\s,}#][^"'\s,}]*|#[^"'\s,}]*)\1/g;

/** Referencias de una línea; `offset` es la posición de la línea en el documento. */
export function findRefsInLine(text: string, offset = 0): RefMatch[] {
  const out: RefMatch[] = [];
  REF_RE.lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = REF_RE.exec(text))) {
    const target = m[2];
    const valueStart = m.index + m[0].length - target.length - m[1].length;
    out.push({ from: offset + valueStart, to: offset + valueStart + target.length, target });
  }
  return out;
}

export interface ResolvedRef {
  /** Ruta del archivo relativa a la bóveda; `null` = el mismo archivo. */
  path: string | null;
  /** Puntero JSON (`/components/schemas/Error`), sin el `#`; `null` = sin salto. */
  pointer: string | null;
}

/** Resuelve `target` desde el archivo `currentPath` (rutas relativas a la bóveda). */
export function resolveRef(currentPath: string, target: string): ResolvedRef | null {
  const hash = target.indexOf('#');
  const file = hash === -1 ? target : target.slice(0, hash);
  const pointer = hash === -1 ? null : target.slice(hash + 1) || null;
  if (/^[a-z][a-z0-9+.-]*:/i.test(file)) return null; // URL externa
  if (!file) return { path: null, pointer };
  const parts = file.startsWith('/') ? [] : currentPath.split('/').slice(0, -1);
  for (const seg of decodeURI(file).split('/')) {
    if (!seg || seg === '.') continue;
    if (seg === '..') {
      if (!parts.length) return null; // fuera de la bóveda
      parts.pop();
    } else parts.push(seg);
  }
  return { path: parts.join('/'), pointer };
}

/** Segmentos de un puntero JSON (`~1` = `/`, `~0` = `~`, y codificación de URL). */
export function pointerSegments(pointer: string): string[] {
  return pointer
    .split('/')
    .filter((s, i) => i > 0 || s !== '')
    .map((s) => {
      let v = s;
      try {
        v = decodeURIComponent(s);
      } catch {
        /* se deja tal cual */
      }
      return v.replace(/~1/g, '/').replace(/~0/g, '~');
    });
}

interface LineInfo {
  /** Columna del primer carácter (el guion en un elemento de lista). */
  indent: number;
  /** Elemento de lista (`- ...`) y columna de su guion. */
  dash: boolean;
  /** Columna donde empieza la clave (tras `- ` en un elemento de lista). */
  col: number;
  key: string | null;
}

function lineInfo(text: string): LineInfo | null {
  if (!text.trim() || /^\s*(#|\/\/)/.test(text)) return null;
  const indent = text.length - text.trimStart().length;
  const dashM = /^(\s*-\s+)/.exec(text);
  const dash = !!dashM || /^\s*-$/.test(text);
  const col = dashM ? dashM[1].length : indent;
  const rest = text.slice(col);
  const m = /^(?:"((?:[^"\\]|\\.)*)"|'([^']*)'|([^\s"'#{}[\],][^:#]*?))\s*:(?:\s|$)/.exec(rest);
  const key = m ? (m[1] ?? m[2] ?? m[3]).trim() : null;
  return { indent, dash, col, key };
}

/**
 * Línea (1-based) de la clave a la que apunta `pointer`, siguiendo la sangría: cada segmento se
 * busca entre los hijos del anterior. Sirve para YAML y para JSON con formato (una clave por
 * línea). Los índices (`/0`) son el n-ésimo elemento `- ` del nivel.
 */
export function findPointerLine(lines: string[], pointer: string): number | null {
  const segs = pointerSegments(pointer);
  if (!segs.length) return 1;
  const infos = lines.map(lineInfo);
  let start = 0;
  let end = lines.length;
  let parentCol = -1;
  let found = -1;
  for (const seg of segs) {
    const isIndex = /^\d+$/.test(seg);
    let hit = -1;
    let hitCol = -1;
    if (isIndex) {
      // Elementos de lista del nivel: guiones en la primera columna de guion del ámbito
      let level: number | null = null;
      let n = -1;
      for (let i = start; i < end; i++) {
        const li = infos[i];
        if (!li || !li.dash || li.indent <= parentCol) continue;
        if (level === null) level = li.indent;
        if (li.indent !== level) continue;
        if (++n === Number(seg)) {
          hit = i;
          hitCol = li.indent;
          break;
        }
      }
      if (hit === -1) return null;
      found = hit;
      // Hijos del elemento: su propia línea (tras el guion) y las de sangría mayor que el guion
      start = hit;
      parentCol = hitCol;
    } else {
      let level: number | null = null;
      for (let i = start; i < end; i++) {
        const li = infos[i];
        // Solo cuentan las líneas con clave (en JSON, `{` o `}` no marcan el nivel)
        if (!li || li.key === null) continue;
        if (li.col <= parentCol) continue;
        if (level === null) level = li.col;
        if (li.col === level && li.key === seg) {
          hit = i;
          hitCol = li.col;
          break;
        }
      }
      if (hit === -1) return null;
      found = hit;
      start = hit + 1;
      parentCol = hitCol;
    }
    // El ámbito termina en la siguiente línea que no esté más adentro que el padre
    let j = isIndex ? hit + 1 : start;
    while (j < lines.length) {
      const li = infos[j];
      if (li && li.indent <= parentCol && !/^\s*[}\]]/.test(lines[j])) break;
      j++;
    }
    end = j;
  }
  return found + 1;
}

/** Nodo del índice de un YAML/JSON (misma forma que el índice de títulos de Markdown). */
export interface KeyNode {
  level: number;
  text: string;
  /** Puntero JSON de la clave (`/paths/~1transfers/post`), para saltar con `jumpToPointer`. */
  raw: string;
  line: number;
  children: KeyNode[];
}

const escapeSegment = (s: string) => s.replace(/~/g, '~0').replace(/\//g, '~1');

/**
 * Árbol de claves de un YAML (o JSON con formato) hasta `maxDepth` niveles, para el índice.
 * Los elementos de lista aparecen como `- valor` de su primera clave (p. ej. `- name: id` → `- id`).
 */
export function extractKeyTree(text: string, maxDepth = 4): KeyNode[] {
  const roots: KeyNode[] = [];
  // Pila de nodos abiertos con la columna a partir de la cual algo es su hijo
  const stack: { col: number; node: KeyNode | null; segs: string[]; items: number }[] = [
    { col: -1, node: null, segs: [], items: 0 },
  ];
  const lines = text.split(/\r?\n/);
  lines.forEach((line, i) => {
    const li = lineInfo(line);
    if (!li || (li.key === null && !li.dash)) return;
    const startCol = li.dash ? li.indent : li.col;
    while (stack.length > 1 && stack[stack.length - 1].col >= startCol) stack.pop();
    const parent = stack[stack.length - 1];
    const depth = stack.length; // 1 = raíz
    let segs: string[];
    let label: string;
    if (li.dash) {
      segs = [...parent.segs, String(parent.items++)];
      const rest = line.slice(li.col).trim();
      const m = /^[^:]+:\s*(.*)$/.exec(rest);
      label = `- ${(m ? m[1] || rest.split(':')[0] : rest).replace(/^["']|["']$/g, '') || '…'}`;
    } else {
      segs = [...parent.segs, li.key!];
      label = li.key!;
    }
    const node: KeyNode = { level: depth, text: label.slice(0, 80), raw: '/' + segs.map(escapeSegment).join('/'), line: i + 1, children: [] };
    // Más allá de `maxDepth` no se muestra (pero se sigue la pila para los punteros)
    // Las `$ref` son valores (enlaces), no secciones: no entran en el índice
    const shown = depth <= maxDepth && (depth === 1 || parent.node !== null) && li.key !== '$ref';
    if (shown) (parent.node ? parent.node.children : roots).push(node);
    // Un elemento de lista abre a sus claves (en su línea y más adentro); una clave, a lo de más adentro
    stack.push({ col: startCol, node: shown ? node : null, segs, items: 0 });
  });
  return roots;
}

/** Clave más profunda en o antes de `line` (1-based): la «sección» donde está el cursor. */
export function keyLineAt(nodes: KeyNode[], line: number): number {
  let best = -1;
  const walk = (ns: KeyNode[]) => {
    for (const n of ns) {
      if (n.line > line) return;
      best = n.line;
      walk(n.children);
    }
  };
  walk(nodes);
  return best;
}

export interface JsonRefOptions {
  /** Ruta (relativa a la bóveda) del archivo abierto en el editor. */
  getPath: () => string | null | undefined;
  /** Abre otro archivo y, si hay puntero, salta a su clave. */
  openRef: (path: string, pointer: string | null) => void;
  /** Avisos (referencia rota). */
  notify?: (message: string) => void;
}

/** Salta a la clave del puntero en el documento abierto; devuelve si la encontró. */
export function jumpToPointer(view: EditorView, pointer: string): boolean {
  const lines = view.state.doc.toString().split('\n');
  const line = findPointerLine(lines, pointer);
  if (line === null) return false;
  const l = view.state.doc.line(line);
  const col = l.text.length - l.text.trimStart().length;
  view.dispatch({
    selection: { anchor: l.from + col },
    effects: EditorView.scrollIntoView(l.from, { y: 'start', yMargin: 40 }),
  });
  view.focus();
  return true;
}

function refAt(view: EditorView, pos: number): RefMatch | null {
  const line = view.state.doc.lineAt(pos);
  return findRefsInLine(line.text, line.from).find((r) => pos >= r.from && pos <= r.to) ?? null;
}

function follow(view: EditorView, ref: RefMatch, opts: JsonRefOptions): void {
  const current = opts.getPath() ?? '';
  const resolved = resolveRef(current, ref.target);
  if (!resolved) {
    opts.notify?.(`No se puede abrir «${ref.target}»`);
    return;
  }
  if (resolved.path === null || resolved.path === current) {
    if (resolved.pointer && !jumpToPointer(view, resolved.pointer)) {
      opts.notify?.(`No se encontró «#${resolved.pointer}» en este archivo`);
    }
    return;
  }
  opts.openRef(resolved.path, resolved.pointer);
}

const refMark = Decoration.mark({ class: 'cm-json-ref' });

function buildRefDecorations(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  for (const { from, to } of view.visibleRanges) {
    let pos = from;
    while (pos <= to) {
      const line = view.state.doc.lineAt(pos);
      for (const r of findRefsInLine(line.text, line.from)) builder.add(r.from, r.to, refMark);
      pos = line.to + 1;
    }
  }
  return builder.finish();
}

/** Extensión: subraya los `$ref` y los sigue con Ctrl/Cmd+clic o Ctrl/Cmd+Enter. */
export function jsonRefLinks(opts: JsonRefOptions): Extension {
  const plugin = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;
      constructor(view: EditorView) {
        this.decorations = buildRefDecorations(view);
      }
      update(u: ViewUpdate) {
        if (u.docChanged || u.viewportChanged) this.decorations = buildRefDecorations(u.view);
      }
    },
    {
      decorations: (v) => v.decorations,
      eventHandlers: {
        mousedown(e, view) {
          if (!(e.ctrlKey || e.metaKey) || e.button !== 0) return false;
          const pos = view.posAtCoords({ x: e.clientX, y: e.clientY });
          const ref = pos === null ? null : refAt(view, pos);
          if (!ref) return false;
          e.preventDefault();
          follow(view, ref, opts);
          return true;
        },
      },
    }
  );
  return [
    plugin,
    Prec.high(
      keymap.of([
        {
          // F12 y Ctrl+B ya son atajos de la app (DevTools, barra lateral)
          key: 'Mod-Enter',
          run: (view) => {
            const ref = refAt(view, view.state.selection.main.head);
            if (!ref) return false;
            follow(view, ref, opts);
            return true;
          },
        },
      ])
    ),
    EditorView.baseTheme({
      '.cm-json-ref': {
        textDecoration: 'underline',
        textDecorationColor: 'color-mix(in srgb, currentColor 45%, transparent)',
        textUnderlineOffset: '3px',
      },
      // Con Ctrl/Cmd pulsado, cursor de enlace
      '&.cm-ref-mod .cm-json-ref': {
        cursor: 'pointer',
        color: 'var(--accent, #0969da)',
        textDecorationColor: 'currentColor',
      },
    }),
    EditorView.domEventHandlers({
      keydown(e, view) {
        if (e.key === 'Control' || e.key === 'Meta') view.dom.classList.add('cm-ref-mod');
        return false;
      },
      keyup(e, view) {
        if (e.key === 'Control' || e.key === 'Meta') view.dom.classList.remove('cm-ref-mod');
        return false;
      },
      blur(_e, view) {
        view.dom.classList.remove('cm-ref-mod');
        return false;
      },
      mousemove(e, view) {
        view.dom.classList.toggle('cm-ref-mod', e.ctrlKey || e.metaKey);
        return false;
      },
    }),
  ];
}
