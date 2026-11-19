import { EditorView, Decoration, type DecorationSet, WidgetType, ViewPlugin, type ViewUpdate } from '@codemirror/view';
import { StateField, StateEffect, type EditorState, type Range } from '@codemirror/state';
import { mount, unmount } from 'svelte';
import InteractiveTable from '../ui/InteractiveTable.svelte';
import {
  findTableBlocks,
  serializeMarkdownTable,
  type TableData,
} from './table-parser';

/** Una celda de una línea de tabla: rango en el documento (sin las `|`) y su columna. */
export interface TableCellSpan {
  from: number;
  to: number;
  col: number;
}

/** Disposición de una línea de tabla: posiciones de las `|` y celdas entre ellas. */
export interface TableLineLayout {
  lineFrom: number;
  kind: 'header' | 'separator' | 'row';
  pipes: number[];
  cells: TableCellSpan[];
}

/** Posiciones (relativas a la línea) de las `|` que separan celdas, sin las escapadas (`\|`) ni las de código. */
function pipePositions(text: string): number[] {
  const out: number[] = [];
  let inCode = false;
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (ch === '\\') {
      i++;
      continue;
    }
    if (ch === '`') inCode = !inCode;
    else if (ch === '|' && !inCode) out.push(i);
  }
  return out;
}

/**
 * Divide las líneas de una tabla Markdown en celdas y calcula el ancho de cada columna (en
 * caracteres) para alinear las celdas sin reemplazar el texto: las líneas siguen siendo del
 * documento, así que el cursor, las alturas y la numeración no se desincronizan.
 */
export function layoutTableLines(lines: { from: number; text: string }[]): {
  lines: TableLineLayout[];
  widths: number[];
} {
  const widths: number[] = [];
  const out = lines.map((line, idx) => {
    const pipes = pipePositions(line.text);
    // Bordes de cada celda: entre pipes, más el texto antes de la primera y después de la última
    const bounds: [number, number][] = [];
    let start = 0;
    for (const p of pipes) {
      bounds.push([start, p]);
      start = p + 1;
    }
    bounds.push([start, line.text.length]);
    // Sin contenido antes de la primera `|` o después de la última: son pipes exteriores
    const cellsRel = bounds.filter(([a, b], i) => {
      const empty = !line.text.slice(a, b).trim();
      return !(empty && (i === 0 || i === bounds.length - 1) && pipes.length > 0);
    });
    const cells = cellsRel.map(([a, b], col) => {
      const len = line.text.slice(a, b).trim().length;
      widths[col] = Math.max(widths[col] ?? 0, idx === 1 ? 3 : len);
      return { from: line.from + a, to: line.from + b, col };
    });
    const kind: TableLineLayout['kind'] = idx === 0 ? 'header' : idx === 1 ? 'separator' : 'row';
    return { lineFrom: line.from, kind, pipes: pipes.map((p) => line.from + p), cells };
  });
  return { lines: out, widths };
}

/**
 * Relleno (px) que falta después del texto de cada celda para alinear su columna, por posición
 * de la `|` que la cierra. Lo mide `tableAlignPlugin` con las coordenadas reales del texto, así que
 * funciona con fuentes proporcionales y con enlaces o imágenes dentro de las celdas.
 */
const setTableSpacersEffect = StateEffect.define<Map<number, number>>();

const tableSpacersField = StateField.define<Map<number, number>>({
  create: () => new Map(),
  update(value, tr) {
    for (const e of tr.effects) if (e.is(setTableSpacersEffect)) return e.value;
    if (!tr.docChanged || value.size === 0) return value;
    const mapped = new Map<number, number>();
    for (const [pos, px] of value) mapped.set(tr.changes.mapPos(pos), px);
    return mapped;
  },
});

class TableSpacerWidget extends WidgetType {
  constructor(readonly px: number) {
    super();
  }
  eq(other: TableSpacerWidget): boolean {
    return Math.abs(other.px - this.px) < 0.5;
  }
  toDOM(): HTMLElement {
    const el = document.createElement('span');
    el.className = 'cm-md-table-spacer';
    el.style.width = `${this.px}px`;
    return el;
  }
  updateDOM(dom: HTMLElement): boolean {
    dom.style.width = `${this.px}px`;
    return true;
  }
  ignoreEvent(): boolean {
    return false;
  }
}

