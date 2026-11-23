<script lang="ts">
  import { EditorState, type Extension } from '@codemirror/state';
  import { EditorView, lineNumbers } from '@codemirror/view';
  import { MergeView, unifiedMergeView } from '@codemirror/merge';
  import { syntaxHighlighting, defaultHighlightStyle } from '@codemirror/language';
  import { yaml } from '@codemirror/lang-yaml';
  import { json } from '@codemirror/lang-json';
  import { markdown } from '@codemirror/lang-markdown';

  interface Props {
    oldContent: string;
    newContent: string;
    fileName: string;
    mode: 'split' | 'unified';
    wrap?: boolean;
    collapseUnchanged?: boolean;
    theme?: 'light' | 'dark';
  }

  let {
    oldContent,
    newContent,
    fileName,
    mode,
    wrap = false,
    collapseUnchanged = true,
    theme = 'dark',
  }: Props = $props();

  let containerEl = $state<HTMLDivElement | null>(null);

  function languageExtension(name: string): Extension[] {
    const lower = name.toLowerCase();
    if (lower.endsWith('.yaml') || lower.endsWith('.yml')) return [yaml()];
    if (lower.endsWith('.json') || lower.endsWith('.jsonc')) return [json()];
    if (lower.endsWith('.md') || lower.endsWith('.markdown')) return [markdown()];
    return [];
  }

  $effect(() => {
    if (!containerEl) return;

    const isDark = theme === 'dark';
    const baseExtensions: Extension[] = [
      lineNumbers(),
      EditorState.readOnly.of(true),
      EditorView.editable.of(false),
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
      ...languageExtension(fileName),
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
          '.cm-merge-revert': {
            display: 'none',
          },
        },
        { dark: isDark },
      ),
    ];

    const collapseConfig = collapseUnchanged ? { margin: 3, minSize: 4 } : undefined;

    let mergeView: MergeView | null = null;
    let unifiedView: EditorView | null = null;

    if (mode === 'split') {
      mergeView = new MergeView({
        parent: containerEl,
        a: {
          doc: oldContent,
          extensions: baseExtensions,
        },
        b: {
          doc: newContent,
          extensions: baseExtensions,
        },
        highlightChanges: true,
        gutter: true,
        collapseUnchanged: collapseConfig,
      });
    } else {
      unifiedView = new EditorView({
        parent: containerEl,
        state: EditorState.create({
          doc: newContent,
          extensions: [
            ...baseExtensions,
            unifiedMergeView({
              original: oldContent,
              highlightChanges: true,
              gutter: true,
              mergeControls: false,
              collapseUnchanged: collapseConfig,
            }),
          ],
        }),
      });
    }

    return () => {
      mergeView?.destroy();
      unifiedView?.destroy();
    };
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

  .cm-diff-host.dark :global(.cm-changedLine) {
    background-color: rgba(46, 160, 67, 0.14);
  }

  .cm-diff-host.dark :global(.cm-merge-a .cm-changedLine),
  .cm-diff-host.dark :global(.cm-deletedChunk) {
    background-color: rgba(248, 81, 73, 0.15);
  }

  .cm-diff-host.dark :global(.cm-changedText) {
    background-color: rgba(46, 160, 67, 0.32);
  }

  .cm-diff-host.dark :global(.cm-merge-a .cm-changedText),
  .cm-diff-host.dark :global(.cm-deletedChunk .cm-changedText) {
    background-color: rgba(248, 81, 73, 0.35);
  }
</style>
