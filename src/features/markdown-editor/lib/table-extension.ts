import { EditorView, Decoration, type DecorationSet, WidgetType } from '@codemirror/view';
import { StateField, StateEffect, type EditorState } from '@codemirror/state';
import { mount, unmount } from 'svelte';
import InteractiveTable from '../ui/InteractiveTable.svelte';
import {
  findTableBlocks,
  serializeMarkdownTable,
  type TableData,
} from './table-parser';

/**
 * Efecto para conmutar una tabla específica entre vista interactiva y vista de código Markdown sin procesar.
 */
export const toggleTableRawEffect = StateEffect.define<{ from: number }>();

/**
 * StateField que mantiene el conjunto de posiciones de tablas que se están editando en modo fuente.
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
 * Banner superior que aparece cuando una tabla está en modo código Markdown,
 * permitiendo volver al modo de tabla visual interactiva.
 */
class TableSourceBannerWidget extends WidgetType {
  constructor(readonly tableFrom: number) {
    super();
  }

  eq(other: TableSourceBannerWidget): boolean {
    return other.tableFrom === this.tableFrom;
  }

  toDOM(view: EditorView): HTMLElement {
    const banner = document.createElement('div');
    banner.className = 'cm-table-source-banner';
    banner.innerHTML = `
      <div class="cm-table-banner-info">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="16 18 22 12 16 6"></polyline>
          <polyline points="8 6 2 12 8 18"></polyline>
        </svg>
        <span>Modo código Markdown de tabla</span>
      </div>
      <button type="button" class="cm-table-banner-btn" title="Volver a la vista de tabla interactiva">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect width="18" height="18" x="3" y="3" rx="2" ry="2"/>
          <line x1="3" y1="9" x2="21" y2="9"/>
          <line x1="3" y1="15" x2="21" y2="15"/>
          <line x1="9" y1="3" x2="9" y2="21"/>
          <line x1="15" y1="3" x2="15" y2="21"/>
        </svg>
        <span>Ver como tabla</span>
      </button>
    `;

    const btn = banner.querySelector<HTMLButtonElement>('.cm-table-banner-btn');
    btn?.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      view.dispatch({
        effects: toggleTableRawEffect.of({ from: this.tableFrom }),
      });
    });

    return banner;
  }

  ignoreEvent(): boolean {
    return true;
  }
}

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

function buildTableDecorations(state: EditorState): DecorationSet {
  try {
    const widgets: any[] = [];
    const blocks = findTableBlocks(state.doc);
    const rawSet = state.field(tableRawField, false) || new Set<number>();

    for (const block of blocks) {
      if (rawSet.has(block.from)) {
        // En modo código fuente: insertar un banner informativo antes de la tabla y dejar ver el texto
        const bannerDeco = Decoration.widget({
          widget: new TableSourceBannerWidget(block.from),
          side: -1,
          block: true,
        });
        widgets.push(bannerDeco.range(block.from));
      } else {
        // En modo visual: reemplazar el bloque de texto completo por el widget de tabla interactivo
        const deco = Decoration.replace({
          widget: new TableWidget(
            block.table,
            block.from,
            block.to,
            block.rawText
          ),
          block: true,
        });
        widgets.push(deco.range(block.from, block.to));
      }
    }

    widgets.sort((a, b) => a.from - b.from);
    return Decoration.set(widgets, true);
  } catch (err) {
    console.error('Error al generar decoraciones de tablas en vivo:', err);
    return Decoration.none;
  }
}

/**
 * StateField que gestiona las decoraciones de tablas interactivas en modo Live Preview.
 */
export const tableLivePreviewField = StateField.define<DecorationSet>({
  create(state: EditorState): DecorationSet {
    return buildTableDecorations(state);
  },
  update(decorations: DecorationSet, tr): DecorationSet {
    if (tr.docChanged || tr.selection || tr.effects.some((e) => e.is(toggleTableRawEffect))) {
      return buildTableDecorations(tr.state);
    }
    return decorations.map(tr.changes);
  },
  provide: (field) => EditorView.decorations.from(field),
});
