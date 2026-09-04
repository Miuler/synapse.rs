import { EditorView, Decoration, type DecorationSet, WidgetType } from '@codemirror/view';
import { StateField, type EditorState, type Text } from '@codemirror/state';
import { renderMermaidSvg, ensureMerman, isMermanInitialized } from '@features/merman-editor';

export interface MermaidBlock {
  from: number;
  to: number;
  codeFrom: number;
  codeTo: number;
  code: string;
}

/**
 * Escanea el documento de forma robusta e instantánea para encontrar
 * bloques de código ```mermaid o ```merman.
 */
export function findMermaidBlocks(doc: Text): MermaidBlock[] {
  const blocks: MermaidBlock[] = [];
  const lineCount = doc.lines;
  let inBlock = false;
  let blockStartFrom = -1;
  let codeStartFrom = -1;
  const codeLines: string[] = [];

  for (let i = 1; i <= lineCount; i++) {
    const line = doc.line(i);
    const text = line.text;
    const trimmed = text.trim();

    if (!inBlock) {
      if (/^```\s*(?:mermaid|mermair|mermai|merman)\b/i.test(trimmed)) {
        inBlock = true;
        blockStartFrom = line.from;
        codeStartFrom = Math.min(line.to + 1, doc.length);
        codeLines.length = 0;
      }
    } else {
      if (/^```\s*$/.test(trimmed)) {
        const code = codeLines.join('\n').trim();
        const codeEndTo = line.from > 0 ? line.from - 1 : line.from;
        blocks.push({
          from: blockStartFrom,
          to: line.to,
          codeFrom: Math.min(codeStartFrom, line.from),
          codeTo: codeEndTo,
          code,
        });
        inBlock = false;
      } else {
        codeLines.push(text);
      }
    }
  }

  return blocks;
}

class MermaidWidget extends WidgetType {
  constructor(
    readonly code: string,
    readonly from: number,
    readonly to: number,
    readonly codeFrom: number,
    readonly isPreviewOnly: boolean,
  ) {
    super();
  }

  eq(other: MermaidWidget): boolean {
    return (
      other.code === this.code &&
      other.isPreviewOnly === this.isPreviewOnly &&
      other.from === this.from &&
      other.to === this.to
    );
  }

