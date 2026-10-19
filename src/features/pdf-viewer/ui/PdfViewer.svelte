<script lang="ts">
  import { PDFViewer, type EmbedPdfContainer, type PluginRegistry, type PDFViewerConfig } from '@embedpdf/svelte-pdf-viewer';
  import pdfiumWasmUrl from '@embedpdf/pdfium/pdfium.wasm?url';
  import { vaultRepository } from '@shared/repositories';
  import { appSettings } from '@entities/settings';
  import { Loader2, AlertCircle } from 'lucide-svelte';

  interface Props {
    src: string;
    onDirtyChange?: (isDirty: boolean) => void;
    onSaved?: () => void;
    class?: string;
  }

  let {
    src,
    onDirtyChange,
    onSaved,
    class: className = ''
  }: Props = $props();

  let container = $state<EmbedPdfContainer | null>(null);
  let registry = $state<PluginRegistry | null>(null);
  let currentBuffer = $state<ArrayBuffer | null>(null);
  let currentBlobUrl = $state<string | null>(null);
  let currentRevoke = $state<(() => void) | null>(null);
  let annotationUnsubscribe: (() => void) | null = null;
  let isLoading = $state(true);
  let hasError = $state(false);
  let errorMessage = $state('');
  let isDirty = $state(false);
  let isSaving = $state(false);
  let autoSaveTimer: ReturnType<typeof setTimeout> | null = null;

  function arrayBufferToBase64(buffer: ArrayBuffer): string {
    let binary = '';
    const bytes = new Uint8Array(buffer);
    const len = bytes.byteLength;
    const chunkSize = 8192;
    for (let i = 0; i < len; i += chunkSize) {
      const chunk = bytes.subarray(i, Math.min(i + chunkSize, len));
      binary += String.fromCharCode.apply(null, chunk as unknown as number[]);
    }
    return btoa(binary);
  }

  function parseDataUrl(dataUrl: string): { buffer: ArrayBuffer; blobUrl: string; revoke: () => void } {
    const parts = dataUrl.split(',');
    const mimeMatch = parts[0]?.match(/:(.*?);/);
    const mime = mimeMatch ? mimeMatch[1] : 'application/pdf';
    const bstr = atob(parts[1] || '');
    let n = bstr.length;
    const u8arr = new Uint8Array(n);
    while (n--) {
      u8arr[n] = bstr.charCodeAt(n);
    }
    const blob = new Blob([u8arr], { type: mime });
    const url = URL.createObjectURL(blob);
    return {
      buffer: u8arr.buffer,
      blobUrl: url,
      revoke: () => URL.revokeObjectURL(url)
    };
  }

  $effect(() => {
    let isCurrent = true;
    const targetSrc = src;

    if (!targetSrc) {
      cleanupResources();
      isLoading = false;
      hasError = false;
      return;
    }

    isLoading = true;
    hasError = false;
    errorMessage = '';

    // Si ya es un blob o data URL externo
    if (/^blob:/i.test(targetSrc)) {
      cleanupResources();
      currentBlobUrl = targetSrc;
      fetch(targetSrc)
        .then((res) => res.arrayBuffer())
        .then((buf) => {
          if (!isCurrent) return;
          currentBuffer = buf;
          isLoading = false;
        })
        .catch(() => {
          if (!isCurrent) return;
          isLoading = false;
        });
      return;
    }

    if (/^data:application\/pdf/i.test(targetSrc)) {
      cleanupResources();
      const { buffer, blobUrl, revoke } = parseDataUrl(targetSrc);
      currentBuffer = buffer;
      currentBlobUrl = blobUrl;
      currentRevoke = revoke;
      isLoading = false;
      return;
    }

    if (vaultRepository.isConnected()) {
      vaultRepository
        .readAssetDataUrl(targetSrc)
        .then((dataUrl) => {
          if (!isCurrent) return;
          if (dataUrl) {
            cleanupResources();
            const { buffer, blobUrl, revoke } = parseDataUrl(dataUrl);
            currentBuffer = buffer;
            currentBlobUrl = blobUrl;
            currentRevoke = revoke;
            isLoading = false;
          } else {
            // Intentar resolviendo ruta absoluta como fallback
            vaultRepository
              .resolveAssetFilePath(targetSrc)
              .then((absPath) => {
                if (!isCurrent) return;
                if (absPath) {
                  return vaultRepository.readAssetDataUrl(absPath);
                }
                return null;
              })
              .then((retryDataUrl) => {
                if (!isCurrent) return;
                if (retryDataUrl) {
                  cleanupResources();
                  const { buffer, blobUrl, revoke } = parseDataUrl(retryDataUrl);
                  currentBuffer = buffer;
                  currentBlobUrl = blobUrl;
                  currentRevoke = revoke;
                  isLoading = false;
                } else {
                  isLoading = false;
                  hasError = true;
                  errorMessage = `No se pudo cargar el archivo PDF: ${targetSrc}`;
                }
              })
              .catch((err) => {
                if (!isCurrent) return;
                isLoading = false;
                hasError = true;
                errorMessage = String(err?.message || err);
              });
          }
        })
        .catch((err) => {
          if (!isCurrent) return;
          isLoading = false;
          hasError = true;
          errorMessage = String(err?.message || err);
        });
    } else {
      isLoading = false;
      hasError = true;
      errorMessage = 'No hay conexión con la bóveda de Synapse';
    }

    return () => {
      isCurrent = false;
    };
  });

  function cleanupResources() {
    if (currentRevoke) {
      currentRevoke();
      currentRevoke = null;
    }
    currentBlobUrl = null;
    currentBuffer = null;
    if (annotationUnsubscribe) {
      annotationUnsubscribe();
      annotationUnsubscribe = null;
    }
  }

  // Sincronización reactiva del tema claro/oscuro de Synapse
  $effect(() => {
    const activeTheme = appSettings.resolvedTheme;
    if (container) {
      try {
        container.setTheme(activeTheme === 'dark' ? 'dark' : 'light');
      } catch (e) {
        console.warn('[PdfViewer] Error aplicando tema al visor:', e);
      }
    }
  });

  // Limpieza al desmontar: forzar guardado inmediato si había cambios pendientes
  $effect(() => {
    return () => {
      if (autoSaveTimer) {
        clearTimeout(autoSaveTimer);
        autoSaveTimer = null;
      }
      if (isDirty && !isSaving && registry && src) {
        saveDocument().catch((err) => {
          console.error('[PdfViewer] Error al guardar cambios al desmontar:', err);
        });
      }
      cleanupResources();
    };
  });

  function markDirty() {
    if (!isDirty) {
      isDirty = true;
      onDirtyChange?.(true);
    }
    if (autoSaveTimer) {
      clearTimeout(autoSaveTimer);
    }
    autoSaveTimer = setTimeout(() => {
      saveDocument().catch((err) => {
        console.error('[PdfViewer] Falló el guardado automático de anotaciones:', err);
      });
    }, 600);
  }

  function setupAnnotationTracking(reg: PluginRegistry) {
    try {
      if (annotationUnsubscribe) {
        annotationUnsubscribe();
        annotationUnsubscribe = null;
      }

      // 1. Escuchar eventos de anotaciones
      const annotationPlugin = reg.getPlugin('annotation');
      const annotationCapability: any = annotationPlugin?.provides?.();
      const handleEvent = (event: any) => {
        if (!event) return;
        const eventType = event.type;
        if (eventType === 'create' || eventType === 'update' || eventType === 'delete') {
          markDirty();
        }
      };

      let unsubEvent: (() => void) | null = null;
      if (typeof annotationCapability?.onAnnotationEvent === 'function') {
        unsubEvent = annotationCapability.onAnnotationEvent(handleEvent);
      } else if (typeof annotationCapability?.onAnnotationEvent?.on === 'function') {
        unsubEvent = annotationCapability.onAnnotationEvent.on(handleEvent);
      }

      // 2. Escuchar cambios de historial (undo/redo o commit de anotaciones)
      let unsubHistory: (() => void) | null = null;
      try {
        const historyPlugin = reg.getPlugin('history');
        const historyCapability: any = historyPlugin?.provides?.();
        if (typeof historyCapability?.onHistoryChange === 'function') {
          unsubHistory = historyCapability.onHistoryChange(() => {
            markDirty();
          });
        }
      } catch (he) {
        // Ignorar si el plugin de historial no está montado
      }

      annotationUnsubscribe = () => {
        if (unsubEvent) unsubEvent();
        if (unsubHistory) unsubHistory();
      };
    } catch (e) {
      console.warn('[PdfViewer] No se pudo inicializar escucha de anotaciones:', e);
    }
  }

  /**
   * Guarda las anotaciones y el estado actual del PDF en disco en la bóveda
   */
  export async function saveDocument(): Promise<void> {
    if (autoSaveTimer) {
      clearTimeout(autoSaveTimer);
      autoSaveTimer = null;
    }

    if (!registry || isSaving || !src) return;

    isSaving = true;
    try {
      // 1. Commit de anotaciones pendientes en memoria en el motor PDFium
      const annotationPlugin = registry.getPlugin('annotation');
      const annotationCapability: any = annotationPlugin?.provides?.();
      if (annotationCapability?.commit) {
        try {
          await annotationCapability.commit().toPromise();
        } catch (e) {
          console.warn('[PdfViewer] Aviso durante commit de anotaciones:', e);
        }
      }

      // 2. Exportar copia completa con anotaciones incrustadas
      const exportPlugin = registry.getPlugin('export');
      const exportCapability: any = exportPlugin?.provides?.();
      if (!exportCapability?.saveAsCopy) {
        console.warn('[PdfViewer] La capacidad export.saveAsCopy no está disponible');
        return;
      }

      const buffer: ArrayBuffer = await exportCapability.saveAsCopy().toPromise();
      const base64Data = arrayBufferToBase64(buffer);

      // 3. Persistir en la bóveda a través del backend Rust
      await vaultRepository.saveBinaryAsset(src, base64Data);

      isDirty = false;
      onDirtyChange?.(false);
      onSaved?.();
    } catch (err) {
      console.error('[PdfViewer] Error guardando PDF con anotaciones:', err);
      throw err;
    } finally {
      isSaving = false;
    }
  }

  const viewerConfig = $derived.by<PDFViewerConfig | undefined>(() => {
    if (!currentBlobUrl && !currentBuffer) return undefined;
    const fileName = src ? src.split('/').pop() || 'document.pdf' : 'document.pdf';
    return {
      src: currentBuffer ? undefined : (currentBlobUrl ?? undefined),
      documentManager: currentBuffer
        ? {
            initialDocuments: [
              {
                documentId: 'active-doc',
                name: fileName,
                buffer: currentBuffer,
                autoActivate: true,
              },
            ],
          }
        : undefined,
      wasmUrl: pdfiumWasmUrl,
      worker: false, // Desactivar web workers para evitar bloqueos por blob module workers en WebKitGTK / Tauri Linux
      fontFallback: null, // Desactivar llamadas externas CDN jsdelivr de fuentes
      fonts: {
        ui: null, // Evitar peticiones a Google Fonts en entornos de escritorio offline
        signature: null,
      },
      stamp: {
        manifests: [], // Evitar petición al CDN jsdelivr de sellos predeterminados
        defaultLibrary: false,
      },
      theme: {
        preference: appSettings.resolvedTheme === 'dark' ? 'dark' : 'light',
      },
      log: true,
    };
  });
