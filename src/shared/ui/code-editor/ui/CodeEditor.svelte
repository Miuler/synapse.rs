<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { basicSetup } from 'codemirror';
  import { EditorView, keymap } from '@codemirror/view';
  import { EditorState, Compartment } from '@codemirror/state';
  import { markdown } from '@codemirror/lang-markdown';
  import { StreamLanguage, syntaxHighlighting, HighlightStyle } from '@codemirror/language';
  import { tags } from '@lezer/highlight';
  import { searchKeymap, search, openSearchPanel } from '@codemirror/search';
  import { vim } from '@replit/codemirror-vim';

  export interface SelectionInfo {
    hasSelection: boolean;
    selectedWords: number;
    selectedChars: number;
    selectedLines: number;
    selectedCols: number;
    cursorLine: number;
    cursorCol: number;
  }

  interface Props {
    content: string;
    readOnly?: boolean;
    vimMode?: boolean;
    mode?: 'markdown' | 'mermaid';
    onChange?: (content: string) => void;
    onSelectionChange?: (info: SelectionInfo) => void;
  }

  let {
    content = '',
    readOnly = false,
    vimMode = false,
    mode = 'markdown',
    onChange,
    onSelectionChange,
  }: Props = $props();

  let containerRef = $state<HTMLDivElement | null>(null);
  let editorView: EditorView | null = null;
  let isInternalUpdate = false;
  const vimCompartment = new Compartment();

  let lastContent = $state(untrack(() => content));

  export function triggerSearch() {
    if (editorView) {
      openSearchPanel(editorView as any);
    }
  }

  function emitSelectionInfo(state: EditorState) {
    if (!onSelectionChange) return;

    const mainSel = state.selection.main;
    const hasSelection = !mainSel.empty;
    const head = mainSel.head;
    const cursorLineObj = state.doc.lineAt(head);
    const cursorLine = cursorLineObj.number;
    const cursorCol = head - cursorLineObj.from + 1;

    if (hasSelection) {
      const from = mainSel.from;
      const to = mainSel.to;
      const selectedText = state.sliceDoc(from, to);
      const fromLine = state.doc.lineAt(from);
      const toLine = state.doc.lineAt(to);
      const selectedLines = toLine.number - fromLine.number + 1;
      const selectedChars = selectedText.length;
      const selectedCols = selectedLines === 1 ? (to - from) : selectedChars;
      const trimmed = selectedText.trim();
      const selectedWords = trimmed ? trimmed.split(/\s+/).length : 0;

      onSelectionChange({
        hasSelection: true,
        selectedWords,
        selectedChars,
        selectedLines,
        selectedCols,
        cursorLine,
        cursorCol,
      });
    } else {
      onSelectionChange({
        hasSelection: false,
        selectedWords: 0,
        selectedChars: 0,
        selectedLines: 1,
        selectedCols: 0,
        cursorLine,
        cursorCol,
      });
    }
  }

  // Lenguaje Mermaid personalizado con StreamLanguage
  const mermaidLanguage = StreamLanguage.define({
    name: 'mermaid',
    startState() {
      return {};
    },
    token(stream) {
      if (stream.eatSpace()) return null;

      // Comentarios %% ...
      if (stream.match(/^%%.*/)) {
        return 'comment';
      }

      // Palabras clave de tipos de diagrama
      if (stream.match(/^(flowchart|graph|sequenceDiagram|classDiagram|stateDiagram(-v2)?|erDiagram|gantt|pie|gitGraph|mindmap|timeline)\b/i)) {
        return 'keyword';
      }

      // Estructuras y palabras clave del flujo
      if (stream.match(/^(TD|TB|BT|RL|LR|subgraph|end|click|call|style|classDef|class|linkStyle|participant|actor|boundary|control|entity|database|collections|queue|box|autonumber|activate|deactivate|loop|alt|else|opt|par|critical|option|break|rect|note|over)\b/i)) {
        return 'typeName';
      }

      // Flechas y conectores
      if (stream.match(/^(-->|---|==>|-.->|->>|-->>|x--x|o--o|--)/)) {
        return 'operator';
      }

      // Texto entre comillas o corchetes
      if (stream.match(/^"[^"]*"/)) {
        return 'string';
      }

      // Nombres de nodos e identificadores
      if (stream.match(/^[a-zA-Z_][a-zA-Z0-9_]*/)) {
        return 'variableName';
      }

      stream.next();
      return null;
    }
  });

  // Tema de colores de sintaxis (Syntax Highlighting)
  const customHighlightStyle = HighlightStyle.define([
    { tag: tags.heading, color: '#0969da', fontWeight: 'bold' },
    { tag: tags.emphasis, fontStyle: 'italic' },
    { tag: tags.strong, fontWeight: 'bold' },
    { tag: tags.keyword, color: '#cf222e', fontWeight: 'bold' },
    { tag: tags.comment, color: '#6e7781', fontStyle: 'italic' },
    { tag: tags.string, color: '#0a3069' },
    { tag: tags.variableName, color: '#953800' },
    { tag: tags.typeName, color: '#116329', fontWeight: 'bold' },
    { tag: tags.link, color: '#0969da', textDecoration: 'underline' },
    { tag: tags.url, color: '#0969da' },
    { tag: tags.number, color: '#0550ae' },
    { tag: tags.operator, color: '#0550ae', fontWeight: 'bold' },
    { tag: tags.meta, color: '#8c959f' },
  ]);

  onMount(() => {
    if (!containerRef) return;

    const updateListener = EditorView.updateListener.of((update) => {
      if (update.docChanged && !isInternalUpdate) {
        const newText = update.state.doc.toString();
        lastContent = newText;
        if (onChange) onChange(newText);
      }
      if (update.selectionSet || update.docChanged) {
        emitSelectionInfo(update.state);
      }
    });

    const theme = EditorView.theme({
      '&': {
        height: '100%',
        width: '100%',
        fontSize: '13px',
        fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace',
        backgroundColor: 'var(--bg-primary, #ffffff)',
        color: 'var(--text-primary, #1f2328)',
      },
      '.cm-scroller': {
        overflow: 'auto',
        fontFamily: 'inherit',
      },
      '.cm-content': {
        padding: '8px 0',
      },
      '.cm-line': {
        padding: '0 8px',
      },
      '.cm-gutters': {
        backgroundColor: 'var(--bg-secondary, #f6f8fa)',
        color: 'var(--text-secondary, #656d76)',
        borderRight: '1px solid var(--border-primary, #d0d7de)',
      },
      '.cm-activeLine': {
        backgroundColor: 'rgba(9, 105, 218, 0.05)',
      },
      '.cm-activeLineGutter': {
        backgroundColor: 'rgba(9, 105, 218, 0.1)',
        color: 'var(--accent, #0969da)',
      },
      '.cm-panel.cm-search': {
        backgroundColor: 'var(--bg-secondary, #f6f8fa)',
        color: 'var(--text-primary, #1f2328)',
        borderBottom: '1px solid var(--border-primary, #d0d7de)',
        padding: '6px 10px',
      },
      '.cm-textfield': {
        backgroundColor: 'var(--bg-primary, #ffffff)',
        color: 'var(--text-primary, #1f2328)',
        border: '1px solid var(--border-primary, #d0d7de)',
        borderRadius: '4px',
        padding: '2px 6px',
      },
      '.cm-button': {
        backgroundColor: 'var(--bg-primary, #ffffff)',
        color: 'var(--text-primary, #1f2328)',
        border: '1px solid var(--border-primary, #d0d7de)',
        borderRadius: '4px',
        backgroundImage: 'none',
        padding: '2px 8px',
        cursor: 'pointer',
      },
      '.cm-button:hover': {
        backgroundColor: 'rgba(9, 105, 218, 0.1)',
        borderColor: 'var(--accent, #0969da)',
      },
      '&.cm-focused': {
        outline: 'none',
      },
    });

    const langExtension = mode === 'mermaid' ? mermaidLanguage : markdown();

    const state = EditorState.create({
      doc: content,
      extensions: [
        vimCompartment.of(vimMode ? vim() : []),
        basicSetup,
        langExtension,
        syntaxHighlighting(customHighlightStyle),
        search({ top: true }),
        keymap.of(searchKeymap as any),
        EditorView.editable.of(!readOnly),
        updateListener,
        theme,
      ],
    });

    editorView = new EditorView({
      state,
      parent: containerRef,
    });

    emitSelectionInfo(editorView.state);

    return () => {
      if (editorView) {
        editorView.destroy();
        editorView = null;
      }
    };
  });

  $effect(() => {
    const isVim = vimMode;
    if (editorView) {
      editorView.dispatch({
        effects: vimCompartment.reconfigure(isVim ? vim() : []),
      });
    }
  });

  $effect(() => {
    const c = content;
    if (editorView && c !== untrack(() => lastContent)) {
      lastContent = c;
      if (c !== editorView.state.doc.toString()) {
        isInternalUpdate = true;
        editorView.dispatch({
          changes: { from: 0, to: editorView.state.doc.length, insert: c },
        });
        isInternalUpdate = false;
        emitSelectionInfo(editorView.state);
      }
    }
  });
</script>

<div class="codemirror-container" bind:this={containerRef}></div>

<style>
  .codemirror-container {
    width: 100%;
    height: 100%;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }</style>
