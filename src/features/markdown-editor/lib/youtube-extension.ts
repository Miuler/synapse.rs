import { EditorView, Decoration, type DecorationSet, WidgetType } from '@codemirror/view';
import { StateField, type EditorState } from '@codemirror/state';
import { extractYouTubeTokens, getYouTubeThumbnailUrl } from './youtube';

class YouTubeWidget extends WidgetType {
  constructor(
    readonly id: string,
    readonly url: string,
  ) {
    super();
  }

  eq(other: YouTubeWidget): boolean {
    return other.id === this.id && other.url === this.url;
  }

  toDOM(view: EditorView): HTMLElement {
    const container = document.createElement('div');
    container.className = 'cm-youtube-widget-wrapper';

    const link = document.createElement('a');
    link.className = 'cm-youtube-card';
    link.href = this.url;
    link.target = '_blank';
    link.rel = 'noopener noreferrer';
    link.title = 'Abrir en YouTube';

    const img = document.createElement('img');
    img.className = 'cm-youtube-thumb';
    img.alt = 'Miniatura de YouTube';
    img.loading = 'lazy';
    img.onload = () => view.requestMeasure();
    img.onerror = () => {
      container.remove();
      view.requestMeasure();
    };
    img.src = getYouTubeThumbnailUrl(this.id);

    const play = document.createElement('span');
    play.className = 'cm-youtube-play';
    play.textContent = '▶';

    link.append(img, play);
    container.appendChild(link);
    return container;
  }

  ignoreEvent(): boolean {
    return false;
  }
}

function buildYouTubeDecorations(state: EditorState): DecorationSet {
  try {
    const widgets: any[] = [];
    const doc = state.doc;
    let inFence = false;

    for (let i = 1; i <= doc.lines; i++) {
      const line = doc.line(i);
      if (/^\s*(?:```|~~~)/.test(line.text)) {
        inFence = !inFence;
        continue;
      }
      if (inFence) continue;

      const token = extractYouTubeTokens(line.text, line.from)[0];
      if (!token) continue;

      const deco = Decoration.widget({
        widget: new YouTubeWidget(token.id, token.url),
        block: true,
        side: 1,
      });
      widgets.push(deco.range(line.to));
    }

    return Decoration.set(widgets, true);
  } catch (err) {
    console.error('Error al generar miniaturas de YouTube en vivo:', err);
    return Decoration.none;
  }
}

export const youtubeLivePreviewField = StateField.define<DecorationSet>({
  create(state: EditorState): DecorationSet {
    return buildYouTubeDecorations(state);
  },
  update(decorations: DecorationSet, tr): DecorationSet {
    if (tr.docChanged) {
      return buildYouTubeDecorations(tr.state);
    }
    return decorations;
  },
  provide: (field) => EditorView.decorations.from(field),
});