  toDOM(view: EditorView): HTMLElement {
    const container = document.createElement('div');
    container.className = 'cm-mermaid-widget-wrapper';
    if (this.isPreviewOnly) {
      container.classList.add('is-preview-only');
    } else {
      container.classList.add('is-live-edit');
    }

    const header = document.createElement('div');
    header.className = 'cm-mermaid-header';

    const titleEl = document.createElement('div');
    titleEl.className = 'cm-mermaid-badge';
    titleEl.innerHTML = `
      <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2">
        <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>
      </svg>
      <span>Diagrama Mermaid</span>
    `;
    header.appendChild(titleEl);

    const actionsEl = document.createElement('div');
    actionsEl.className = 'cm-mermaid-actions';

    if (this.isPreviewOnly) {
      const editBtn = document.createElement('button');
      editBtn.type = 'button';
      editBtn.className = 'cm-mermaid-edit-btn';
      editBtn.title = 'Editar código de este diagrama';
      editBtn.innerHTML = `
        <svg viewBox="0 0 24 24" width="11" height="11" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
          <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
        </svg>
        <span>Editar código</span>
      `;
      editBtn.addEventListener('mousedown', (e) => {
        e.preventDefault();
        e.stopPropagation();
        const targetPos = Math.min(this.codeFrom, view.state.doc.length);
        view.dispatch({
          selection: { anchor: targetPos },
          scrollIntoView: true,
        });
        view.focus();
      });
      actionsEl.appendChild(editBtn);
    } else {
      const liveBadge = document.createElement('span');
      liveBadge.className = 'cm-mermaid-live-pill';
      liveBadge.innerHTML = `<span class="cm-mermaid-live-dot"></span>Vista previa en vivo`;
      actionsEl.appendChild(liveBadge);
    }

    header.appendChild(actionsEl);
    container.appendChild(header);

    const body = document.createElement('div');
    body.className = 'cm-mermaid-body';

    // Si es modo sustitución/preview, hacer clic en el cuerpo también activa la edición
    if (this.isPreviewOnly) {
      body.addEventListener('click', (e) => {
        const target = e.target as HTMLElement;
        if (target && target.closest('.cm-mermaid-edit-btn')) return;
        const targetPos = Math.min(this.codeFrom, view.state.doc.length);
        view.dispatch({
          selection: { anchor: targetPos },
          scrollIntoView: true,
        });
        view.focus();
      });
    }

    const render = () => {
      if (!this.code) {
        body.innerHTML = `<div class="cm-mermaid-empty">Diagrama vacío. Añade contenido al bloque mermaid.</div>`;
        return;
      }

      const { svg, error } = renderMermaidSvg(this.code);
      if (error) {
        body.innerHTML = `
          <div class="cm-mermaid-error">
            <div class="cm-mermaid-error-title">
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <line x1="12" y1="8" x2="12" y2="12"/>
                <line x1="12" y1="16" x2="12.01" y2="16"/>
              </svg>
              <span>Error de sintaxis Mermaid</span>
            </div>
            <pre>${escapeHtml(error)}</pre>
          </div>
        `;
      } else if (svg) {
        body.innerHTML = `<div class="cm-mermaid-svg-container">${svg}</div>`;
      } else {
        body.innerHTML = `<div class="cm-mermaid-loading">Generando diagrama...</div>`;
      }
    };

    if (isMermanInitialized()) {
      render();
    } else {
      ensureMerman()
        .then(() => {
          render();
          view.requestMeasure();
        })
        .catch((err) => {
          body.innerHTML = `<div class="cm-mermaid-error">Error al iniciar Merman: ${escapeHtml(String(err))}</div>`;
        });
    }

    container.appendChild(body);
    return container;
  }

  ignoreEvent(): boolean {
    return true;
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

function buildMermaidDecorations(state: EditorState): DecorationSet {
  const widgets: any[] = [];
  const blocks = findMermaidBlocks(state.doc);
  const selectionRanges = state.selection.ranges;

  for (const block of blocks) {
    const hasCursor = selectionRanges.some(
      (r) => r.from <= block.to && r.to >= block.from
    );

    if (hasCursor) {
      // Si el cursor está dentro del bloque: se muestran las líneas de código para editar
      // y se adjunta la vista previa interactiva abajo con un widget de bloque
      const deco = Decoration.widget({
        widget: new MermaidWidget(block.code, block.from, block.to, block.codeFrom, false),
        side: 1,
        block: true,
      });
      widgets.push(deco.range(block.to));
    } else {
      // Si el cursor NO está dentro del bloque: sustituimos por completo el bloque
      // de código Markdown por la tarjeta interactiva del diagrama
      const deco = Decoration.replace({
        widget: new MermaidWidget(block.code, block.from, block.to, block.codeFrom, true),
        block: true,
      });
      widgets.push(deco.range(block.from, block.to));
    }
  }

  // Ordenar rangos de menor a mayor inicio para Decoration.set
  widgets.sort((a, b) => a.from - b.from);
  return Decoration.set(widgets, true);
}

/**
 * En CodeMirror 6, los reemplazos de bloques multilinea y decoraciones de bloque
 * DEBEN definirse mediante un StateField (y no un ViewPlugin) para que el motor
 * de renderizado del editor permita sustitución de bloques y cálculo de saltos de línea.
 */
export const mermaidLivePreviewField = StateField.define<DecorationSet>({
  create(state: EditorState): DecorationSet {
    return buildMermaidDecorations(state);
  },
  update(decorations: DecorationSet, tr): DecorationSet {
    if (tr.docChanged || tr.selection) {
      return buildMermaidDecorations(tr.state);
    }
    return decorations.map(tr.changes);
  },
  provide: (field) => EditorView.decorations.from(field),
});