/** Mide el ancho del texto de las celdas visibles y actualiza los rellenos de cada columna. */
const tableAlignPlugin = ViewPlugin.fromClass(
  class {
    constructor(view: EditorView) {
      this.schedule(view);
    }
    update(u: ViewUpdate) {
      if (u.docChanged || u.viewportChanged || u.geometryChanged) this.schedule(u.view);
    }
    schedule(view: EditorView) {
      view.requestMeasure({
        key: this,
        read: (v) => measureSpacers(v),
        write: (spacers, v) => {
          const current = v.state.field(tableSpacersField, false);
          if (!current || sameSpacers(current, spacers)) return;
          // No se puede despachar dentro del ciclo de medida
          queueMicrotask(() => v.dispatch({ effects: setTableSpacersEffect.of(spacers) }));
        },
      });
    }
  }
);

function sameSpacers(a: Map<number, number>, b: Map<number, number>): boolean {
  if (a.size !== b.size) return false;
  for (const [k, v] of a) if (Math.abs((b.get(k) ?? -100) - v) > 1) return false;
  return true;
}

function measureSpacers(view: EditorView): Map<number, number> {
  const state = view.state;
  const previous = state.field(tableSpacersField, false) ?? new Map<number, number>();
  const visual = state.field(tableRawField, false) ?? new Set<number>();
  const out = new Map<number, number>();
  for (const block of findTableBlocks(state.doc)) {
    if (visual.has(block.from)) continue;
    const layout = tableLayout(state, block.from, block.to).lines;
    // Ancho natural de cada celda: del inicio de la celda al final de su texto (sin el relleno)
    const natural: { pipe: number; col: number; w: number | null }[] = [];
    for (const line of layout) {
      for (const cell of line.cells) {
        const pipe = line.pipes.find((p) => p === cell.to);
        if (pipe === undefined) continue;
        // Del borde izquierdo de la celda al borde derecho de su último carácter: el relleno
        // (que va justo antes de la `|`) nunca entra en la medida
        const a = view.coordsAtPos(cell.from, 1);
        const b = cell.to > cell.from ? view.coordsForChar(cell.to - 1) : null;
        const w = a && b && Math.abs(a.top - b.top) < 4 ? b.right - a.left : null;
        natural.push({ pipe, col: cell.col, w });
      }
    }
    const maxByCol = new Map<number, number>();
    for (const n of natural) if (n.w !== null) maxByCol.set(n.col, Math.max(maxByCol.get(n.col) ?? 0, n.w));
    for (const n of natural) {
      // Fuera del área visible no hay coordenadas: se conserva el relleno anterior
      const px = n.w === null ? previous.get(n.pipe) : (maxByCol.get(n.col) ?? n.w) - n.w;
      if (px !== undefined && px > 0.5) out.set(n.pipe, Math.round(px));
    }
  }
  return out;
}

function tableLayout(state: EditorState, from: number, to: number) {
  const first = state.doc.lineAt(from).number;
  const last = state.doc.lineAt(to).number;
  const lines = [];
  for (let n = first; n <= last; n++) {
    const l = state.doc.line(n);
    lines.push({ from: l.from, text: l.text });
  }
  return layoutTableLines(lines);
}

/** Botón al final del encabezado para pasar la tabla al editor visual. */
class TableVisualButtonWidget extends WidgetType {
  constructor(readonly tableFrom: number) {
    super();
  }

  eq(other: TableVisualButtonWidget): boolean {
    return other.tableFrom === this.tableFrom;
  }

  toDOM(view: EditorView): HTMLElement {
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'cm-md-table-visual-btn';
    btn.title = 'Editar esta tabla con el editor visual de celdas';
    btn.textContent = 'Editor visual';
    btn.addEventListener('mousedown', (e) => e.preventDefault());
    btn.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      view.dispatch({ effects: toggleTableRawEffect.of({ from: this.tableFrom }) });
    });
    return btn;
  }

  ignoreEvent(): boolean {
    return true;
  }
}

/**
 * Efecto para conmutar una tabla entre las líneas decoradas (por defecto) y el editor visual de celdas.
 */
export const toggleTableRawEffect = StateEffect.define<{ from: number }>();

/**
 * StateField con las posiciones de las tablas abiertas en el editor visual de celdas.
 */
export const tableRawField = StateField.define<Set<number>>({
  create() {
    return new Set<number>();
  },
  update(rawSet, tr) {
    let next = rawSet;
    for (const effect of tr.effects) {
      if (effect.is(toggleTableRawEffect)) {
        next = new Set(next);
        if (next.has(effect.value.from)) {
          next.delete(effect.value.from);
        } else {
          next.add(effect.value.from);
        }
      }
    }
    if (tr.docChanged && next.size > 0) {
      const mapped = new Set<number>();
      for (const pos of next) {
        mapped.add(tr.changes.mapPos(pos));
      }
      return mapped;
    }
    return next;
  },
});

