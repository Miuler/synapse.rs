<script lang="ts">
  import { onMount, onDestroy, untrack } from 'svelte';
  import React from 'react';
  import { createRoot, type Root } from 'react-dom/client';
  import { Excalidraw } from '@excalidraw/excalidraw';
  import '@excalidraw/excalidraw/index.css';

  interface Props {
    content: string;
    readOnly?: boolean;
    onChange?: (content: string) => void;
  }

  let { content = '', readOnly = false, onChange }: Props = $props();

  let containerRef = $state<HTMLDivElement | null>(null);
  let root: Root | null = null;
  let excalidrawAPI = $state<any>(null);
  let isMounted = false;

  let lastContent = $state(untrack(() => content));
  let lastReadOnly = $state(untrack(() => readOnly));

  function parseInitialData(raw: string, isReadOnly: boolean) {
    if (!raw || !raw.trim()) {
      return {
        elements: [],
        appState: { viewModeEnabled: isReadOnly },
        files: {},
      };
    }
    try {
      const parsed = JSON.parse(raw);
      return {
        elements: parsed.elements || [],
        appState: parsed.appState
          ? { ...parsed.appState, viewModeEnabled: isReadOnly }
          : { viewModeEnabled: isReadOnly },
        files: parsed.files || {},
      };
    } catch (e) {
      console.warn('Error al parsear contenido Excalidraw:', e);
      return {
        elements: [],
        appState: { viewModeEnabled: isReadOnly },
        files: {},
      };
    }
  }

  function renderReactApp(data: string, isReadOnly: boolean) {
    if (!containerRef) return;
    if (!root) {
      root = createRoot(containerRef);
    }

    const initialData = parseInitialData(data, isReadOnly);

    const reactElement = React.createElement(Excalidraw, {
      initialData,
      excalidrawAPI: (api: any) => {
        excalidrawAPI = api;
      },
      viewModeEnabled: isReadOnly,
      onChange: (elements: readonly any[], appState: any, files: any) => {
        if (isReadOnly) return;

        // Evitar emitir guarda en el primer render de montaje si no ha habido interacción
        if (!isMounted) {
          isMounted = true;
          return;
        }

        const serialized = JSON.stringify(
          {
            type: 'excalidraw',
            version: 2,
            source: 'synapse',
            elements,
            appState: {
              theme: appState.theme,
              viewBackgroundColor: appState.viewBackgroundColor,
              gridSize: appState.gridSize,
            },
            files,
          },
          null,
          2
        );

        if (serialized !== lastContent) {
          lastContent = serialized;
          if (onChange) onChange(serialized);
        }
      }
    });

    root.render(reactElement);
  }

  onMount(() => {
    renderReactApp(content, readOnly);
  });

  onDestroy(() => {
    if (root) {
      root.unmount();
      root = null;
    }
    excalidrawAPI = null;
  });

  $effect(() => {
    const c = content;
    const r = readOnly;
    if (root && containerRef && (c !== untrack(() => lastContent) || r !== untrack(() => lastReadOnly))) {
      lastContent = c;
      lastReadOnly = r;
      if (excalidrawAPI) {
        const parsed = parseInitialData(c, r);
        excalidrawAPI.updateScene(parsed);
      } else {
        renderReactApp(c, r);
      }
    }
  });
</script>

<div class="excalidraw-container" bind:this={containerRef}></div>

<style>
  .excalidraw-container {
    width: 100%;
    height: 100%;
    min-height: 500px;
    position: relative;
    overflow: hidden;
  }

  .excalidraw-container :global(.excalidraw) {
    height: 100% !important;
    width: 100% !important;
  }
</style>
