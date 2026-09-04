<script lang="ts">
  import { onMount, onDestroy, untrack } from 'svelte';
  import { basicSetup } from 'codemirror';
  import { EditorView, keymap } from '@codemirror/view';
  import { EditorState, Compartment } from '@codemirror/state';
  import { markdown } from '@codemirror/lang-markdown';
  import { mermaidLanguageDescription } from 'codemirror-lang-mermaid';
  import { syntaxHighlighting, HighlightStyle } from '@codemirror/language';
  import { tags } from '@lezer/highlight';
  import { vim } from '@replit/codemirror-vim';
  import { openSearchPanel, searchKeymap } from '@codemirror/search';
  import { mermaidLivePreviewPlugin } from '../lib/mermaid-extension';
  import MarkdownReadingView from './MarkdownReadingView.svelte';

  export interface SelectionInfo {
    hasSelection: boolean;
    selectedWords: number;
    selectedChars: number;
    selectedLines: number;
    selectedCols: number;
    cursorLine: number;
    cursorCol: number;
  }

  export type MarkdownViewMode = 'live' | 'source' | 'reading';

  interface Props {
    content: string;
    isMarkdown?: boolean;
    readOnly?: boolean;
    vimMode?: boolean;
    viewMode?: MarkdownViewMode;
    onChange?: (markdown: string) => void;
    onSelectionChange?: (info: SelectionInfo) => void;
  }

  let {
    content = '',
    isMarkdown = true,
    readOnly = false,
    vimMode = false,
    viewMode = 'live',
    onChange,
    onSelectionChange,
  }: Props = $props();

  let containerRef = $state<HTMLDivElement | null>(null);
  let editorView: EditorView | null = null;
  let isInternalUpdate = false;
  let lastContent = $state(untrack(() => content));

  const readOnlyCompartment = new Compartment();
  const vimCompartment = new Compartment();
  const viewModeCompartment = new Compartment();

  export function triggerSearch() {
    if (editorView) {
      openSearchPanel(editorView as any);
    }
  }

  export function getView(): EditorView | null {
    return editorView;
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

  // 1. Estilos y tema para modo Vista Previa en Vivo (Live Preview con estilos y diagramas)
  const livePreviewHighlightStyle = HighlightStyle.define([
    { tag: tags.heading1, fontSize: '1.6em', fontWeight: 'bold', color: 'var(--text-primary, #1f2328)' },
    { tag: tags.heading2, fontSize: '1.35em', fontWeight: 'bold', color: 'var(--text-primary, #1f2328)' },
    { tag: tags.heading3, fontSize: '1.18em', fontWeight: 'bold', color: 'var(--text-primary, #1f2328)' },
    { tag: tags.heading, fontWeight: 'bold', color: 'var(--text-primary, #1f2328)' },
    { tag: tags.emphasis, fontStyle: 'italic' },
    { tag: tags.strong, fontWeight: 'bold' },
    { tag: tags.strikethrough, textDecoration: 'line-through' },
    { tag: tags.link, color: 'var(--accent, #0969da)', textDecoration: 'underline' },
    { tag: tags.url, color: 'var(--accent, #0969da)' },
    { tag: tags.monospace, fontFamily: 'var(--code-font, monospace)', color: '#0969da' },
    { tag: tags.quote, color: 'var(--text-secondary, #656d76)', fontStyle: 'italic' },
    { tag: tags.keyword, color: '#cf222e', fontWeight: 'bold' },
    { tag: tags.comment, color: '#6e7781', fontStyle: 'italic' },
    { tag: tags.content, color: 'var(--text-primary, #1f2328)' },
  ]);

  const livePreviewTheme = EditorView.theme({
    '&': {
      fontFamily: 'var(--main-font, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif) !important',
      fontSize: '15px !important',
      lineHeight: '1.65 !important',
    },
    '.cm-scroller': {
      fontFamily: 'inherit !important',
    },
    '.cm-content': {
      fontFamily: 'inherit !important',
    },
  });

  const livePreviewExtensions = [
    livePreviewTheme,
    syntaxHighlighting(livePreviewHighlightStyle),
    mermaidLivePreviewPlugin,
  ];

  // 2. Estilos y tema para modo Fuente puro (Source Mode sin widgets de diagramas)
  const sourceModeHighlightStyle = HighlightStyle.define([
    { tag: tags.heading, fontWeight: 'bold', color: 'var(--accent, #0969da)' },
    { tag: tags.emphasis, fontStyle: 'italic', color: 'var(--text-primary, #1f2328)' },
    { tag: tags.strong, fontWeight: 'bold', color: 'var(--text-primary, #1f2328)' },
    { tag: tags.strikethrough, textDecoration: 'line-through' },
    { tag: tags.link, color: 'var(--accent, #0969da)', textDecoration: 'underline' },
    { tag: tags.url, color: 'var(--accent, #0969da)' },
    { tag: tags.monospace, color: '#0969da' },
    { tag: tags.quote, color: 'var(--text-secondary, #656d76)' },
    { tag: tags.keyword, color: '#cf222e', fontWeight: 'bold' },
    { tag: tags.comment, color: '#6e7781', fontStyle: 'italic' },
    { tag: tags.content, color: 'var(--text-primary, #1f2328)' },
  ]);

  const sourceModeTheme = EditorView.theme({
    '&': {
      fontFamily: 'var(--code-font, ui-monospace, SFMono-Regular, \"SF Mono\", Menlo, Consolas, \"Liberation Mono\", monospace) !important',
      fontSize: '13.5px !important',
      lineHeight: '1.6 !important',
    },
    '.cm-scroller': {
      fontFamily: 'inherit !important',
    },
    '.cm-content': {
      fontFamily: 'inherit !important',
    },
    '.cm-line': {
      fontFamily: 'inherit !important',
      fontSize: '13.5px !important',
    },
  });

  const sourceModeExtensions = [
    sourceModeTheme,
    syntaxHighlighting(sourceModeHighlightStyle),
  ];

  const baseEditorTheme = EditorView.theme({
    '&': {
      height: '100%',
      width: '100%',
      backgroundColor: 'transparent',
      color: 'var(--text-primary, #1f2328)',
    },
    '.cm-scroller': {
      overflow: 'auto',
    },
    '.cm-content': {
      padding: '16px 24px',
      maxWidth: '900px',
      minHeight: '100%',
    },
    '.cm-line': {
      padding: '0 4px',
    },
    '.cm-gutters': {
      backgroundColor: 'transparent',
      color: 'var(--text-secondary, #8c959f)',
      borderRight: '1px solid var(--border-primary, #d0d7de)',
      paddingRight: '4px',
    },
    '.cm-activeLine': {
      backgroundColor: 'rgba(9, 105, 218, 0.04)',
    },
    '.cm-activeLineGutter': {
      backgroundColor: 'rgba(9, 105, 218, 0.08)',
      color: 'var(--accent, #0969da)',
    },
    '.cm-cursor, .cm-dropCursor': {
      borderLeftColor: 'var(--text-primary, #1f2328)',
    },
    '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': {
      backgroundColor: 'rgba(9, 105, 218, 0.2) !important',
    },
    '.cm-panel.cm-search': {
      backgroundColor: 'var(--bg-secondary, #f6f8fa)',
      color: 'var(--text-primary, #1f2328)',
      borderBottom: '1px solid var(--border-primary, #d0d7de)',
      padding: '6px 12px',
    },
    '.cm-textfield': {
      backgroundColor: 'var(--bg-primary, #ffffff)',
      color: 'var(--text-primary, #1f2328)',
      border: '1px solid var(--border-primary, #d0d7de)',
      borderRadius: '4px',
      padding: '3px 8px',
    },
    '.cm-button': {
      backgroundColor: 'var(--bg-primary, #ffffff)',
      color: 'var(--text-primary, #1f2328)',
      border: '1px solid var(--border-primary, #d0d7de)',
      borderRadius: '4px',
      backgroundImage: 'none',
      padding: '3px 10px',
      cursor: 'pointer',
    },
    '.cm-button:hover': {
      backgroundColor: 'rgba(9, 105, 218, 0.1)',
      borderColor: 'var(--accent, #0969da)',
    },
    '&.cm-focused': {
      outline: 'none',
    },
    /* Estilos del widget de diagrama Mermaid dentro de CodeMirror */
    '.cm-mermaid-widget-wrapper': {
      margin: '12px 0 16px 0',
      border: '1px solid var(--border-primary, #d0d7de)',
      borderRadius: '8px',
      overflow: 'hidden',
      backgroundColor: 'var(--bg-primary, #ffffff)',
      boxShadow: '0 2px 8px rgba(0, 0, 0, 0.04)',
    },
    '.cm-mermaid-header': {
      display: 'flex',
      alignItems: 'center',
      padding: '4px 10px',
      backgroundColor: 'var(--bg-secondary, #f6f8fa)',
      borderBottom: '1px solid var(--border-primary, #d0d7de)',
    },
    '.cm-mermaid-badge': {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '4px',
      fontSize: '11px',
      fontWeight: '600',
      color: 'var(--text-secondary, #656d76)',
    },
    '.cm-mermaid-body': {
      padding: '12px',
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
      minHeight: '60px',
      backgroundColor: 'var(--bg-primary, #ffffff)',
    },
    '.cm-mermaid-svg-container': {
      width: '100%',
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
    },
    '.cm-mermaid-svg-container svg': {
      maxWidth: '100%',
      height: 'auto',
      pointerEvents: 'none',
    },
    '.cm-mermaid-error': {
      width: '100%',
      padding: '8px 12px',
      backgroundColor: '#ffebe9',
      border: '1px solid rgba(207, 34, 46, 0.3)',
      borderRadius: '6px',
      color: '#cf222e',
      fontSize: '11px',
    },
    '.cm-mermaid-error-title': {
      fontWeight: '600',
      display: 'block',
      marginBottom: '4px',
    },
    '.cm-mermaid-error pre': {
      margin: '0',
      whiteSpace: 'pre-wrap',
      fontFamily: 'var(--code-font, monospace)',
    },
    '.cm-mermaid-loading': {
      fontSize: '11px',
      color: 'var(--text-secondary, #656d76)',
    },
  });

  function getActiveExtensions(mode: MarkdownViewMode) {
    return mode === 'source' ? sourceModeExtensions : livePreviewExtensions;
  }

  function initEditor() {
    if (!containerRef || editorView) return;

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

    const state = EditorState.create({
      doc: content,
      extensions: [
        vimCompartment.of(vimMode ? vim() : []),
        readOnlyCompartment.of(EditorView.editable.of(!readOnly)),
        viewModeCompartment.of(getActiveExtensions(viewMode)),
        basicSetup,
        markdown({ codeLanguages: [mermaidLanguageDescription] }),
        EditorView.lineWrapping,
        keymap.of(searchKeymap as any),
        updateListener,
        baseEditorTheme,
      ],
    });

    editorView = new EditorView({
      state,
      parent: containerRef,
    });

    emitSelectionInfo(editorView.state);
  }

  onMount(() => {
    if (isMarkdown && containerRef) {
      initEditor();
    }
  });

  onDestroy(() => {
    if (editorView) {
      editorView.destroy();
      editorView = null;
    }
  });

  // Reaccionar a cambios en vimMode
  $effect(() => {
    const isVim = vimMode;
    if (editorView) {
      editorView.dispatch({
        effects: vimCompartment.reconfigure(isVim ? vim() : []),
      });
    }
  });

  // Reaccionar a cambios en readOnly
  $effect(() => {
    const ro = readOnly;
    if (editorView) {
      editorView.dispatch({
        effects: readOnlyCompartment.reconfigure(EditorView.editable.of(!ro)),
      });
    }
  });

  // Reaccionar a cambios en viewMode (Modo Fuente vs Modo Vista Previa)
  $effect(() => {
    const mode = viewMode;
    if (editorView) {
      editorView.dispatch({
        effects: viewModeCompartment.reconfigure(getActiveExtensions(mode)),
      });
    }
  });

  // Reaccionar a cambios en el contenido recibido por props
  $effect(() => {
    const c = content;
    const activeMarkdown = isMarkdown;

    if (activeMarkdown && containerRef && !editorView) {
      initEditor();
      return;
    }

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

<div class="editor-wrapper">
  {#if isMarkdown && viewMode === 'reading'}
    <MarkdownReadingView {content} />
  {:else}
    <div class="editor-container" class:hidden={!isMarkdown} bind:this={containerRef}></div>
    {#if !isMarkdown}
      <pre class="plain-text">{content}</pre>
    {/if}
  {/if}
</div>

<style>
  .editor-wrapper {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    background-color: transparent;
  }

  .editor-container {
    width: 100%;
    height: 100%;
    min-height: 0;
    overflow: hidden;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
  }

  .editor-container.hidden {
    display: none;
  }

  .plain-text {
    font-family: var(--code-font, monospace);
    white-space: pre-wrap;
    color: var(--text-primary);
    padding: 16px 24px;
    overflow: auto;
    height: 100%;
  }
</style>
