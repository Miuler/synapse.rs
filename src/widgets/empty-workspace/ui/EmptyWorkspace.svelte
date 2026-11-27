<script lang="ts">
  import { DropdownMenu } from 'bits-ui';
  import {
    FileText, Plus, ChevronRight, ChevronDown, Workflow, Network, Shapes, PenTool, PencilRuler, Kanban,
  } from 'lucide-svelte';
  import { appSettings, useWallpaper } from '@entities/settings';
  import { FileIcon } from '@entities/file-type';
  import { vaultItemStore } from '@entities/vault-item';
  import { NEW_FILE_KINDS, type NewFileKind } from '@features/markdown-editor/lib/folder-index-actions';
  import { QuickOpen } from '@widgets/quick-open';
  import CustomizeEmptyTab from './CustomizeEmptyTab.svelte';
  import ShortcutTip from './ShortcutTip.svelte';
  import { t } from '@shared/lib/i18n';

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
  // Con el buscador oculto no queda nada escrito que tape lo demás
  const searching = $derived(appSettings.emptyTab.search && searchQuery.trim().length > 0);

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
  const shown = $derived(appSettings.emptyTab);
  const wallpaper = useWallpaper();
</script>

<div
  class="empty-workspace synapse-empty-tab"
  class:has-wallpaper={!!wallpaper.style}
  style={wallpaper.style}
>
  {#if hasVaultItems && onSelectFile && shown.search}
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

  {#if shown.recents && onSelectFile && lastOpened.length > 0 && !searching}
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

  {#if !hasVaultItems}
    <div class="empty-message">
      <h2>{t('emptyTab.noFiles')}</h2>
      <p>{t('emptyTab.noFilesHint')}</p>
    </div>
  {/if}

  {#if shown.create && onCreateNew && !searching}
    <DropdownMenu.Root>
      <DropdownMenu.Trigger class="create-btn">
        <Plus size={16} />
        <span>{t('emptyTab.create')}</span>
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

  {#if shown.tips && !searching}
    <ShortcutTip />
  {/if}

  <CustomizeEmptyTab {active} />
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
    gap: 8px;
    padding: 20px 28px;
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-primary, #ffffff) 82%, transparent);
    z-index: 1;
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
    flex-shrink: 0;
    z-index: 1;
    padding: 8px 16px;
    background-color: color-mix(in srgb, var(--bg-primary, #ffffff) 90%, transparent);
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
