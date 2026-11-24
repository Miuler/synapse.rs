<script lang="ts">
  import { untrack } from 'svelte';
  import { basicSetup } from 'codemirror';
  import { Compartment, EditorState, type Extension } from '@codemirror/state';
  import { EditorView, keymap, tooltips } from '@codemirror/view';
  import {
    MergeView,
    unifiedMergeView,
    getChunks,
    goToNextChunk,
    goToPreviousChunk,
  } from '@codemirror/merge';
  import {
    search,
    searchKeymap,
    openSearchPanel,
  } from '@codemirror/search';
  import { autocompletion } from '@codemirror/autocomplete';
  import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
  import { tags } from '@lezer/highlight';
  import { yaml } from '@codemirror/lang-yaml';
  import { json } from '@codemirror/lang-json';
  import { sql } from '@codemirror/lang-sql';
  import { markdown } from '@codemirror/lang-markdown';
  import { vim } from '@replit/codemirror-vim';
  import { undoRedoExtension } from '@shared/lib/editor-history';
  import { commentExtension } from '@shared/lib/editor-comment';
  import {
    registerVimFolding,
    renameOccurrencesKeymap,
    lineKeymap,
    documentWordsCompletion,
  } from '@shared/ui/code-editor';

  registerVimFolding();

  export interface DiffChunksInfo {
    total: number;
    current: number;
  }

  interface Props {
    oldContent: string;
    newContent: string;
    fileName: string;
    mode: 'split' | 'unified';
    wrap?: boolean;
    collapseUnchanged?: boolean;
    theme?: 'light' | 'dark';
    editable?: boolean;
    vimMode?: boolean;
    onContentChange?: (content: string) => void;
    onChunksChange?: (info: DiffChunksInfo) => void;
  }

  let {
    oldContent,
    newContent,
    fileName,
    mode,
    wrap = false,
    collapseUnchanged = true,
    theme = 'dark',
    editable = false,
    vimMode = false,
    onContentChange,
    onChunksChange,
  }: Props = $props();

  let containerEl = $state<HTMLDivElement | null>(null);
  let activeMergeView: MergeView | null = null;
  let activeUnifiedView: EditorView | null = null;

  const vimCompartment = new Compartment();
  const readOnlyCompartment = new Compartment();
  const unifiedMergeCompartment = new Compartment();

  const diffHighlightStyle = HighlightStyle.define([
    { tag: tags.heading, color: 'var(--accent, #0969da)', fontWeight: 'bold' },
    { tag: tags.emphasis, fontStyle: 'italic' },
    { tag: tags.strong, fontWeight: 'bold' },
    { tag: tags.keyword, color: 'var(--syntax-keyword, #cf222e)', fontWeight: 'bold' },
    { tag: tags.comment, color: 'var(--syntax-comment, #6e7781)', fontStyle: 'italic' },
    { tag: tags.string, color: 'var(--syntax-string, #0a3069)' },
    { tag: tags.variableName, color: 'var(--syntax-variable, #953800)' },
    { tag: tags.typeName, color: 'var(--syntax-type, #116329)', fontWeight: 'bold' },
    { tag: tags.link, color: 'var(--accent, #0969da)', textDecoration: 'underline' },
    { tag: tags.url, color: 'var(--accent, #0969da)' },
    { tag: tags.number, color: 'var(--syntax-number, #0550ae)' },
    { tag: tags.operator, color: 'var(--syntax-number, #0550ae)', fontWeight: 'bold' },
    { tag: tags.meta, color: 'var(--text-secondary, #8c959f)' },
    { tag: [tags.propertyName, tags.definition(tags.propertyName)], color: 'var(--syntax-variable, #953800)' },
    { tag: [tags.bool, tags.null, tags.atom], color: 'var(--syntax-keyword, #cf222e)' },
  ]);

  function languageExtension(name: string): Extension[] {
    const lower = name.toLowerCase();
    if (lower.endsWith('.yaml') || lower.endsWith('.yml')) return [yaml()];
    if (lower.endsWith('.json') || lower.endsWith('.jsonc')) return [json()];
    if (lower.endsWith('.sql')) return [sql()];
    if (lower.endsWith('.md') || lower.endsWith('.markdown')) return [markdown()];
    return [];
  }

  function renderRevertButton(): HTMLElement {
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'cm-diff-revert-btn';
    btn.textContent = '→';
    btn.title = 'Revertir este bloque al original';
    btn.setAttribute('aria-label', 'Revertir bloque');
    return btn;
  }

  function buildUnifiedMerge(original: string, collapse: boolean, canEdit: boolean): Extension {
    return unifiedMergeView({
      original,
      highlightChanges: true,
      gutter: true,
      collapseUnchanged: collapse ? { margin: 3, minSize: 4 } : undefined,
      mergeControls: canEdit
        ? (type, action) => {
            const btn = document.createElement('button');
            btn.type = 'button';
            btn.className = `cm-diff-merge-btn cm-diff-merge-${type}`;
            if (type === 'reject') {
              btn.textContent = 'Revertir';
              btn.title = 'Revertir este bloque al original';
            } else {
              btn.textContent = 'Mantener';
              btn.title = 'Aceptar este bloque en la vista';
            }
            btn.onclick = action;
            return btn;
          }
        : false,
    });
  }

  function primaryView(): EditorView | null {
    if (activeMergeView) {
      if (activeMergeView.a.hasFocus) return activeMergeView.a;
      return activeMergeView.b;
    }
    return activeUnifiedView;
  }

  function navView(): EditorView | null {
    if (activeMergeView) return activeMergeView.b;
    return activeUnifiedView;
  }

  function reportChunks(view: EditorView | null) {
    if (!onChunksChange) return;
    if (!view) {
      onChunksChange({ total: 0, current: 0 });
      return;
    }
    const info = getChunks(view.state);
    if (!info || info.chunks.length === 0) {
      onChunksChange({ total: 0, current: 0 });
      return;
    }
    const head = view.state.selection.main.head;
    const isA = info.side === 'a';
    const idx = info.chunks.findIndex((c) => {
      const from = isA ? c.fromA : c.fromB;
      const to = isA ? c.toA : c.toB;
      return head >= from && head <= to;
    });
    onChunksChange({ total: info.chunks.length, current: idx >= 0 ? idx + 1 : 0 });
  }

  export function nextChunk(): boolean {
    const view = navView();
    if (!view) return false;
    const moved = goToNextChunk(view);
    if (moved) view.focus();
    reportChunks(view);
    return moved;
  }

  export function prevChunk(): boolean {
    const view = navView();
    if (!view) return false;
    const moved = goToPreviousChunk(view);
    if (moved) view.focus();
    reportChunks(view);
    return moved;
  }

  export function openSearch(): boolean {
    const view = primaryView();
    if (!view) return false;
    view.focus();
    return openSearchPanel(view);
  }

  $effect(() => {
    if (!containerEl) return;

    const isDark = theme === 'dark';
    const initialEditable = untrack(() => editable);
    const initialVim = untrack(() => vimMode);

    const sharedExtensions: Extension[] = [
      vimCompartment.of(initialVim ? vim() : []),
      tooltips({ parent: document.body }),
      basicSetup,
      undoRedoExtension(),
      commentExtension(),
      ...languageExtension(fileName),
      autocompletion({ override: [documentWordsCompletion] }),
      syntaxHighlighting(diffHighlightStyle),
      EditorState.phrases.of({
        'Fold line': 'Plegar',
        'Unfold line': 'Desplegar',
        'folded code': 'código plegado',
        unfold: 'desplegar',
      }),
      search({ top: true }),
      keymap.of(searchKeymap as any),
      renameOccurrencesKeymap(),
      lineKeymap(),
      ...(wrap ? [EditorView.lineWrapping] : []),
      EditorView.theme(
        {
          '&': {
            fontSize: '12.5px',
            fontFamily: 'var(--font-mono, ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace)',
            backgroundColor: 'var(--bg-primary)',
            color: 'var(--text-primary)',
          },
          '.cm-scroller': {
            fontFamily: 'inherit',
            lineHeight: '1.5',
          },
          '.cm-gutters': {
            backgroundColor: 'var(--bg-secondary)',
            color: 'var(--text-muted)',
            borderRight: '1px solid var(--border-primary)',
          },
          '.cm-content': {
            caretColor: 'var(--text-primary)',
          },
          '.cm-cursor, .cm-dropCursor': {
            borderLeftColor: 'var(--text-primary)',
          },
          '.cm-activeLine': {
            backgroundColor: 'rgba(9, 105, 218, 0.05)',
          },
          '.cm-activeLineGutter': {
            backgroundColor: 'rgba(9, 105, 218, 0.1)',
            color: 'var(--accent)',
          },
          '.cm-vim-panel': {
            backgroundColor: 'var(--bg-secondary)',
            color: 'var(--text-primary)',
            fontFamily: 'var(--font-mono, monospace)',
            padding: '2px 10px',
          },
          '.cm-vim-panel input': {
            color: 'inherit',
            font: 'inherit',
            caretColor: 'var(--accent)',
            flex: '1',
            minWidth: '0',
          },
          '&.cm-focused': {
            outline: 'none',
          },
        },
        { dark: isDark },
      ),
    ];

    // Lado A: siempre solo lectura, manteniendo foco/selección/Vim/búsqueda activos
    const sideAExtensions: Extension[] = [
      ...sharedExtensions,
      EditorState.readOnly.of(true),
    ];

    const changeListener = EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        onContentChange?.(update.state.doc.toString());
        reportChunks(update.view);
      } else if (update.selectionSet) {
        reportChunks(update.view);
      }
    });

    const targetExtensions: Extension[] = [
      ...sharedExtensions,
      readOnlyCompartment.of(EditorState.readOnly.of(!initialEditable)),
      changeListener,
    ];

    const collapseConfig = collapseUnchanged ? { margin: 3, minSize: 4 } : undefined;

    let mergeView: MergeView | null = null;
    let unifiedView: EditorView | null = null;

    if (mode === 'split') {
      mergeView = new MergeView({
        parent: containerEl,
        a: {
          doc: oldContent,
          extensions: sideAExtensions,
        },
        b: {
          doc: newContent,
          extensions: targetExtensions,
        },
        highlightChanges: true,
        gutter: true,
        collapseUnchanged: collapseConfig,
        revertControls: initialEditable ? 'a-to-b' : undefined,
        renderRevertControl: initialEditable ? renderRevertButton : undefined,
      });
      activeMergeView = mergeView;
      activeUnifiedView = null;
      reportChunks(mergeView.b);
    } else {
      unifiedView = new EditorView({
        parent: containerEl,
        state: EditorState.create({
          doc: newContent,
          extensions: [
            ...targetExtensions,
            unifiedMergeCompartment.of(buildUnifiedMerge(oldContent, collapseUnchanged, initialEditable)),
          ],
        }),
      });
      activeMergeView = null;
      activeUnifiedView = unifiedView;
      reportChunks(unifiedView);
    }

    return () => {
      mergeView?.destroy();
      unifiedView?.destroy();
      if (activeMergeView === mergeView) activeMergeView = null;
      if (activeUnifiedView === unifiedView) activeUnifiedView = null;
    };
  });

  // Alternar Vim en caliente sin recrear las vistas
  $effect(() => {
    const ext = vimMode ? vim() : [];
    if (activeMergeView) {
      activeMergeView.a.dispatch({ effects: vimCompartment.reconfigure(ext) });
      activeMergeView.b.dispatch({ effects: vimCompartment.reconfigure(ext) });
    } else if (activeUnifiedView) {
      activeUnifiedView.dispatch({ effects: vimCompartment.reconfigure(ext) });
    }
  });

  // Alternar Lectura/Edición en caliente sin perder cursor ni cambios
  $effect(() => {
    const canEdit = editable;
    const roExt = EditorState.readOnly.of(!canEdit);
    if (activeMergeView) {
      activeMergeView.b.dispatch({ effects: readOnlyCompartment.reconfigure(roExt) });
      activeMergeView.reconfigure({
        revertControls: canEdit ? 'a-to-b' : undefined,
        renderRevertControl: canEdit ? renderRevertButton : undefined,
      });
    } else if (activeUnifiedView) {
      const orig = untrack(() => oldContent);
      const collapse = untrack(() => collapseUnchanged);
      activeUnifiedView.dispatch({
        effects: [
          readOnlyCompartment.reconfigure(roExt),
          unifiedMergeCompartment.reconfigure(buildUnifiedMerge(orig, collapse, canEdit)),
        ],
      });
    }
  });