/**
 * Widget de CodeMirror 6 que monta el componente Svelte interactivo para la tabla.
 */
class TableWidget extends WidgetType {
  private component: any = null;

  constructor(
    public tableData: TableData,
    public from: number,
    public to: number,
    public rawText: string
  ) {
    super();
  }

  eq(other: TableWidget): boolean {
    return (
      other.rawText === this.rawText &&
      other.from === this.from &&
      other.to === this.to
    );
  }

  toDOM(view: EditorView): HTMLElement {
    const container = document.createElement('div');
    container.className = 'cm-table-widget-container';

    let currentFrom = this.from;
    (container as any).__currentFrom = currentFrom;

    this.component = mount(InteractiveTable, {
      target: container,
      props: {
        initialData: this.tableData,
        onUpdate: (newData: TableData) => {
          const newMarkdown = serializeMarkdownTable(newData);
          const currentDoc = view.state.doc;
          const blocks = findTableBlocks(currentDoc);

          // Determinar la posición viva actual del widget en el documento
          let targetPos = (container as any).__currentFrom ?? currentFrom;
          try {
            const domPos = view.posAtDOM(container);
            if (domPos !== null && domPos >= 0) {
              targetPos = domPos;
            }
          } catch {
            // Usar targetPos previo si posAtDOM no está listo
          }

          // Localizar el bloque exacto correspondiente a esta tabla en el documento actual
          let targetBlock = blocks.find((b) => b.from <= targetPos && b.to >= targetPos);
          if (!targetBlock && blocks.length > 0) {
            targetBlock = blocks.reduce((prev, curr) =>
              Math.abs(curr.from - targetPos) < Math.abs(prev.from - targetPos) ? curr : prev
            );
          }

          if (targetBlock) {
            const currentSlice = currentDoc.sliceString(targetBlock.from, targetBlock.to);
            if (currentSlice !== newMarkdown) {
              this.rawText = newMarkdown;
              this.tableData = newData;

              view.dispatch({
                changes: {
                  from: targetBlock.from,
                  to: targetBlock.to,
                  insert: newMarkdown,
                },
              });

              currentFrom = targetBlock.from;
              (container as any).__currentFrom = currentFrom;
              this.from = targetBlock.from;
              this.to = targetBlock.from + newMarkdown.length;
            }
          }
        },
        onSwitchToSource: () => {
          let targetPos = (container as any).__currentFrom ?? currentFrom;
          try {
            const domPos = view.posAtDOM(container);
            if (domPos !== null && domPos >= 0) targetPos = domPos;
          } catch {}

          const blocks = findTableBlocks(view.state.doc);
          const block = blocks.find((b) => b.from <= targetPos && b.to >= targetPos) || blocks[0];
          const fromPos = block ? block.from : targetPos;

          view.dispatch({
            effects: toggleTableRawEffect.of({ from: fromPos }),
          });
        },
      },
    });

    return container;
  }

  updateDOM(dom: HTMLElement, view: EditorView): boolean {
    // Al reutilizar el DOM del widget en actualizaciones de CodeMirror,
    // sincronizamos la posición viva para este contenedor
    (dom as any).__currentFrom = this.from;
    return true;
  }

  destroy(): void {
    if (this.component) {
      unmount(this.component);
      this.component = null;
    }
  }

  ignoreEvent(): boolean {
    return true;
  }
}

