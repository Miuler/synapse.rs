<script lang="ts">
  import { onMount, onDestroy, untrack, tick } from 'svelte';
  import { basicSetup } from 'codemirror';
  import { EditorView, keymap } from '@codemirror/view';
  import { EditorState, Compartment } from '@codemirror/state';
  import { markdown } from '@codemirror/lang-markdown';
  import { mermaidLanguageDescription } from 'codemirror-lang-mermaid';
  import { syntaxHighlighting, HighlightStyle } from '@codemirror/language';
  import { tags } from '@lezer/highlight';
  import { vim } from '@replit/codemirror-vim';
  import { openSearchPanel, searchKeymap } from '@codemirror/search';
  import { mermaidLivePreviewField, activeFilePathFacet } from '../lib/mermaid-extension';
  import { imageLivePreviewField } from '../lib/image-extension';
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
    filePath?: string | null;
    isMarkdown?: boolean;
    readOnly?: boolean;
    vimMode?: boolean;
    viewMode?: MarkdownViewMode;
    onChange?: (markdown: string) => void;
    onSelectionChange?: (info: SelectionInfo) => void;
  }

  let {
    content = '',
    filePath = null,
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
  const filePathCompartment = new Compartment();

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

  // 1. Estilos y tema para modo Vista Previa en Vivo (Live Preview interactivo estilo Obsidian)
  const livePreviewHighlightStyle = HighlightStyle.define([
    { tag: tags.heading1, fontSize: '1.8em', fontWeight: '700', color: 'var(--text-primary, #1f2328)' },
    { tag: tags.heading2, fontSize: '1.45em', fontWeight: '700', color: 'var(--text-primary, #1f2328)' },
    { tag: tags.heading3, fontSize: '1.25em', fontWeight: '600', color: 'var(--text-primary, #1f2328)' },
    { tag: tags.heading, fontWeight: '600', color: 'var(--text-primary, #1f2328)' },
    { tag: tags.processingInstruction, color: 'var(--text-secondary, #8c959f)', opacity: '0.6', fontWeight: '400' },
    { tag: tags.emphasis, fontStyle: 'italic' },
    { tag: tags.strong, fontWeight: '700', color: 'var(--text-primary, #1f2328)' },
    { tag: tags.strikethrough, textDecoration: 'line-through', color: 'var(--text-secondary, #656d76)' },
    { tag: tags.link, color: 'var(--accent, #0969da)', textDecoration: 'underline' },
    { tag: tags.url, color: 'var(--accent, #0969da)' },
    { tag: tags.monospace, fontFamily: 'var(--code-font, monospace)', color: 'var(--accent, #0969da)' },
    { tag: tags.quote, color: 'var(--text-secondary, #656d76)', fontStyle: 'italic', borderLeft: '3px solid var(--border-primary, #d0d7de)', paddingLeft: '8px' },
    { tag: tags.keyword, color: '#cf222e', fontWeight: '600' },
    { tag: tags.comment, color: '#6e7781', fontStyle: 'italic' },
    { tag: tags.content, color: 'var(--text-primary, #1f2328)' },
  ]);

  const livePreviewTheme = EditorView.theme({
    '&': {
      fontFamily: 'var(--main-font, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif) !important',
      fontSize: '15.5px !important',
      lineHeight: '1.75 !important',
    },
    '.cm-scroller': {
      fontFamily: 'inherit !important',
    },
    '.cm-content': {
      fontFamily: 'inherit !important',
    },
    '.cm-line': {
      fontFamily: 'var(--main-font, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif) !important',
      fontSize: '15.5px !important',
      lineHeight: '1.75 !important',
      padding: '0 4px',
    },
  });

  const livePreviewExtensions = [
    livePreviewTheme,
    syntaxHighlighting(livePreviewHighlightStyle),
    mermaidLivePreviewField,
    imageLivePreviewField,
  ];

  // 2. Estilos y tema para modo Fuente puro (Source Mode monospaciado sin sustituciones de diagramas)
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
    { tag: tags.processingInstruction, color: 'var(--accent, #0969da)' },
  ]);

  const sourceModeTheme = EditorView.theme({
    '&': {
      fontFamily: 'var(--code-font, ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace) !important',
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
      lineHeight: '1.6 !important',
      padding: '0 4px',
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
      maxWidth: '100%',
      boxSizing: 'border-box',
      minHeight: '100%',
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
    /* Estilos del widget de imagen interactivo en CodeMirror */
    '.cm-image-widget-wrapper': {
      display: 'block',
      margin: '12px 0 16px 0',
      textAlign: 'center',
    },
    '.cm-image-card': {
      display: 'inline-block',
      maxWidth: '100%',
      position: 'relative',
      borderRadius: '8px',
      overflow: 'hidden',
      border: '1px solid var(--border-primary, #d0d7de)',
      backgroundColor: 'var(--bg-primary, #ffffff)',
      boxShadow: '0 2px 8px rgba(0, 0, 0, 0.04)',
      transition: 'border-color 0.2s ease, box-shadow 0.2s ease',
    },
    '.cm-image-card:hover': {
      borderColor: 'rgba(9, 105, 218, 0.4)',
      boxShadow: '0 4px 12px rgba(0, 0, 0, 0.08)',
    },
    '.cm-image-actions': {
      position: 'absolute',
      top: '8px',
      right: '8px',
      opacity: '0',
      transition: 'opacity 0.2s ease',
      zIndex: '2',
    },
    '.cm-image-card:hover .cm-image-actions': {
      opacity: '1',
    },
    '.cm-image-action-btn': {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '4px',
      fontSize: '11px',
      fontWeight: '500',
      padding: '3px 8px',
      borderRadius: '4px',
      border: '1px solid var(--border-primary, #d0d7de)',
      backgroundColor: 'rgba(255, 255, 255, 0.92)',
      backdropFilter: 'blur(4px)',
      color: 'var(--text-secondary, #656d76)',
      cursor: 'pointer',
      boxShadow: '0 1px 3px rgba(0,0,0,0.1)',
    },
    '.cm-image-action-btn:hover': {
      color: 'var(--accent, #0969da)',
      borderColor: 'var(--accent, #0969da)',
      backgroundColor: '#ffffff',
    },
    '.cm-image-content': {
      position: 'relative',
      minHeight: '40px',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      padding: '4px',
    },
    '.cm-image-element': {
      display: 'block',
      maxWidth: '100%',
      height: 'auto',
      borderRadius: '6px',
      cursor: 'pointer',
    },
    '.cm-image-caption': {
      padding: '6px 12px',
      fontSize: '12px',
      fontStyle: 'italic',
      color: 'var(--text-secondary, #656d76)',
      backgroundColor: 'var(--bg-secondary, #f6f8fa)',
      borderTop: '1px solid var(--border-primary, #d0d7de)',
      textAlign: 'center',
    },
    '.cm-image-loading': {
      display: 'flex',
      alignItems: 'center',
      gap: '8px',
      padding: '20px 28px',
      color: 'var(--text-secondary, #656d76)',
      fontSize: '12.5px',
    },
    '.cm-image-spinner': {
      width: '14px',
      height: '14px',
      border: '2px solid var(--border-primary, #d0d7de)',
      borderTopColor: 'var(--accent, #0969da)',
      borderRadius: '50%',
      animation: 'spin 0.6s linear infinite',
    },
    '.cm-image-error': {
      display: 'flex',
      alignItems: 'center',
      gap: '8px',
      padding: '12px 18px',
      color: '#cf222e',
      backgroundColor: '#ffebe9',
      fontSize: '12px',
    },
    '.cm-image-error code': {
      fontFamily: 'var(--code-font, monospace)',
      fontSize: '11px',
    },
    /* Estilos del widget de diagrama Mermaid dentro de CodeMirror */
    '.cm-mermaid-widget-wrapper': {
      margin: '14px 0 18px 0',
      border: '1px solid var(--border-primary, #d0d7de)',
      borderRadius: '8px',
      overflow: 'hidden',
      backgroundColor: 'var(--bg-primary, #ffffff)',
      boxShadow: '0 2px 8px rgba(0, 0, 0, 0.04)',
      transition: 'border-color 0.2s ease, box-shadow 0.2s ease',
    },
    '.cm-mermaid-widget-wrapper:hover': {
      borderColor: 'rgba(9, 105, 218, 0.4)',
      boxShadow: '0 4px 14px rgba(0, 0, 0, 0.07)',
    },
    '.cm-mermaid-widget-wrapper.is-live-edit': {
      borderStyle: 'dashed',
      borderColor: 'var(--accent, #0969da)',
    },
    '.cm-mermaid-header': {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      padding: '6px 12px',
      backgroundColor: 'var(--bg-secondary, #f6f8fa)',
      borderBottom: '1px solid var(--border-primary, #d0d7de)',
    },
    '.cm-mermaid-badge': {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '6px',
      fontSize: '11px',
      fontWeight: '600',
      color: 'var(--text-secondary, #656d76)',
    },
    '.cm-mermaid-badge svg': {
      color: 'var(--accent, #0969da)',
    },
    '.cm-mermaid-include-pill': {
      fontSize: '9.5px',
      fontWeight: '700',
      letterSpacing: '0.03em',
      textTransform: 'uppercase',
      padding: '1px 6px',
      borderRadius: '4px',
      backgroundColor: 'rgba(9, 105, 218, 0.12)',
      color: 'var(--accent, #0969da)',
    },
    '.cm-mermaid-actions': {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '6px',
    },
    '.cm-mermaid-edit-btn': {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '4px',
      fontSize: '11px',
      fontWeight: '500',
      padding: '2px 8px',
      borderRadius: '4px',
      border: '1px solid var(--border-primary, #d0d7de)',
      backgroundColor: 'var(--bg-primary, #ffffff)',
      color: 'var(--text-secondary, #656d76)',
      cursor: 'pointer',
      transition: 'all 0.15s ease',
    },
    '.cm-mermaid-edit-btn:hover': {
      color: 'var(--accent, #0969da)',
      borderColor: 'var(--accent, #0969da)',
      backgroundColor: 'rgba(9, 105, 218, 0.05)',
    },
    '.cm-mermaid-live-pill': {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '5px',
      fontSize: '10.5px',
      fontWeight: '600',
      padding: '2px 8px',
      borderRadius: '999px',
      backgroundColor: 'rgba(9, 105, 218, 0.1)',
      color: 'var(--accent, #0969da)',
    },
    '.cm-mermaid-live-dot': {
      width: '6px',
      height: '6px',
      borderRadius: '50%',
      backgroundColor: 'var(--accent, #0969da)',
      boxShadow: '0 0 4px var(--accent, #0969da)',
    },
    '.cm-mermaid-body': {
      padding: '16px',
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
      minHeight: '80px',
      backgroundColor: 'var(--bg-primary, #ffffff)',
    },
    '.cm-mermaid-svg-container': {
      width: '100%',
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
      overflowX: 'auto',
    },
    '.cm-mermaid-svg-container svg': {
      maxWidth: '100%',
      height: 'auto',
    },
    '.cm-mermaid-empty': {
      fontSize: '12px',
      fontStyle: 'italic',
      color: 'var(--text-secondary, #656d76)',
    },
    '.cm-mermaid-error': {
      width: '100%',
      padding: '10px 14px',
      backgroundColor: '#ffebe9',
      border: '1px solid rgba(207, 34, 46, 0.3)',
      borderRadius: '6px',
      color: '#cf222e',
      fontSize: '12px',
    },
    '.cm-mermaid-error-title': {
      fontWeight: '600',
      display: 'flex',
      alignItems: 'center',
      gap: '6px',
      marginBottom: '6px',
    },
    '.cm-mermaid-error pre': {
      margin: '0',
      whiteSpace: 'pre-wrap',
      wordBreak: 'break-word',
      fontFamily: 'var(--code-font, monospace)',
      fontSize: '11px',
    },
    '.cm-mermaid-loading': {
      fontSize: '12px',
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
        filePathCompartment.of(activeFilePathFacet.of(filePath)),
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

  // Reaccionar a cambios en filePath para el contexto de includes
  $effect(() => {
    const fp = filePath;
    if (editorView) {
      editorView.dispatch({
        effects: filePathCompartment.reconfigure(activeFilePathFacet.of(fp)),
      });
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
      if (mode !== 'reading') {
        tick().then(() => {
          editorView?.requestMeasure();
        });
      }
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
    <MarkdownReadingView {content} {filePath} />
  {/if}
  <div
    class="editor-container"
    class:hidden={!isMarkdown || viewMode === 'reading'}
    bind:this={containerRef}
  ></div>
  {#if !isMarkdown}
    <pre class="plain-text">{content}</pre>
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
