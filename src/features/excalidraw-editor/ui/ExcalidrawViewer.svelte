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
    onSave?: (content?: string) => void;
  }

  let { content = '', readOnly = false, onChange, onSave }: Props = $props();

  let containerRef = $state<HTMLDivElement | null>(null);
  let root: Root | null = null;
  let excalidrawAPI = $state<any>(null);

  // Guardar la última cadena de contenido emitida o recibida
  let lastContent = untrack(() => content);
  let lastReadOnly = untrack(() => readOnly);
  let isInternalUpdate = false;

  function parseInitialData(raw: string, isReadOnly: boolean) {
    if (!raw || !raw.trim()) {
      return {
        elements: [],
        appState: {
          viewModeEnabled: isReadOnly,
          activeTool: isReadOnly ? { type: 'hand' } : { type: 'selection' },
        },
        files: {},
      };
    }
    try {
      const parsed = JSON.parse(raw);
      const appState = parsed.appState ? { ...parsed.appState } : {};
      appState.viewModeEnabled = isReadOnly;
      // Si estamos en modo edición (no readOnly), asegurarnos de que la herramienta activa no quede atascada en mano
      if (!isReadOnly && (appState.activeTool?.type === 'hand' || !appState.activeTool)) {
        appState.activeTool = { type: 'selection' };
      }
      return {
        elements: parsed.elements || [],
        appState,
        files: parsed.files || {},
      };
    } catch (e) {
      console.warn('Error al parsear contenido Excalidraw:', e);
      return {
        elements: [],
        appState: {
          viewModeEnabled: isReadOnly,
          activeTool: isReadOnly ? { type: 'hand' } : { type: 'selection' },
        },
        files: {},
      };
    }
  }

  function serializeSceneData(elements: readonly any[], appState: any, files: any): string {
    return JSON.stringify(
      {
        type: 'excalidraw',
        version: 2,
        source: 'synapse',
        elements: elements || [],
        appState: {
          theme: appState?.theme,
          viewBackgroundColor: appState?.viewBackgroundColor,
          gridSize: appState?.gridSize,
        },
        files: files || {},
      },
      null,
      2
    );
  }

  export function getCurrentSerializedScene(): string {
    let elements: readonly any[] = [];
    let appState: any = {};
    let files: any = {};

    if (excalidrawAPI) {
      elements = excalidrawAPI.getSceneElements?.() || [];
      appState = excalidrawAPI.getAppState?.() || {};
      files = excalidrawAPI.getFiles?.() || {};
    } else {
      const parsed = parseInitialData(content, readOnly);
      elements = parsed.elements;
      appState = parsed.appState;
      files = parsed.files;
    }

    const serialized = serializeSceneData(elements, appState, files);
    lastContent = serialized;
    isInternalUpdate = true;
    return serialized;
  }

  export function saveCurrentScene(): string {
    const serialized = getCurrentSerializedScene();
    if (onChange) {
      onChange(serialized);
    }
    if (onSave) {
      onSave(serialized);
    }
    return serialized;
  }

  function triggerSave() {
    if (readOnly) return;
    saveCurrentScene();
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
        if (initialData.files && Object.keys(initialData.files).length > 0) {
          api.addFiles?.(Object.values(initialData.files));
        }
        if (!isReadOnly && api?.setActiveTool) {
          api.setActiveTool({ type: 'selection' });
        }
      },
      viewModeEnabled: isReadOnly,
      handleKeyboardGlobally: false,
      UIOptions: {
        canvasActions: {
          loadScene: false,
          saveToActiveFile: false,
          saveAsImage: false,
          export: {
            saveFileToDisk: false,
          },
        },
      },
      onChange: (elements: readonly any[], appState: any, files: any) => {
        if (isReadOnly) return;

        const serialized = serializeSceneData(elements, appState, files);

        if (serialized !== lastContent) {
          lastContent = serialized;
          isInternalUpdate = true;
          if (onChange) onChange(serialized);
        }
      }
    });

    root.render(reactElement);
  }

  onMount(() => {
    lastContent = content;
    lastReadOnly = readOnly;
    renderReactApp(content, readOnly);
  });

  onDestroy(() => {
    if (!readOnly && excalidrawAPI) {
      try {
        const serialized = getCurrentSerializedScene();
        if (serialized !== lastContent) {
          lastContent = serialized;
          if (onChange) onChange(serialized);
        }
      } catch (err) {
        console.warn('Error al vaciar Excalidraw al destruir:', err);
      }
    }
    if (root) {
      root.unmount();
      root = null;
    }
    excalidrawAPI = null;
  });

  $effect(() => {
    const c = content;
    const r = readOnly;

    // Si la actualización vino del propio onChange de este visor, no resetear la escena
    if (isInternalUpdate) {
      isInternalUpdate = false;
      lastContent = c;
      lastReadOnly = r;
      return;
    }

    const contentChanged = c !== lastContent;
    const readOnlyChanged = r !== lastReadOnly;

    if (root && containerRef && (contentChanged || readOnlyChanged)) {
      lastContent = c;
      lastReadOnly = r;

      if (excalidrawAPI) {
        const parsed = parseInitialData(c, r);
        excalidrawAPI.updateScene({
          elements: parsed.elements,
          appState: {
            ...parsed.appState,
            viewModeEnabled: r,
            ...(r ? {} : { activeTool: { type: 'selection' } }),
          },
        });
        if (parsed.files && Object.keys(parsed.files).length > 0) {
          excalidrawAPI.addFiles?.(Object.values(parsed.files));
        }
      } else {
        renderReactApp(c, r);
      }
    }
  });

  function handleContainerKeyDownCapture(e: KeyboardEvent) {
    const isCtrlOrMeta = e.ctrlKey || e.metaKey;
    if (isCtrlOrMeta && !e.altKey) {
      const key = e.key.toLowerCase();
      if (key === 's') {
        e.preventDefault();
        e.stopPropagation();
        e.stopImmediatePropagation();
        triggerSave();
      } else if (key === 'o') {
        e.preventDefault();
        e.stopPropagation();
        e.stopImmediatePropagation();
      }
    }
  }
</script>

<div
  class="excalidraw-container"
  bind:this={containerRef}
  onkeydowncapture={handleContainerKeyDownCapture}
></div>

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
