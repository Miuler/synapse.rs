<script lang="ts">
  import { DropdownMenu } from 'bits-ui';
  import {
    FileText, Plus, ImagePlus, ImageOff, ChevronRight, ChevronDown, Workflow, Network, Shapes, PenTool, PencilRuler, Kanban,
  } from 'lucide-svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { appSettings } from '@entities/settings';
  import { FileIcon } from '@entities/file-type';
  import { vaultItemStore } from '@entities/vault-item';
  import { NEW_FILE_KINDS, type NewFileKind } from '@features/markdown-editor/lib/folder-index-actions';
  import { QuickOpen } from '@widgets/quick-open';
  import { t } from '@shared/lib/i18n';
  import { invokeTauri } from '@shared/repositories/tauri';
  // Fondos por defecto según el tema; Vite los empaqueta y van dentro del binario
  import defaultWallpaperDark from '@shared/assets/synapsers_w_dark.webp';
  import defaultWallpaperLight from '@shared/assets/synapsers_w_light.webp';

  interface Props {
    hasVaultItems?: boolean;
    onCreateNew?: (kind: NewFileKind) => void;
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

  // Un icono y un color por tipo, para distinguirlos de un vistazo (en el árbol los diagramas comparten icono)
  const KIND_LOOK: Record<NewFileKind, { icon: typeof FileText; color: string; ext: string }> = {
    markdown: { icon: FileText, color: 'var(--icon-file-color, var(--accent))', ext: '.md' },
    mermaid: { icon: Workflow, color: '#d6336c', ext: '.mmd' },
    d2: { icon: Network, color: '#1c7ed6', ext: '.d2' },
    plantuml: { icon: Shapes, color: '#bf8700', ext: '.puml' },
    excalidraw: { icon: PenTool, color: '#8250df', ext: '.excalidraw' },
    'obsidian-excalidraw': { icon: PencilRuler, color: '#6741d9', ext: '.excalidraw.md' },
    board: { icon: Kanban, color: '#2da44e', ext: '.board.md' },
  };

  // Los últimos abiertos que siguen en la bóveda
  const lastOpened = $derived(recentFiles.filter((path) => vaultItemStore.get(path)).slice(0, appSettings.recentsCount));
  // Si el fondo ya no se puede leer (se borró o no está autorizado), queda el de por defecto
  let brokenWallpaper = $state<string | null>(null);
  let wallpaperUrl = $derived(
    appSettings.wallpaper && appSettings.wallpaper !== brokenWallpaper ? convertFileSrc(appSettings.wallpaper) : null,
  );
  $effect(() => {
    const path = appSettings.wallpaper;
    if (!path) return;
    const probe = new Image();
    probe.onerror = () => (brokenWallpaper = path);
    probe.src = convertFileSrc(path);
    return () => (probe.onerror = null);
  });

  async function chooseWallpaper() {
    try {
      const path = await invokeTauri<string | null>('pick_image_file');
      if (path) {
        brokenWallpaper = null;
        appSettings.setWallpaper(path);
      }
    } catch (err) {
      console.warn('No se pudo elegir el fondo:', err);
    }
  }
</script>

<div
  class="empty-workspace has-wallpaper synapse-empty-tab"
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

  {#if onSelectFile && lastOpened.length > 0 && !searchQuery.trim()}
    {@const collapsed = appSettings.recentsCollapsed}
    <section class="empty-recents">
      <button
        type="button"
        class="recents-toggle"
        aria-expanded={!collapsed}
        onclick={() => appSettings.setRecentsCollapsed(!collapsed)}
      >
        {#if collapsed}<ChevronRight size={14} />{:else}<ChevronDown size={14} />{/if}
        <span>{t('emptyTab.recentFiles')}</span>
      </button>
      {#if !collapsed}
        {#each lastOpened as path (path)}
          {@const slash = path.lastIndexOf('/')}
          <button type="button" class="recent-file" data-path={path} title={path} onclick={() => onSelectFile(path)}>
            <FileIcon {path} size={15} />
            <span class="recent-name">{path.slice(slash + 1)}</span>
            {#if slash > 0}<span class="recent-folder">{path.slice(0, slash)}</span>{/if}
          </button>
        {/each}
      {/if}
    </section>
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
    <DropdownMenu.Root>
      <DropdownMenu.Trigger class="create-btn">
        <Plus size={16} />
        <span>Crear nuevo archivo</span>
        <ChevronDown size={14} />
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content class="empty-new-menu" sideOffset={6}>
          {#each NEW_FILE_KINDS as { kind, label } (kind)}
            {@const look = KIND_LOOK[kind]}
            <DropdownMenu.Item class="empty-new-item" data-kind={kind} onSelect={() => onCreateNew(kind)}>
              <look.icon size={16} color={look.color} />
              <span class="new-label">{label}</span>
              <span class="new-ext">{look.ext}</span>
            </DropdownMenu.Item>
          {/each}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
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

  .empty-recents {
    display: flex;
    flex-direction: column;
    flex: 0 1 auto;
    min-height: 40px;
    overflow-y: auto;
    width: min(640px, 100%);
    padding: 6px;
    box-sizing: border-box;
    z-index: 1;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-primary, #ffffff) 88%, transparent);
    backdrop-filter: blur(6px);
  }

  .recents-toggle,
  .recent-file {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .recents-toggle {
    font-size: 12px;
    font-weight: 600;
  }

  .recent-file {
    font-size: 13px;
  }

  .recents-toggle:hover,
  .recent-file:hover,
  .recent-file:focus-visible {
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-primary, #1f2328);
  }

  .recent-name {
    flex-shrink: 0;
    max-width: 60%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-primary, #1f2328);
  }

  .recent-folder {
    margin-left: auto;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
  }

  :global(.empty-new-menu) {
    min-width: 320px;
    padding: 6px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 10px;
    box-shadow: var(--popover-shadow);
    z-index: 1000;
    outline: none;
  }

  :global(.empty-new-item) {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    border-radius: 6px;
    font-size: 13px;
    color: var(--text-primary, #1f2328);
    cursor: pointer;
    outline: none;
    user-select: none;
  }

  :global(.empty-new-item[data-highlighted]) {
    background: var(--bg-secondary, #f6f8fa);
  }

  .new-label {
    flex: 1;
    text-align: left;
  }

  .new-ext {
    font-family: var(--mono, monospace);
    font-size: 11px;
    color: var(--text-secondary, #656d76);
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
    color: var(--on-accent);
    border-color: var(--accent, #0969da);
    box-shadow: 0 4px 12px color-mix(in srgb, var(--accent) 20%, transparent);
  }
</style>
