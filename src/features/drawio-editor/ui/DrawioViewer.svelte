<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { appSettings } from '@entities/settings';
  import { vaultRepository } from '@shared/repositories';
  import {
    buildDrawioUrl,
    parseDrawioMessage,
    bytesToDataUrl,
    dataUrlToBytes,
    EMPTY_DRAWIO_XML,
    type DrawioFormat,
  } from '../lib/drawio-embed';

  interface Props {
    /** Ruta relativa en la bóveda. */
    filePath: string;
    /** `xml` para `.drawio` (texto), `xmlpng` / `xmlsvg` para un PNG o SVG con el diagrama incrustado. */
    format: DrawioFormat;
    /** Contenido de texto (solo `xml`). */
    content?: string;
    /** Guardado del XML (solo `xml`): la página lo escribe en disco. */
    onSaveXml?: (xml: string) => void;
    /** Tras reescribir la imagen (`xmlpng` / `xmlsvg`). */
    onSavedPng?: () => void;
    /** El usuario cerró el editor (imágenes: vuelve al visor de imagen). */
    onExit?: () => void;
  }

  let { filePath, format, content = '', onSaveXml, onSavedPng, onExit }: Props = $props();

  let iframe = $state<HTMLIFrameElement | null>(null);
  let status = $state<'loading' | 'ready' | 'saving' | 'error'>('loading');
  let errorMessage = $state('');

  // El tema y la URL se fijan al abrir: recargar el iframe perdería los cambios sin guardar.
  const isDark = typeof document !== 'undefined' && document.documentElement.getAttribute('data-theme') === 'dark';
  const isImage = untrack(() => format !== 'xml');
  const src = buildDrawioUrl(appSettings.settings.drawioUrl, isDark, appSettings.settings.locale ?? 'es', isImage);
  const origin = new URL(src).origin;

  function post(msg: Record<string, unknown>) {
    iframe?.contentWindow?.postMessage(JSON.stringify(msg), origin);
  }

  async function loadDiagram() {
    try {
      if (format === 'xmlpng') {
        const buffer = await vaultRepository.readBinary(filePath);
        post({ action: 'load', xmlpng: bytesToDataUrl(new Uint8Array(buffer)), autosave: 0, title: filePath.split('/').pop() });
      } else if (format === 'xmlsvg') {
        // draw.io lee el diagrama del atributo `content` del SVG
        const svg = new TextDecoder().decode(new Uint8Array(await vaultRepository.readBinary(filePath)));
        post({ action: 'load', xml: svg, autosave: 0, title: filePath.split('/').pop() });
      } else {
        post({ action: 'load', xml: content.trim() ? content : EMPTY_DRAWIO_XML, autosave: 0, title: filePath.split('/').pop() });
      }
    } catch (err) {
      status = 'error';
      errorMessage = `No se pudo leer el diagrama: ${err}`;
    }
  }

  async function writeImage(dataUrl: string) {
    try {
      await vaultRepository.writeBinary(filePath, dataUrlToBytes(dataUrl));
      post({ action: 'status', modified: false, message: '' });
      status = 'ready';
      onSavedPng?.();
    } catch (err) {
      status = 'error';
      errorMessage = `No se pudo guardar la imagen: ${err}`;
    }
  }

  function handleMessage(e: MessageEvent) {
    if (!iframe || e.source !== iframe.contentWindow) return;
    const msg = parseDrawioMessage(e.data);
    if (!msg) return;
    switch (msg.event) {
      case 'init':
        void loadDiagram();
        break;
      case 'load':
        status = 'ready';
        break;
      case 'save':
        if (isImage) {
          status = 'saving';
          post({ action: 'export', format, spinKey: 'saving' });
        } else if (typeof msg.xml === 'string') {
          onSaveXml?.(msg.xml);
          post({ action: 'status', modified: false, message: '' });
        }
        break;
      case 'export':
        if (isImage && typeof msg.data === 'string') void writeImage(msg.data);
        break;
      case 'exit':
        onExit?.();
        break;
    }
  }

  onMount(() => {
    window.addEventListener('message', handleMessage);
    return () => window.removeEventListener('message', handleMessage);
  });
</script>

<div class="drawio-viewer" data-drawio-path={filePath} data-drawio-status={status}>
  {#if status === 'loading'}
    <div class="drawio-overlay">Cargando draw.io…</div>
  {:else if status === 'error'}
    <div class="drawio-overlay drawio-error">{errorMessage}</div>
  {/if}
  <div class="drawio-source" title="El editor se descarga de esa web; el diagrama no se envía, viaja solo dentro de la app">
    draw.io cargado desde {new URL(src).host} (requiere conexión)
  </div>
  <iframe bind:this={iframe} {src} title={`draw.io: ${filePath}`} class="drawio-frame"></iframe>
</div>

<style>
  .drawio-viewer {
    position: relative;
    flex: 1;
    width: 100%;
    height: 100%;
    min-height: 0;
    display: flex;
  }
  .drawio-frame {
    flex: 1;
    width: 100%;
    height: 100%;
    border: 0;
  }
  .drawio-overlay {
    position: absolute;
    top: 8px;
    left: 50%;
    transform: translateX(-50%);
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 12px;
    background: var(--bg-secondary, #eee);
    color: var(--text-muted, #666);
    pointer-events: none;
    z-index: 1;
  }
  .drawio-source {
    position: absolute;
    right: 8px;
    bottom: 6px;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    background: var(--bg-secondary, #eee);
    color: var(--text-muted, #666);
    opacity: 0.85;
    z-index: 1;
  }
  .drawio-error {
    color: var(--color-error, #c0392b);
  }
</style>
