<script lang="ts">
  import { Button } from 'bits-ui';
  import { FileText, Plus, ImagePlus, ImageOff } from 'lucide-svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { appSettings } from '@entities/settings';
  import { QuickOpen } from '@widgets/quick-open';
  import { invokeTauri } from '@shared/repositories/tauri';
  // Fondos por defecto según el tema; Vite los empaqueta y van dentro del binario
  import defaultWallpaperDark from '@shared/assets/synapsers_w_dark.avif';
  import defaultWallpaperLight from '@shared/assets/synapsers_w_light.avif';

  interface Props {
    hasVaultItems?: boolean;
    onCreateNew?: () => void;
    /** Pestaña visible: el buscador toma el foco al activarse. */
    active?: boolean;
    recentFiles?: string[];
    onSelectFile?: (path: string, matchedTerms?: string[]) => void;
    onRevealFolder?: (folderPath: string) => void;
  }

  let {
    hasVaultItems = true,
    onCreateNew,
    active = true,
    recentFiles = [],
    onSelectFile,
    onRevealFolder
  }: Props = $props();

  let searchQuery = $state('');
  let wallpaperUrl = $derived(appSettings.wallpaper ? convertFileSrc(appSettings.wallpaper) : null);

  async function chooseWallpaper() {
    try {
      const path = await invokeTauri<string | null>('pick_image_file');
      if (path) appSettings.setWallpaper(path);
    } catch (err) {
      console.warn('No se pudo elegir el fondo:', err);
    }
  }
</script>

<div
  class="empty-workspace has-wallpaper"
  class:has-search-open={searchQuery.trim().length > 0}
  style={wallpaperUrl
    ? `--wallpaper: url("${wallpaperUrl}")`
    : `--wallpaper-light: url("${defaultWallpaperLight}"); --wallpaper-dark: url("${defaultWallpaperDark}")`}
>
  {#if hasVaultItems && onSelectFile}
    <div class="empty-search">
      <QuickOpen
        inline
        isOpen={true}
        focusOnActive={active}
        bind:searchQuery
        {recentFiles}
        {onSelectFile}
        {onRevealFolder}
      />
    </div>
  {/if}

  <div class="empty-message">
  <FileText
    class="empty-icon"
    size={56}
    strokeWidth={1.5}
  />
  <h2>
    {!hasVaultItems
      ? "No hay archivos en la bóveda"
      : "Ningún archivo abierto"}
  </h2>
  <p>
    {!hasVaultItems
      ? "Crea un nuevo archivo para comenzar a escribir."
      : "Selecciona un archivo del panel lateral para abrirlo."}
  </p>
  {#if onCreateNew}
    <Button.Root type="button" class="create-btn" onclick={onCreateNew}>
      <Plus size={16} />
      <span>Crear nuevo archivo</span>
    </Button.Root>
  {/if}
  </div>

  <div class="wallpaper-actions">
    <button type="button" class="wallpaper-btn" onclick={chooseWallpaper} title="Cambiar el fondo de la pestaña vacía">
      <ImagePlus size={14} />
      <span>Cambiar fondo</span>
    </button>
    {#if appSettings.wallpaper}
      <button type="button" class="wallpaper-btn" onclick={() => appSettings.setWallpaper(undefined)} title="Quitar el fondo">
        <ImageOff size={14} />
        <span>Quitar</span>
      </button>
    {/if}
  </div>
</div>

<style>
  .empty-workspace {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    height: 100%;
    width: 100%;
    gap: 12px;
    padding: 12vh 24px 24px;
    box-sizing: border-box;
    overflow: hidden;
    color: var(--text-secondary, #656d76);
    text-align: center;
    user-select: none;
  }

  :global(.empty-icon) {
    width: 56px;
    height: 56px;
    color: var(--border-primary, #d0d7de);
    margin-bottom: 8px;
  }

  /* Sin fondo propio se usa el de la app según el tema */
  .empty-workspace {
    --wallpaper: var(--wallpaper-light);
  }
  :global([data-theme='dark']) .empty-workspace {
    --wallpaper: var(--wallpaper-dark);
  }

  .empty-workspace.has-wallpaper {
    background-image: var(--wallpaper);
    background-size: cover;
    background-position: center;
  }

  .empty-search {
    width: min(640px, 100%);
    z-index: 1;
  }

  .empty-message {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    margin-top: 6vh;
  }

  /* Con fondo, el texto va sobre una tarjeta para que se lea */
  .has-wallpaper .empty-message {
    padding: 20px 28px;
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-primary, #ffffff) 82%, transparent);
    backdrop-filter: blur(6px);
  }

  .has-search-open .empty-message {
    display: none;
  }

  .wallpaper-actions {
    position: absolute;
    right: 16px;
    bottom: 16px;
    display: flex;
    gap: 8px;
  }

  .wallpaper-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    font-size: 12px;
    color: var(--text-secondary, #656d76);
    background: color-mix(in srgb, var(--bg-primary, #ffffff) 85%, transparent);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    cursor: pointer;
  }

  .wallpaper-btn:hover {
    color: var(--text-primary, #1f2328);
    border-color: var(--accent, #0969da);
  }

  .empty-workspace h2 {
    font-size: 20px;
    font-weight: 500;
    color: var(--text-primary, #1f2328);
    margin: 0;
  }

  .empty-workspace p {
    font-size: 14px;
    color: var(--text-secondary, #656d76);
    margin: 0;
  }

  :global(.create-btn) {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
    padding: 8px 16px;
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.1));
    color: var(--accent, #0969da);
    border: 1px solid var(--accent-border, rgba(9, 105, 218, 0.3));
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  }

  :global(.create-btn:hover) {
    background-color: var(--accent, #0969da);
    color: #ffffff;
    border-color: var(--accent, #0969da);
    box-shadow: 0 4px 12px rgba(9, 105, 218, 0.2);
  }
</style>
