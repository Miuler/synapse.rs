<script lang="ts">
  import { onMount, onDestroy, untrack } from 'svelte';
  import { basicSetup } from 'codemirror';
  import { EditorView, keymap } from '@codemirror/view';
  import { EditorState, Compartment } from '@codemirror/state';
  import { markdown } from '@codemirror/lang-markdown';
  import { syntaxHighlighting, HighlightStyle } from '@codemirror/language';
  import { tags } from '@lezer/highlight';
  import { vim } from '@replit/codemirror-vim';
  import { openSearchPanel, searchKeymap } from '@codemirror/search';

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
    isMarkdown?: boolean;
    readOnly?: boolean;
    vimMode?: boolean;
    onChange?: (markdown: string) => void;
    onSelectionChange?: (info: SelectionInfo) => void;
  }

  let {
    content = '',
    isMarkdown = true,
    readOnly = false,
    vimMode = false,
    onChange,
    onSelectionChange,
  }: Props = $props();

  let containerRef = $state<HTMLDivElement | null>(null);
  let editorView: EditorView | null = null;
  let isInternalUpdate = false;
  let lastContent = $state(untrack(() => content));

  const readOnlyCompartment = new Compartment();
  const vimCompartment = new Compartment();

  export function triggerSearch() {
    if (editorView) {
      openSearchPanel(editorView);
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

  const markdownHighlightStyle = HighlightStyle.define([
    { tag: tags.heading1, fontSize: '1.5em', fontWeight: 'bold', color: 'var(--text-primary, #1f2328)' },
    { tag: tags.heading2, fontSize: '1.3em', fontWeight: 'bold', color: 'var(--text-primary, #1f2328)' },
    { tag: tags.heading3, fontSize: '1.15em', fontWeight: 'bold', color: 'var(--text-primary, #1f2328)' },
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

  const editorTheme = EditorView.theme({
    '&': {
      height: '100%',
      width: '100%',
      fontSize: '15px',
      lineHeight: '1.65',
      fontFamily: 'var(--main-font, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif)',
      backgroundColor: 'transparent',
      color: 'var(--text-primary, #1f2328)',
    },
    '.cm-scroller': {
      overflow: 'auto',
      fontFamily: 'inherit',
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
  });

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
        basicSetup,
        markdown(),
        EditorView.lineWrapping,
        syntaxHighlighting(markdownHighlightStyle),
        keymap.of(searchKeymap),
        updateListener,
        editorTheme,
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
  <div class="editor-container" class:hidden={!isMarkdown} bind:this={containerRef}></div>
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
