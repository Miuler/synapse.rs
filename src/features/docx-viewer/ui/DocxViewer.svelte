<script lang="ts">
  // Visor de documentos Word (.docx, .docm, .dotx) en solo lectura con docx-preview: páginas,
  // fuentes, colores, tablas, imágenes, encabezados y pies. La librería se carga al abrir el
  // primer documento. Para editar: «Abrir en la aplicación».
  import { vaultRepository } from '@shared/repositories';
  import { systemApi } from '@shared/api';
  import { ExternalLink, TriangleAlert } from 'lucide-svelte';

  interface Props {
    /** Ruta relativa a la bóveda */
    path: string;
    /** Cambia cuando el archivo cambió en disco: se vuelve a leer */
    reloadKey?: number;
  }
  let { path, reloadKey = 0 }: Props = $props();

  let body = $state<HTMLDivElement | null>(null);
  let styles = $state<HTMLDivElement | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let generation = 0;

  async function render() {
    if (!body || !styles) return;
    const gen = ++generation;
    loading = true;
    error = null;
    try {
      const [{ renderAsync }, data] = await Promise.all([import('docx-preview'), vaultRepository.readBinary(path)]);
      if (gen !== generation || !body || !styles) return;
      if (!data.byteLength) throw new Error('No se pudo leer el archivo');
      body.replaceChildren();
      styles.replaceChildren();
      await renderAsync(data, body, styles, {
        className: 'docx',
        inWrapper: true,
        breakPages: true,
        ignoreLastRenderedPageBreak: true,
        renderHeaders: true,
        renderFooters: true,
        renderFootnotes: true,
        renderEndnotes: true,
        renderComments: false,
        renderChanges: false,
        experimental: false,
        useBase64URL: true,
      });
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

  function openExternal() {
    systemApi.openFileWithDefaultApp(path).catch((e) => console.warn('No se pudo abrir con la aplicación del sistema:', e));
  }
</script>

<div class="docx-viewer" data-path={path}>
  <div class="docx-toolbar">
    <span class="docx-status">
      {#if loading}Leyendo documento…{:else if error}<TriangleAlert size={14} /> {error}{:else}Solo lectura{/if}
    </span>
    <button type="button" class="open-external" title="Abrir con la aplicación del sistema para editarlo" onclick={openExternal}>
      <ExternalLink size={14} /> Abrir en la aplicación
    </button>
  </div>
  <div class="docx-scroll">
    <div class="docx-styles" bind:this={styles}></div>
    <div class="docx-body" bind:this={body}></div>
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
  .docx-scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  /* El fondo gris de docx-preview, con las páginas blancas como papel */
  .docx-body :global(.docx-wrapper) {
    background: var(--bg-secondary);
    padding: 24px 16px;
  }
  .docx-body :global(.docx-wrapper > section.docx) {
    margin-bottom: 24px;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.35);
    color: #000;
  }
</style>
