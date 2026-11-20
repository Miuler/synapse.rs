<script lang="ts">
  // Aviso flotante (abajo a la derecha) del updater: versión nueva, progreso, instalada o error.
  import { Download, Loader2, RotateCw, Sparkles, X } from 'lucide-svelte';
  import { appUpdates } from '../model/app-updates.svelte';

  interface Props {
    /** Abre la pestaña con las novedades de la versión nueva */
    onShowNotes: () => void;
  }
  let { onShowNotes }: Props = $props();

  const percent = $derived(
    appUpdates.progress?.total ? Math.min(100, Math.round((appUpdates.progress.downloaded / appUpdates.progress.total) * 100)) : null,
  );
</script>

{#if appUpdates.noticeOpen}
  <div class="update-notice" role="status" aria-live="polite">
    <div class="update-notice-head">
      <Sparkles size={15} />
      <strong>
        {#if appUpdates.status === 'available'}Synapse {appUpdates.available?.version} disponible
        {:else if appUpdates.status === 'installing'}Instalando Synapse {appUpdates.available?.version}
        {:else if appUpdates.status === 'installed'}Synapse {appUpdates.available?.version} instalado
        {:else if appUpdates.status === 'up-to-date'}Synapse está al día
        {:else if appUpdates.status === 'error'}No se pudo actualizar
        {:else}Buscando actualizaciones…{/if}
      </strong>
      {#if appUpdates.status !== 'installing'}
        <button type="button" class="update-notice-close" onclick={() => appUpdates.dismiss()} aria-label="Cerrar">
          <X size={14} />
        </button>
      {/if}
    </div>

    {#if appUpdates.status === 'available'}
      <p>Tienes la {appUpdates.available?.currentVersion}.</p>
      <div class="update-notice-actions">
        <button type="button" class="update-btn" onclick={onShowNotes}>Ver novedades</button>
        <button type="button" class="update-btn" onclick={() => appUpdates.dismiss()}>Más tarde</button>
        <button type="button" class="update-btn primary" onclick={() => appUpdates.install()}>
          <Download size={13} /> Actualizar
        </button>
      </div>
    {:else if appUpdates.status === 'installing'}
      <div class="update-progress"><div class="update-progress-bar" style:width={`${percent ?? 15}%`} class:indeterminate={percent === null}></div></div>
      <p><Loader2 size={12} class="update-spin" /> {percent !== null ? `${percent} %` : 'Descargando…'}</p>
    {:else if appUpdates.status === 'installed'}
      <p>Se aplicará al reiniciar.</p>
      <div class="update-notice-actions">
        <button type="button" class="update-btn" onclick={() => appUpdates.dismiss()}>Más tarde</button>
        <button type="button" class="update-btn primary" onclick={() => appUpdates.restart()}>
          <RotateCw size={13} /> Reiniciar ahora
        </button>
      </div>
    {:else if appUpdates.status === 'error'}
      <p class="update-error">{appUpdates.error}</p>
    {/if}
  </div>
{/if}

<style>
  .update-notice {
    position: fixed;
    right: 16px;
    bottom: 36px;
    z-index: 900;
    width: 320px;
    max-width: calc(100vw - 32px);
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.25);
    color: var(--text-primary);
    font-size: 13px;
  }
  .update-notice-head {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--accent);
  }
  .update-notice-head strong {
    flex: 1;
    color: var(--text-primary);
  }
  .update-notice-close {
    display: inline-flex;
    padding: 2px;
    border: none;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
  }
  p {
    margin: 6px 0 0;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .update-error {
    color: var(--danger);
    word-break: break-word;
  }
  .update-notice-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-top: 10px;
  }
  .update-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    border-radius: 6px;
    border: 1px solid var(--border-primary);
    background: var(--bg-primary);
    color: var(--text-primary);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
  }
  .update-btn.primary {
    background: var(--accent);
    color: var(--on-accent);
    border-color: transparent;
  }
  .update-progress {
    height: 5px;
    margin-top: 10px;
    border-radius: 3px;
    background: var(--bg-tertiary);
    overflow: hidden;
  }
  .update-progress-bar {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s ease;
  }
  .update-progress-bar.indeterminate {
    animation: update-indeterminate 1.2s ease-in-out infinite;
  }
  @keyframes update-indeterminate {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(700%);
    }
  }
  :global(.update-spin) {
    animation: update-spin 1s linear infinite;
  }
  @keyframes update-spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
