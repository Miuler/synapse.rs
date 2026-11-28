<script lang="ts">
  // Visor de documentos Word (.docx, .docm, .dotx) en solo lectura con docx-preview: páginas,
  // fuentes, colores, tablas, imágenes, encabezados y pies. La librería se carga al abrir el
  // primer documento. Para editar: «Abrir en la aplicación».
  import { vaultRepository } from '@shared/repositories';
  import { systemApi } from '@shared/api';
  import { renderDocx } from '@shared/lib/docx-render';
  import { ExternalLink, TriangleAlert } from 'lucide-svelte';
  import { t } from '@shared/lib/i18n';
  import { DomFindBar } from '@shared/ui/find-bar';
  import { appSettings } from '@entities/settings';
  import { ZoomControl } from '@shared/ui/zoom-control';
  import { stepViewerZoom } from '@shared/lib/viewer-zoom';

  interface Props {
    /** Ruta relativa a la bóveda */
    path: string;
    /** Cambia cuando el archivo cambió en disco: se vuelve a leer */
    reloadKey?: number;
  }
  let { path, reloadKey = 0 }: Props = $props();

  let body = $state<HTMLDivElement | null>(null);
  let styles = $state<HTMLDivElement | null>(null);
  let scroller = $state<HTMLDivElement | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let generation = 0;

  async function render() {
    if (!body || !styles) return;
    const gen = ++generation;
    loading = true;
    error = null;
    try {
      const data = await vaultRepository.readBinary(path);
      if (gen !== generation || !body || !styles) return;
      if (!data.byteLength) throw new Error(t('docxViewer.readError'));
      await renderDocx(data, body, styles);
    } catch (e) {
      if (gen === generation) error = e instanceof Error ? e.message : String(e);
    } finally {
      if (gen === generation) loading = false;
    }
  }

  $effect(() => {
    void path;
    void reloadKey;
    if (body && styles) render();
  });

  // Zoom de toda la página (CSS `zoom`: la maquetación crece entera, como en un PDF)
  const zoom = $derived(appSettings.docxZoom);

  function setZoom(z: number) {
    if (!scroller) return appSettings.setViewerZoom('docx', z);
    // Mantener a la vista el mismo punto del documento
    const ratioY = scroller.scrollHeight ? (scroller.scrollTop + scroller.clientHeight / 2) / scroller.scrollHeight : 0;
    appSettings.setViewerZoom('docx', z);
    requestAnimationFrame(() => {
      if (scroller) scroller.scrollTop = ratioY * scroller.scrollHeight - scroller.clientHeight / 2;
    });
  }

  /** La página más ancha ocupa todo el ancho disponible */
  function fitWidth() {
    const pages = body ? [...body.querySelectorAll<HTMLElement>('section.docx')] : [];
    if (!scroller || !pages.length) return;
    const pageWidth = Math.max(...pages.map((p) => p.getBoundingClientRect().width)) / zoom;
    const padding = 32; // relleno lateral del fondo gris (también escala)
    setZoom((scroller.clientWidth - 16) / (pageWidth + padding));
  }

  // Ctrl+rueda: zoom (no pasivo para poder evitar el desplazamiento)
  $effect(() => {
    const el = scroller;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if (!e.ctrlKey || !e.deltaY) return;
      e.preventDefault();
      setZoom(stepViewerZoom(appSettings.docxZoom, e.deltaY < 0 ? 1 : -1));
    };
    el.addEventListener('wheel', onWheel, { passive: false });
    return () => el.removeEventListener('wheel', onWheel);
  });

  // Búsqueda (Ctrl+F)
  let searchOpen = $state(false);
  let searchRequest = $state(0);
  export function triggerSearch() {
    searchOpen = true;
    searchRequest++;
  }
  function closeSearch() {
    searchOpen = false;
    scroller?.focus();
  }

  function openExternal() {
    systemApi.openFileWithDefaultApp(path).catch((e) => console.warn('No se pudo abrir con la aplicación del sistema:', e));
  }
</script>

<div class="docx-viewer" data-path={path}>
  <div class="docx-toolbar">
    <span class="docx-status">
      {#if loading}{t('docxViewer.loading')}{:else if error}<TriangleAlert size={14} /> {error}{:else}{t('docxViewer.readOnly')}{/if}
    </span>
    <ZoomControl {zoom} onChange={setZoom} onFit={fitWidth} />
    <button type="button" class="open-external" title={t('docxViewer.openExternalTitle')} onclick={openExternal}>
      <ExternalLink size={14} /> {t('docxViewer.openExternal')}
    </button>
  </div>
  {#if searchOpen && body}
    <DomFindBar scroller={scroller ?? body} request={searchRequest} placeholder={t('find.document')} onClose={closeSearch} />
  {/if}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="docx-scroll" tabindex="-1" bind:this={scroller}>
    <div class="docx-styles" bind:this={styles}></div>
    <div class="docx-body" style={`zoom:${zoom}`} bind:this={body}></div>
  </div>
</div>

<style>
  .docx-viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--bg-primary);
    color: var(--text-primary);
  }
  .docx-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    min-height: 30px;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    font-size: 12px;
  }
  .docx-status {
    flex: 1;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
  }
  .open-external {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border: 1px solid var(--border-primary);
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
  }
  .open-external:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .docx-scroll:focus {
    outline: none;
  }
  .docx-scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  /* El fondo gris de docx-preview, con las páginas blancas como papel */
  /* Centrado con márgenes automáticos (no con flex): con zoom, una página más ancha que la
     ventana no se corta por la izquierda, se llega a todo desplazando */
  .docx-body :global(.docx-wrapper) {
    background: var(--bg-secondary);
    padding: 24px 16px;
    align-items: flex-start;
    width: max-content;
    min-width: 100%;
    box-sizing: border-box;
  }
  .docx-body :global(.docx-wrapper > section.docx) {
    margin-left: auto;
    margin-right: auto;
    margin-bottom: 24px;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.35);
    color: #000;
  }
</style>