function buildTableDecorations(state: EditorState, spacers: Map<number, number>): DecorationSet {
  try {
    const decos: Range<Decoration>[] = [];
    const blocks = findTableBlocks(state.doc);
    const visualSet = state.field(tableRawField, false) || new Set<number>();

    for (const block of blocks) {
      if (visualSet.has(block.from)) {
        // Editor visual de celdas: reemplaza el bloque por el widget interactivo
        decos.push(
          Decoration.replace({
            widget: new TableWidget(block.table, block.from, block.to, block.rawText),
            block: true,
          }).range(block.from, block.to)
        );
        continue;
      }

      // Por defecto: las líneas siguen en el documento y solo se decoran
      const first = state.doc.lineAt(block.from).number;
      const layout = tableLayout(state, block.from, block.to).lines;
      layout.forEach((line, i) => {
        const cls = [
          'cm-md-table-line',
          `cm-md-table-${line.kind}`,
          i === 0 ? 'cm-md-table-first' : '',
          i === layout.length - 1 ? 'cm-md-table-last' : '',
          i > 1 && i % 2 === 1 ? 'cm-md-table-even' : '',
        ]
          .filter(Boolean)
          .join(' ');
        decos.push(Decoration.line({ class: cls }).range(line.lineFrom));
        for (const p of line.pipes) {
          const px = spacers.get(p);
          if (px) decos.push(Decoration.widget({ widget: new TableSpacerWidget(px), side: -1 }).range(p));
          decos.push(Decoration.mark({ class: 'cm-md-table-pipe' }).range(p, p + 1));
        }
        if (i === 0) {
          const end = state.doc.line(first).to;
          decos.push(Decoration.widget({ widget: new TableVisualButtonWidget(block.from), side: 1 }).range(end));
        }
      });
    }

    return Decoration.set(decos, true);
  } catch (err) {
    console.error('Error al generar decoraciones de tablas en vivo:', err);
    return Decoration.none;
  }
}

/** Estilo de las líneas de tabla decoradas. */
export const tableLineTheme = EditorView.baseTheme({
  '.cm-md-table-line': {
    borderLeft: '1px solid var(--border-primary, #d0d7de)',
    borderRight: '1px solid var(--border-primary, #d0d7de)',
  },
  '.cm-md-table-first': {
    borderTop: '1px solid var(--border-primary, #d0d7de)',
    borderTopLeftRadius: '6px',
    borderTopRightRadius: '6px',
  },
  '.cm-md-table-last': {
    borderBottom: '1px solid var(--border-primary, #d0d7de)',
    borderBottomLeftRadius: '6px',
    borderBottomRightRadius: '6px',
  },
  '.cm-md-table-header': {
    fontWeight: '600',
    background: 'var(--bg-secondary, #f6f8fa)',
  },
  '.cm-md-table-even': {
    background: 'color-mix(in srgb, var(--bg-secondary, #f6f8fa) 55%, transparent)',
  },
  '.cm-md-table-separator': {
    color: 'var(--text-tertiary, #8c959f)',
    fontSize: '0.75em',
    lineHeight: '1.2',
    background: 'var(--bg-secondary, #f6f8fa)',
  },
  '.cm-md-table-spacer': {
    display: 'inline-block',
  },
  // Las `|` como separadores tenues (por encima del color del resaltado de Markdown)
  '.cm-md-table-pipe, .cm-md-table-pipe *': {
    color: 'var(--border-primary, #d0d7de) !important',
  },
  '.cm-md-table-visual-btn': {
    marginLeft: '12px',
    padding: '0 6px',
    fontSize: '11px',
    fontWeight: '500',
    lineHeight: '18px',
    borderRadius: '4px',
    border: '1px solid var(--border-primary, #d0d7de)',
    background: 'var(--bg-primary, #fff)',
    color: 'var(--text-secondary, #656d76)',
    cursor: 'pointer',
    opacity: '0',
    transition: 'opacity 0.15s',
  },
  '.cm-md-table-header:hover .cm-md-table-visual-btn, .cm-md-table-visual-btn:focus': {
    opacity: '1',
  },
});

/**
 * StateField que gestiona las decoraciones de tablas interactivas en modo Live Preview.
 */
export const tableLivePreviewField = StateField.define<DecorationSet>({
  create(state: EditorState): DecorationSet {
    return buildTableDecorations(state, new Map());
  },
  update(decorations: DecorationSet, tr): DecorationSet {
    if (
      tr.docChanged ||
      tr.selection ||
      tr.effects.some((e) => e.is(toggleTableRawEffect) || e.is(setTableSpacersEffect))
    ) {
      // Rellenos de este mismo paso (sin depender del orden de actualización de los campos)
      let spacers: Map<number, number> | null = null;
      for (const e of tr.effects) if (e.is(setTableSpacersEffect)) spacers = e.value;
      if (!spacers) {
        spacers = new Map();
        for (const [pos, px] of tr.startState.field(tableSpacersField, false) ?? new Map<number, number>()) {
          spacers.set(tr.changes.mapPos(pos), px);
        }
      }
      return buildTableDecorations(tr.state, spacers);
    }
    return decorations.map(tr.changes);
  },
  provide: (field) => [EditorView.decorations.from(field), tableLineTheme, tableSpacersField, tableAlignPlugin],
});