</script>

<div class="cm-diff-host" class:dark={theme === 'dark'} bind:this={containerEl}></div>

<style>
  .cm-diff-host {
    width: 100%;
    height: 100%;
    overflow: auto;
    background: var(--bg-primary);
  }

  .cm-diff-host :global(.cm-mergeView) {
    height: 100%;
  }

  .cm-diff-host :global(.cm-mergeViewEditors) {
    min-height: 100%;
  }

  .cm-diff-host :global(.cm-mergeViewEditor) {
    flex: 1 1 50%;
    min-width: 0;
  }

  .cm-diff-host :global(.cm-editor) {
    height: 100%;
  }

  .cm-diff-host :global(.cm-collapsedLines) {
    background: var(--bg-secondary);
    color: var(--text-secondary);
    border-top: 1px solid var(--border-primary);
    border-bottom: 1px solid var(--border-primary);
    padding: 4px 12px;
    font-size: 12px;
    cursor: pointer;
  }

  .cm-diff-host :global(.cm-collapsedLines:hover) {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  /* Colores de diff — tema claro */
  .cm-diff-host :global(.cm-changedLine) {
    background-color: rgba(46, 160, 67, 0.12);
  }

  .cm-diff-host :global(.cm-merge-a .cm-changedLine),
  .cm-diff-host :global(.cm-deletedChunk) {
    background-color: rgba(207, 34, 46, 0.1);
  }

  .cm-diff-host :global(.cm-changedText) {
    background-color: rgba(46, 160, 67, 0.28);
    border-radius: 2px;
  }

  .cm-diff-host :global(.cm-merge-a .cm-changedText),
  .cm-diff-host :global(.cm-deletedChunk .cm-changedText) {
    background-color: rgba(207, 34, 46, 0.25);
    border-radius: 2px;
  }

  /* Colores de diff — tema oscuro */
  .cm-diff-host.dark :global(.cm-changedLine) {
    background-color: rgba(46, 160, 67, 0.14);
  }

  .cm-diff-host.dark :global(.cm-merge-a .cm-changedLine),
  .cm-diff-host.dark :global(.cm-deletedChunk) {
    background-color: rgba(248, 81, 73, 0.15);
  }

  .cm-diff-host.dark :global(.cm-changedText) {
    background-color: rgba(46, 160, 67, 0.34);
  }

  .cm-diff-host.dark :global(.cm-merge-a .cm-changedText),
  .cm-diff-host.dark :global(.cm-deletedChunk .cm-changedText) {
    background-color: rgba(248, 81, 73, 0.36);
  }

  /* Controles de revertir en vista dividida y unificada */
  .cm-diff-host :global(.cm-merge-revert) {
    width: 1.8em;
    background: var(--bg-secondary);
    border-left: 1px solid var(--border-primary);
    border-right: 1px solid var(--border-primary);
  }

  .cm-diff-host :global(.cm-diff-revert-btn) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 18px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    border-radius: 3px;
  }

  .cm-diff-host :global(.cm-diff-revert-btn:hover) {
    background: var(--accent);
    color: #fff;
  }

  .cm-diff-host :global(.cm-diff-merge-btn) {
    margin: 2px 4px 2px 0;
    padding: 1px 7px;
    border: 1px solid var(--border-primary);
    border-radius: 4px;
    background: var(--bg-secondary);
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
  }

  .cm-diff-host :global(.cm-diff-merge-btn:hover) {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  /* Panel de búsqueda (@codemirror/search) integrado con el tema */
  .cm-diff-host :global(.cm-panels) {
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-bottom: 1px solid var(--border-primary);
    z-index: 5;
  }

  .cm-diff-host :global(.cm-search) {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    font-size: 12px;
  }

  .cm-diff-host :global(.cm-search .cm-textfield) {
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border-primary);
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 12px;
    outline: none;
  }

  .cm-diff-host :global(.cm-search .cm-textfield:focus) {
    border-color: var(--accent);
  }

  .cm-diff-host :global(.cm-search .cm-button) {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border-primary);
    border-radius: 4px;
    padding: 2px 8px;
    font-size: 11.5px;
    cursor: pointer;
    background-image: none;
  }

  .cm-diff-host :global(.cm-search .cm-button:hover) {
    border-color: var(--accent);
  }

  .cm-diff-host :global(.cm-search label) {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--text-secondary);
    font-size: 11.5px;
  }

  .cm-diff-host :global(.cm-searchMatch) {
    background-color: rgba(255, 200, 0, 0.32);
    border-radius: 2px;
  }

  .cm-diff-host :global(.cm-searchMatch-selected) {
    background-color: rgba(255, 140, 0, 0.55);
  }
</style>
