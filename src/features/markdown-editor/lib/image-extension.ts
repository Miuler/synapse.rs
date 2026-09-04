import { EditorView, Decoration, type DecorationSet, WidgetType } from '@codemirror/view';
import { StateField, type EditorState } from '@codemirror/state';
import { activeFilePathFacet } from './mermaid-extension';
import { extractImageTokens, resolveVaultImageUrl, type ImageToken } from './image-resolver';

class ImageWidget extends WidgetType {
  constructor(
    readonly token: ImageToken,
    readonly basePath?: string | null,
  ) {
    super();
  }

  eq(other: ImageWidget): boolean {
    return (
      other.token.raw === this.token.raw &&
      other.token.src === this.token.src &&
      other.token.alt === this.token.alt &&
      other.token.width === this.token.width &&
      other.token.height === this.token.height &&
      other.basePath === this.basePath
    );
  }

  toDOM(view: EditorView): HTMLElement {
    const container = document.createElement('div');
    container.className = 'cm-image-widget-wrapper';

    const card = document.createElement('div');
    card.className = 'cm-image-card';

    // Header flotante con acciones discretas
    const actions = document.createElement('div');
    actions.className = 'cm-image-actions';

    const editBtn = document.createElement('button');
    editBtn.type = 'button';
    editBtn.className = 'cm-image-action-btn';
    editBtn.title = 'Editar sintaxis de la imagen';
    editBtn.innerHTML = `
      <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
        <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
      </svg>
      <span>Editar</span>
    `;
    editBtn.addEventListener('mousedown', (e) => {
      e.preventDefault();
      e.stopPropagation();
      view.dispatch({
        selection: { anchor: this.token.from },
        scrollIntoView: true,
      });
      view.focus();
    });
    actions.appendChild(editBtn);
    card.appendChild(actions);

    const imgContainer = document.createElement('div');
    imgContainer.className = 'cm-image-content';

    const loadingPlaceholder = document.createElement('div');
    loadingPlaceholder.className = 'cm-image-loading';
    loadingPlaceholder.innerHTML = `
      <span class="cm-image-spinner"></span>
      <span>Cargando imagen...</span>
    `;
    imgContainer.appendChild(loadingPlaceholder);

    const img = document.createElement('img');
    img.alt = this.token.alt || 'Imagen';
    if (this.token.title) {
      img.title = this.token.title;
    }
    img.className = 'cm-image-element';

    if (this.token.width) {
      img.style.width = this.token.width;
    }
    if (this.token.height) {
      img.style.height = this.token.height;
    }

    // Permitir clic para enfocar y editar
    img.addEventListener('click', (e) => {
      e.preventDefault();
      view.dispatch({
        selection: { anchor: this.token.from },
        scrollIntoView: true,
      });
      view.focus();
    });

    img.onload = () => {
      loadingPlaceholder.remove();
      view.requestMeasure();
    };

    img.onerror = () => {
      loadingPlaceholder.remove();
      const errorEl = document.createElement('div');
      errorEl.className = 'cm-image-error';
      errorEl.innerHTML = `
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
          <rect width="18" height="18" x="3" y="3" rx="2" ry="2"/>
          <circle cx="9" cy="9" r="2"/>
          <path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/>
        </svg>
        <span>No se pudo cargar la imagen: <code>${escapeHtml(this.token.src)}</code></span>
      `;
      imgContainer.appendChild(errorEl);
      view.requestMeasure();
    };

    // Resolver URL asíncronamente
    resolveVaultImageUrl(this.token.src, this.basePath).then((resolvedUrl) => {
      if (resolvedUrl) {
        img.src = resolvedUrl;
        imgContainer.appendChild(img);
      } else {
        img.onerror?.(new Event('error'));
      }
    });

    card.appendChild(imgContainer);

    // Caption opcional si tiene descripción
    if (this.token.alt && this.token.alt !== this.token.src) {
      const caption = document.createElement('div');
      caption.className = 'cm-image-caption';
      caption.textContent = this.token.alt;
      card.appendChild(caption);
    }

    container.appendChild(card);
    return container;
  }

  ignoreEvent(event: Event): boolean {
    return event.type === 'mousedown';
  }
}

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

function buildImageDecorations(state: EditorState): DecorationSet {
  const widgets: any[] = [];
  const selectionRanges = state.selection.ranges;
  const basePath = state.facet(activeFilePathFacet);
  const doc = state.doc;

  for (let i = 1; i <= doc.lines; i++) {
    const line = doc.line(i);
    const tokens = extractImageTokens(line.text, line.from);

    for (const token of tokens) {
      const hasCursor = selectionRanges.some(
        (r) => r.from <= token.to && r.to >= token.from
      );

      if (!hasCursor) {
        const deco = Decoration.replace({
          widget: new ImageWidget(token, basePath),
          inclusive: false,
        });
        widgets.push(deco.range(token.from, token.to));
      }
    }
  }

  widgets.sort((a, b) => a.from - b.from);
  return Decoration.set(widgets, true);
}

export const imageLivePreviewField = StateField.define<DecorationSet>({
  create(state: EditorState): DecorationSet {
    return buildImageDecorations(state);
  },
  update(decorations: DecorationSet, tr): DecorationSet {
    if (tr.docChanged || tr.selection) {
      return buildImageDecorations(tr.state);
    }
    return decorations.map(tr.changes);
  },
  provide: (field) => EditorView.decorations.from(field),
});
