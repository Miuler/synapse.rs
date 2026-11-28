<script lang="ts">
  // Pestaña en memoria con novedades (no es un archivo del vault): las de la versión instalada o
  // las de una versión nueva disponible, con su botón para instalarla.
  import { MarkdownViewer } from '@features/markdown-editor';
  import { Download, Loader2, RotateCw, Sparkles } from 'lucide-svelte';
  import { t } from '@shared/lib/i18n';
  import { appUpdates } from '../model/app-updates.svelte';

  interface Props {
    markdown: string;
    /** Novedades de una versión nueva todavía no instalada */
    pendingUpdate?: boolean;
  }
  let { markdown, pendingUpdate = false }: Props = $props();
</script>

<div class="news-tab">
  {#if pendingUpdate}
    <div class="news-update-bar">
      <Sparkles size={14} />
      {#if appUpdates.status === 'installed'}
        <span>{t('updates.installedBanner', { version: appUpdates.available?.version ?? '' })}</span>
        <button type="button" class="news-btn primary" onclick={() => appUpdates.restart()}>
          <RotateCw size={13} /> {t('updates.restartNow')}
        </button>
      {:else if appUpdates.status === 'installing'}
        <span><Loader2 size={13} class="news-spin" /> {t('updates.downloadingAndInstalling')}</span>
      {:else if appUpdates.available}
        <span>{t('updates.availableBanner', { version: appUpdates.available.version, current: appUpdates.available.currentVersion })}</span>
        <button type="button" class="news-btn primary" onclick={() => appUpdates.install()}>
          <Download size={13} /> {t('updates.update')}
        </button>
      {:else}
        <span>{t('updates.alreadyLatest')}</span>
      {/if}
    </div>
    {#if appUpdates.error}
      <div class="news-error" role="alert">{appUpdates.error}</div>
    {/if}
  {/if}
  <div class="news-content">
    <MarkdownViewer content={markdown} readOnly={true} viewMode="reading" isMarkdown={true} />
  </div>
</div>

<style>
  .news-tab {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    background: var(--bg-primary);
    overflow: hidden;
  }
  .news-update-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 16px;
    font-size: 13px;
    color: var(--text-primary);
    background: color-mix(in srgb, var(--accent) 10%, var(--bg-secondary));
    border-bottom: 1px solid var(--border-primary);
  }
  .news-update-bar span {
    flex: 1;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .news-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    border-radius: 6px;
    border: 1px solid var(--border-primary);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
    background: var(--bg-primary);
    color: var(--text-primary);
  }
  .news-btn.primary {
    background: var(--accent);
    color: var(--on-accent);
    border-color: transparent;
  }
  .news-error {
    padding: 8px 16px;
    font-size: 12.5px;
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 10%, transparent);
  }
  .news-content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  :global(.news-spin) {
    animation: news-spin 1s linear infinite;
  }
  @keyframes news-spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
