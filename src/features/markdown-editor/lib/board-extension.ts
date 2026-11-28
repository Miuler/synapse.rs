// El bloque `board` en la vista en vivo: se ve el tablero mientras el cursor está fuera y su texto
// cuando se entra a editarlo. Los enlaces a tareas los pinta la extensión de wikilinks.
import { EditorView, Decoration, type DecorationSet, WidgetType } from '@codemirror/view';
import { StateField, type EditorState } from '@codemirror/state';
import { mountBoardReferences } from '@entities/board';
import { vaultApi } from '@entities/vault-item';
import { activeFilePathFacet } from './editor-facets';
import { onNavigateFacet } from './wikilink-extension';
import { findBoardBlocks, type BoardBlock } from './board-blocks';

const mounted = new WeakMap<HTMLElement, () => void>();

class BoardWidget extends WidgetType {
  constructor(
    readonly code: string,
    readonly codeOffset: number,
    readonly basePath: string | null,
  ) {
    super();
  }

  eq(other: BoardWidget): boolean {
    return other.code === this.code && other.codeOffset === this.codeOffset && other.basePath === this.basePath;
  }

  toDOM(view: EditorView): HTMLElement {
    const wrapper = document.createElement('div');
    wrapper.className = 'cm-board-ref';
    const box = document.createElement('div');
    box.className = 'board-ref';
    box.dataset.code = encodeURIComponent(this.code);
    wrapper.append(box);
    wrapper.addEventListener('click', (event) => {
      event.preventDefault();
      // Un enlace del tablero abre; el resto del bloque lleva a editarlo
      const anchor = (event.target as HTMLElement).closest<HTMLElement>('a[data-internal-path]');
      if (anchor) {
        view.state.facet(onNavigateFacet)(anchor.dataset.internalPath!);
        return;
      }
      const position = Math.min(view.posAtDOM(wrapper) + this.codeOffset, view.state.doc.length);
      view.dispatch({ selection: { anchor: position }, scrollIntoView: true });
      view.focus();
    });
    mounted.set(wrapper, mountBoardReferences(wrapper, this.basePath, vaultApi, () => view.requestMeasure()));
    return wrapper;
  }

  destroy(dom: HTMLElement): void {
    mounted.get(dom)?.();
    mounted.delete(dom);
  }

  ignoreEvent(): boolean {
    return true;
  }
}

function decorations(state: EditorState, blocks: BoardBlock[]): DecorationSet {
  const basePath = state.facet(activeFilePathFacet);
  const widgets = blocks
    .filter((block) => !state.selection.ranges.some((r) => r.from <= block.to && r.to >= block.from))
    .map((block) =>
      Decoration.replace({ widget: new BoardWidget(block.code, block.codeOffset, basePath), block: true }).range(
        block.from,
        block.to,
      ),
    );
  return Decoration.set(widgets);
}

/** Con los bloques ya buscados: mover el cursor no vuelve a recorrer la nota. */
export const boardLivePreviewField = StateField.define<{ blocks: BoardBlock[]; shown: DecorationSet }>({
  create(state) {
    const blocks = findBoardBlocks(state.doc.toString());
    return { blocks, shown: decorations(state, blocks) };
  },
  update(value, transaction) {
    const pathChanged =
      transaction.startState.facet(activeFilePathFacet) !== transaction.state.facet(activeFilePathFacet);
    if (!transaction.docChanged && !transaction.selection && !pathChanged) return value;
    const blocks = transaction.docChanged ? findBoardBlocks(transaction.state.doc.toString()) : value.blocks;
    return { blocks, shown: decorations(transaction.state, blocks) };
  },
  provide: (field) => EditorView.decorations.from(field, (value) => value.shown),
});
