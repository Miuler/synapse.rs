import { EditorView, ViewPlugin, ViewUpdate, Decoration, type DecorationSet, WidgetType } from '@codemirror/view';
import { syntaxTree } from '@codemirror/language';
import { renderMermaidSvg, ensureMerman } from '@features/merman-editor';

class MermaidWidget extends WidgetType {
  constructor(readonly code: string) {
    super();
  }

  eq(other: MermaidWidget) {
    return other.code === this.code;
  }

  toDOM(view: EditorView): HTMLElement {
    const container = document.createElement('div');
    container.className = 'cm-mermaid-widget-wrapper';

    const header = document.createElement('div');
    header.className = 'cm-mermaid-header';
    header.innerHTML = `
      <span class="cm-mermaid-badge">
        <svg viewBox="0 0 24 24" width="11" height="11" fill="none" stroke="currentColor" stroke-width="2">
          <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>
        </svg>
        Diagrama Mermaid
      </span>
    `;
    container.appendChild(header);

    const body = document.createElement('div');
    body.className = 'cm-mermaid-body';

    const render = () => {
      const { svg, error } = renderMermaidSvg(this.code);
      if (error) {
        body.innerHTML = `
          <div class="cm-mermaid-error">
            <span class="cm-mermaid-error-title">Error en sintaxis Mermaid</span>
            <pre>${escapeHtml(error)}</pre>
          </div>
        `;
      } else if (svg) {
        body.innerHTML = `<div class="cm-mermaid-svg-container">${svg}</div>`;
      } else {
        body.innerHTML = `<div class="cm-mermaid-loading">Generando diagrama...</div>`;
      }
    };

    ensureMerman()
      .then(() => {
        render();
        view.requestMeasure();
      })
      .catch((err) => {
        body.innerHTML = `<div class="cm-mermaid-error">Error al iniciar Merman: ${escapeHtml(String(err))}</div>`;
      });

    container.appendChild(body);
    return container;
  }

  ignoreEvent() {
    return false;
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

function buildMermaidDecorations(view: EditorView): DecorationSet {
  const widgets: any[] = [];
  const state = view.state;

  syntaxTree(state).iterate({
    enter(node) {
      if (node.name === 'FencedCode') {
        const text = state.sliceDoc(node.from, node.to);
        const match = text.match(/^```(?:mermaid|merman)\s*\n([\s\S]*?)\n?```$/);
        if (match) {
          const code = match[1].trim();
          if (code) {
            const deco = Decoration.widget({
              widget: new MermaidWidget(code),
              side: 1,
              block: true,
            });
            widgets.push(deco.range(node.to));
          }
        }
      }
    },
  });

  return Decoration.set(widgets, true);
}

export const mermaidLivePreviewPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = buildMermaidDecorations(view);
    }

    update(update: ViewUpdate) {
      if (update.docChanged || update.viewportChanged) {
        this.decorations = buildMermaidDecorations(update.view);
      }
    }
  },
  {
    decorations: (v) => v.decorations,
  }
);