</script>

<div class="synapse-pdf-viewer-root {className}">
  {#if isLoading}
    <div class="synapse-pdf-loading">
      <Loader2 class="synapse-spin-icon" size={32} />
      <span>Cargando documento PDF...</span>
    </div>
  {:else if hasError}
    <div class="synapse-pdf-error">
      <AlertCircle size={32} />
      <span class="synapse-pdf-error-title">Error al abrir el PDF</span>
      <span class="synapse-pdf-error-desc">{errorMessage}</span>
    </div>
  {:else if currentBlobUrl || currentBuffer}
    {#key (currentBlobUrl || src)}
      <div class="synapse-pdf-container">
        <PDFViewer
          config={viewerConfig}
          class="synapse-embedpdf-element"
          oninit={(c) => {
            container = c;
            try {
              c.setTheme(appSettings.resolvedTheme === 'dark' ? 'dark' : 'light');
            } catch (e) {
              // Ignorar error transitorio durante init
            }
          }}
          onready={(reg) => {
            registry = reg;
            setupAnnotationTracking(reg);
          }}
        />
      </div>
    {/key}
  {/if}
</div>

<style>
  .synapse-pdf-viewer-root {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
    background-color: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
  }

  .synapse-pdf-loading,
  .synapse-pdf-error {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 24px;
    color: var(--text-secondary, #656d76);
  }

  .synapse-pdf-error {
    color: #cf222e;
  }

  .synapse-pdf-error-title {
    font-weight: 600;
    font-size: 16px;
  }

  .synapse-pdf-error-desc {
    font-size: 13px;
    color: var(--text-muted, #8c959f);
    max-width: 480px;
    text-align: center;
    word-break: break-word;
  }

  .synapse-pdf-container {
    width: 100%;
    height: 100%;
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  :global(.synapse-embedpdf-element),
  :global(.synapse-embedpdf-element > embedpdf-container),
  :global(embedpdf-container) {
    width: 100% !important;
    height: 100% !important;
    flex: 1 !important;
    display: flex !important;
  }

  :global(.synapse-spin-icon) {
    animation: synapse-spin 1s linear infinite;
  }

  @keyframes synapse-spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>
