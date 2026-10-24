<script lang="ts">
  import { onMount, onDestroy, untrack } from 'svelte';
  import React from 'react';
  import { createRoot, type Root } from 'react-dom/client';
  import { Excalidraw } from '@excalidraw/excalidraw';
  import '@excalidraw/excalidraw/index.css';
  import {
    parseExcalidrawContent,
    serializeExcalidrawMarkdown,
    isExcalidrawMarkdown,
  } from '@shared/obsidian-api';
  import { vaultRepository } from '@shared/repositories';
  import { Sparkles } from 'lucide-svelte';

  interface Props {
    content: string;
    filePath?: string;
    readOnly?: boolean;
    onChange?: (content: string) => void;
    onSave?: (content?: string) => void;
    onSwitchToObsidian?: () => void;
  }

  let { content = '', filePath = '', readOnly = false, onChange, onSave, onSwitchToObsidian }: Props = $props();

  let containerRef = $state<HTMLDivElement | null>(null);
  let root: Root | null = null;
  let excalidrawAPI = $state<any>(null);

  // Mapeo en memoria de archivos/imágenes resueltas (fileId -> { id, dataURL, mimeType })
  let resolvedFilesMap: Record<string, any> = {};
  const pendingOrLoadedFileIds = new Set<string>();
  let suppressOnChange = false;
  let onChangeDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  // Función para resolver y cargar imágenes vinculadas en el diagrama
  async function resolveEmbeddedImages(raw: string, api?: any) {
    if (!raw) return;
    try {
      const parsedDoc = parseExcalidrawContent(raw);
      const embedded = parsedDoc.embeddedFiles || {};
      const fileIdsToResolve = Object.entries(embedded);

      if (fileIdsToResolve.length === 0) return;

      const newFiles: Record<string, any> = {};
      let hasNewFiles = false;

      // Cargar todas las imágenes incrustadas en paralelo
      const loadPromises = fileIdsToResolve.map(async ([fileId, targetPath]) => {
        if (pendingOrLoadedFileIds.has(fileId) || typeof targetPath !== 'string') return;
        pendingOrLoadedFileIds.add(fileId);

        try {
          // Resolver primero la ruta nativa para streaming directo con el protocolo de assets (zero-copy)
          let assetUrl: string | null = null;
          const absPath = await vaultRepository.resolveAssetFilePath(targetPath, filePath);
          if (absPath) {
            assetUrl = vaultRepository.resolveAssetUrl(absPath);
          } else {
            assetUrl = await vaultRepository.readAssetDataUrl(targetPath, filePath);
          }

          if (assetUrl) {
            let mimeType = 'image/png';
            const cleanTarget = targetPath.toLowerCase().split('?')[0].split('#')[0];
            if (cleanTarget.endsWith('.svg')) mimeType = 'image/svg+xml';
            else if (cleanTarget.endsWith('.jpg') || cleanTarget.endsWith('.jpeg')) mimeType = 'image/jpeg';
            else if (cleanTarget.endsWith('.webp')) mimeType = 'image/webp';
            else if (cleanTarget.endsWith('.gif')) mimeType = 'image/gif';
            else if (cleanTarget.endsWith('.bmp')) mimeType = 'image/bmp';
            else if (assetUrl.startsWith('data:')) {
              const mimeMatch = assetUrl.match(/^data:([^;]+);/);
              if (mimeMatch) mimeType = mimeMatch[1];
            }

            const fileRecord = {
              id: fileId,
              dataURL: assetUrl,
              mimeType,
              created: Date.now(),
            };
            resolvedFilesMap[fileId] = fileRecord;
            newFiles[fileId] = fileRecord;
            hasNewFiles = true;
          }
        } catch (err) {
          console.warn('[ExcalidrawViewer] No se pudo resolver imagen incrustada:', { fileId, targetPath, err });
        }
      });

      await Promise.all(loadPromises);

      if (hasNewFiles) {
        const targetApi = api || excalidrawAPI;
        if (targetApi?.addFiles) {
          // Desactivar temporalmente el listener onChange durante addFiles para evitar bucles de renderizado
          suppressOnChange = true;
          try {
            targetApi.addFiles(Object.values(newFiles));
          } finally {
            setTimeout(() => {
              suppressOnChange = false;
            }, 300);
          }
        }
      }
    } catch (e) {
      console.warn('[ExcalidrawViewer] Error procesando embeddedFiles:', e);
    }
  }

  // Guardar la última cadena de contenido conocida en Excalidraw
  let lastKnownScene = untrack(() => content);
  let lastReadOnly = untrack(() => readOnly);

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
      const parsedDoc = parseExcalidrawContent(raw);
      const scene = parsedDoc.scene;
      const appState = scene.appState ? { ...scene.appState } : {};
      appState.viewModeEnabled = isReadOnly;
      // Si estamos en modo edición (no readOnly), asegurarnos de que la herramienta activa no quede atascada en mano
      if (!isReadOnly && (appState.activeTool?.type === 'hand' || !appState.activeTool)) {
        appState.activeTool = { type: 'selection' };
      }
      return {
        elements: scene.elements || [],
        appState,
        files: {
          ...(scene.files || {}),
          ...resolvedFilesMap,
        },
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
    const isMd = isExcalidrawMarkdown(content) || content.startsWith('---');
    if (isMd) {
      // Para documentos Obsidian Markdown, no persistir archivos incrustados en files JSON (residen en ## Embedded Files)
      const cleanFiles: Record<string, any> = {};
      if (files) {
        for (const [id, record] of Object.entries(files)) {
          if (!resolvedFilesMap[id]) {
            cleanFiles[id] = record;
          }
        }
      }
      return serializeExcalidrawMarkdown(
        {
          elements: elements ? [...elements] : [],
          appState,
          files: cleanFiles,
        },
        content
      );
    }

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
    lastKnownScene = serialized;
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
        // Resolver y cargar imágenes vinculadas del vault (## Embedded Files)
        resolveEmbeddedImages(data, api);
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
        if (isReadOnly || suppressOnChange) return;

        if (onChangeDebounceTimer) clearTimeout(onChangeDebounceTimer);
        onChangeDebounceTimer = setTimeout(() => {
          if (isReadOnly || suppressOnChange) return;
          const serialized = serializeSceneData(elements, appState, files);
          if (serialized !== lastKnownScene) {
            lastKnownScene = serialized;
            if (onChange) onChange(serialized);
          }
        }, 300);
      }
    });

    root.render(reactElement);
  }

  onMount(() => {
    lastKnownScene = content;
    lastReadOnly = readOnly;
    renderReactApp(content, readOnly);
  });

  onDestroy(() => {
    if (onChangeDebounceTimer) {
      clearTimeout(onChangeDebounceTimer);
      onChangeDebounceTimer = null;
    }
    if (!readOnly && excalidrawAPI) {
      try {
        const serialized = getCurrentSerializedScene();
        if (serialized !== lastKnownScene) {
          lastKnownScene = serialized;
          if (onChange) onChange(serialized);
          if (onSave) onSave(serialized);
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

    const readOnlyChanged = r !== lastReadOnly;
    const contentChanged = c !== lastKnownScene;

    if (!root || !containerRef) return;

    if (readOnlyChanged) {
      lastReadOnly = r;
      if (excalidrawAPI) {
        excalidrawAPI.updateScene({
          appState: {
            viewModeEnabled: r,
            ...(r ? { activeTool: { type: 'hand' } } : {}),
          },
        });
        if (!r && excalidrawAPI.setActiveTool) {
          excalidrawAPI.setActiveTool({ type: 'selection' });
        }
      } else {
        renderReactApp(c, r);
      }
    }

    // Solo si el contenido cambió externamente (diferente de lo que Excalidraw emitió)
    if (contentChanged) {
      lastKnownScene = c;
      if (excalidrawAPI) {
        const parsed = parseInitialData(c, r);
        const currentAppState = excalidrawAPI.getAppState?.() || {};
        excalidrawAPI.updateScene({
          elements: parsed.elements,
          appState: {
            ...parsed.appState,
            // Conservar la herramienta activa actual si no estamos en solo lectura (evita volver a 'hand')
            ...(!r && currentAppState.activeTool && currentAppState.activeTool.type !== 'hand'
              ? { activeTool: currentAppState.activeTool }
              : {}),
            viewModeEnabled: r,
          },
          commitToHistory: false,
        });
        if (!r && excalidrawAPI.setActiveTool && (!currentAppState.activeTool || currentAppState.activeTool.type === 'hand')) {
          excalidrawAPI.setActiveTool({ type: 'selection' });
        }
        if (parsed.files && Object.keys(parsed.files).length > 0) {
          excalidrawAPI.addFiles?.(Object.values(parsed.files));
        }
        resolveEmbeddedImages(c, excalidrawAPI);
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
>
  {#if onSwitchToObsidian}
    <button
      type="button"
      class="switch-to-obsidian-btn"
      onclick={onSwitchToObsidian}
      title="Abrir con el visor de Excalidraw para Obsidian"
    >
      <Sparkles size={13} />
      <span>Abrir con Obsidian</span>
    </button>
  {/if}
</div>

<style>
  .excalidraw-container {
    width: 100%;
    height: 100%;
    min-height: 500px;
    position: relative;
    overflow: hidden;
  }

  .switch-to-obsidian-btn {
    position: absolute;
    bottom: 14px;
    right: 14px;
    z-index: 100;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 6px;
    background-color: color-mix(in srgb, var(--bg-secondary, #f6f8fa) 92%, transparent);
    backdrop-filter: blur(8px);
    border: 1px solid var(--border-primary, #d0d7de);
    color: var(--text-primary, #1f2328);
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
    transition: all 0.15s ease;
  }

  .switch-to-obsidian-btn:hover {
    background-color: var(--bg-tertiary, #eaeef2);
    border-color: var(--accent, #6366f1);
    color: var(--accent, #6366f1);
  }

  .excalidraw-container :global(.excalidraw) {
    height: 100% !important;
    width: 100% !important;
  }
</style>
