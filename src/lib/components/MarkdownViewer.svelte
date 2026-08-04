<script lang="ts">
  import { Editor, rootCtx, defaultValueCtx, editorViewCtx } from '@milkdown/core';
  import { nord } from '@milkdown/theme-nord';
  import { commonmark } from '@milkdown/preset-commonmark';
  import { gfm } from '@milkdown/preset-gfm';
  import { diagram } from '@milkdown/plugin-diagram';
  import { listener, listenerCtx } from '@milkdown/plugin-listener';
  import { replaceAll } from '@milkdown/utils';
  import { onMount, onDestroy, untrack } from 'svelte';

  import '@milkdown/theme-nord/style.css';

  interface Props {
    content: string;
    isMarkdown?: boolean;
    readOnly?: boolean;
    onChange?: (markdown: string) => void;
  }

  let {
    content = '',
    isMarkdown = true,
    readOnly = false,
    onChange
  }: Props = $props();

  let containerRef = $state<HTMLDivElement | null>(null);
  let editorInstance = $state<Editor | null>(null);
  let internalMarkdown = $state('');
  let isInitializing = false;

  async function initMilkdown(initialText: string) {
    if (!containerRef || editorInstance || isInitializing) return;
    isInitializing = true;
    internalMarkdown = initialText;

    try {
      const editor = await Editor.make()
        .config((ctx) => {
          ctx.set(rootCtx, containerRef!);
          ctx.set(defaultValueCtx, initialText || '');
          ctx.get(listenerCtx).markdownUpdated((_, markdown) => {
            if (markdown !== internalMarkdown) {
              internalMarkdown = markdown;
              if (onChange) onChange(markdown);
            }
          });
        })
        .config(nord)
        .use(listener)
        .use(commonmark)
        .use(gfm)
        .use(diagram)
        .create();

      editorInstance = editor;
      isInitializing = false;

      if (readOnly) {
        editor.action((ctx) => {
          const view = ctx.get(editorViewCtx);
          if (view) {
            view.setProps({ editable: () => false });
          }
        });
      }

      // Sincronizar si el contenido cambio durante la inicializacion
      if (content !== initialText) {
        internalMarkdown = content;
        editor.action(replaceAll(content));
      }
    } catch (e) {
      isInitializing = false;
      console.error('Error al inicializar Milkdown:', e);
    }
  }

  onMount(() => {
    if (isMarkdown && containerRef) {
      initMilkdown(content);
    }
  });

  onDestroy(() => {
    if (editorInstance) {
      editorInstance.destroy();
      editorInstance = null;
    }
  });

  // Efecto reactivo para inicializar o actualizar el editor de Milkdown
  $effect(() => {
    const nextContent = content;
    const currentReadOnly = readOnly;
    const activeMarkdown = isMarkdown;

    // Inicializar si el nodo DOM ya esta montado y el editor aun no se ha creado
    if (containerRef && !editorInstance && !isInitializing && activeMarkdown) {
      initMilkdown(nextContent);
      return;
    }

    // Actualizar contenido del editor existente si cambio la nota
    if (editorInstance && activeMarkdown && nextContent !== untrack(() => internalMarkdown)) {
      internalMarkdown = nextContent;
      editorInstance.action(replaceAll(nextContent));
    }

    // Alternar modo editable / solo lectura
    if (editorInstance) {
      editorInstance.action((ctx) => {
        const view = ctx.get(editorViewCtx);
        if (view) {
          view.setProps({ editable: () => !currentReadOnly });
        }
      });
    }
  });
</script>

<div class="milkdown-wrapper">
  <div class="milkdown-container" class:hidden={!isMarkdown} bind:this={containerRef}></div>
  {#if !isMarkdown}
    <pre class="plain-text">{content}</pre>
  {/if}
</div>

<style>
  .milkdown-wrapper {
    width: 100%;
    min-height: 400px;
    background-color: transparent;
  }

  .milkdown-container {
    padding: 16px;
    min-height: 400px;
  }

  .milkdown-container.hidden {
    display: none;
  }

  :global(.milkdown) {
    position: relative;
    width: 100%;
    min-height: 400px;
    background: transparent !important;
    color: var(--text-primary) !important;
    box-shadow: none !important;
  }

  :global(.milkdown .editor),
  :global(.milkdown .ProseMirror),
  :global(.ProseMirror) {
    min-height: 400px !important;
    color: var(--text-primary) !important;
    outline: none !important;
    font-size: 15px !important;
    line-height: 1.6 !important;
  }

  :global(.ProseMirror p) {
    color: var(--text-primary) !important;
    margin-bottom: 12px !important;
  }

  :global(.ProseMirror h1, .ProseMirror h2, .ProseMirror h3) {
    color: var(--text-primary) !important;
    font-weight: 600 !important;
  }

  .plain-text {
    font-family: var(--code-font, monospace);
    white-space: pre-wrap;
    color: var(--text-primary);
    padding: 16px;
  }
</style>
