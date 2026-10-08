import { EditorView, Decoration, type DecorationSet, WidgetType } from '@codemirror/view';
import { StateField, type EditorState, Facet } from '@codemirror/state';
import { vaultRepository } from '@shared/repositories';
import { activeFilePathFacet } from './mermaid-extension';
import { extractWikilinkTokens, type WikilinkToken } from './link-resolver';

export const onNavigateFacet = Facet.define<(path: string) => void, (path: string) => void>({
  combine: (values) => values[values.length - 1] || (() => {}),
});

const resolvedLinkCache = new Map<string, string>();

class WikilinkWidget extends WidgetType {
  constructor(
    readonly token: WikilinkToken,
    readonly resolvedPath?: string,
    readonly basePath?: string | null,
  ) {
    super();
  }

  eq(other: WikilinkWidget): boolean {
    return (
      other.token.raw === this.token.raw &&
      other.token.target === this.token.target &&
      other.token.displayText === this.token.displayText &&
      other.resolvedPath === this.resolvedPath &&
      other.basePath === this.basePath
    );
  }

  toDOM(view: EditorView): HTMLElement {
    const span = document.createElement('span');
    span.className = 'cm-wikilink-widget';
    span.setAttribute('data-target', this.token.target);

    const anchor = document.createElement('a');
    anchor.className = 'cm-wikilink-anchor';
    anchor.textContent = this.token.displayText;
    anchor.title = this.resolvedPath || this.token.target;
    anchor.href = '#';

    const cacheKey = `${this.basePath || ''}::${this.token.target}`;

    // Manejar clic para navegar a la nota correspondiente
    anchor.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();

      const onNav = view.state.facet(onNavigateFacet);
      const targetPath = this.resolvedPath || resolvedLinkCache.get(cacheKey);

      if (targetPath) {
        onNav(targetPath);
      } else {
        vaultRepository.resolveVaultLink(this.token.target, this.basePath).then((p) => {
          const finalPath = p || this.token.target;
          resolvedLinkCache.set(cacheKey, finalPath);
          onNav(finalPath);
        });
      }
    });

    span.appendChild(anchor);

    // Precargar en caché si aún no está
    if (!resolvedLinkCache.has(cacheKey)) {
      vaultRepository.resolveVaultLink(this.token.target, this.basePath).then((resolved) => {
        if (resolved) {
          resolvedLinkCache.set(cacheKey, resolved);
          anchor.title = resolved;
        }
      });
    }

    return span;
  }

  ignoreEvent(event: Event): boolean {
    // Permitir clic para navegar sin mover el cursor inmediatamente
    if (event.type === 'click') return true;
    return false;
  }
}

function buildWikilinkDecorations(state: EditorState): DecorationSet {
  try {
    const widgets: any[] = [];
    const selectionRanges = state.selection.ranges;
    const basePath = state.facet(activeFilePathFacet);
    const doc = state.doc;

    for (let i = 1; i <= doc.lines; i++) {
      const line = doc.line(i);
      const tokens = extractWikilinkTokens(line.text, line.from);

      for (const token of tokens) {
        // Los archivos de imagen son gestionados visualmente por image-extension
        if (token.isImage) continue;

        const hasCursor = selectionRanges.some(
          (r) => r.from <= token.to && r.to >= token.from
        );

        if (!hasCursor) {
          const cacheKey = `${basePath || ''}::${token.target}`;
          const resolved = resolvedLinkCache.get(cacheKey);
          const deco = Decoration.replace({
            widget: new WikilinkWidget(token, resolved, basePath),
            inclusive: false,
          });
          widgets.push(deco.range(token.from, token.to));
        }
      }
    }

    widgets.sort((a, b) => a.from - b.from);
    return Decoration.set(widgets, true);
  } catch (err) {
    console.error('Error generando decoraciones de WikiLinks:', err);
    return Decoration.none;
  }
}

export const wikilinkLivePreviewField = StateField.define<DecorationSet>({
  create(state) {
    return buildWikilinkDecorations(state);
  },
  update(decorations, transaction) {
    if (transaction.docChanged || transaction.selection) {
      return buildWikilinkDecorations(transaction.state);
    }
    return decorations;
  },
  provide(field) {
    return EditorView.decorations.from(field);
  },
});
