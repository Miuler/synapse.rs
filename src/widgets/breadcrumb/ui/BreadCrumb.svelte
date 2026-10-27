<script lang="ts">
  import { tick } from 'svelte';
  import { ChevronRight, Edit2, ExternalLink, FolderSearch, Copy, GitCompare } from 'lucide-svelte';
  import { ContextMenu } from 'bits-ui';
  import { FolderIcon } from '@shared/ui/icons';
  import { FileIcon } from '@entities/file-type';
  import { parseGitStatusInfo, type GitFileStatusKind } from '@entities/git';

  interface Props {
    path?: string;
    readOnly?: boolean;
    class?: string;
    isGitRepo?: boolean;
    gitStatuses?: Record<string, GitFileStatusKind>;
    onNavigateFolder?: (folderPath: string) => void;
    onRenameFile?: (newName: string) => Promise<string | void> | string | void;
    onOpenFileWithDefaultApp?: (path: string) => void;
    onShowItemInFolder?: (path: string) => void;
    onOpenInNewWindow?: (path: string) => void;
    onOpenDiff?: (path: string, staged: boolean) => void;
  }

  let {
    path = '',
    readOnly = false,
    class: customClass = '',
    isGitRepo = false,
    gitStatuses,
    onNavigateFolder,
    onRenameFile,
    onOpenFileWithDefaultApp,
    onShowItemInFolder,
    onOpenInNewWindow,
    onOpenDiff
  }: Props = $props();

  interface FolderSegment {
    name: string;
    path: string;
  }

  let isEditing = $state(false);
  let editValue = $state('');
  let inputEl = $state<HTMLInputElement | null>(null);
  let isSubmitting = $state(false);

  // Normalizar y dividir la ruta
  let cleanPath = $derived(
    path && !path.startsWith('empty:')
      ? path.replace(/\\/g, '/').replace(/^\/+|\/+$/g, '')
      : ''
  );

  let parts = $derived(cleanPath ? cleanPath.split('/').filter(Boolean) : []);

  let fileName = $derived(
    parts.length > 0
      ? parts[parts.length - 1]
      : path.startsWith('empty:')
        ? 'Nueva nota'
        : ''
  );

  let folders = $derived.by<FolderSegment[]>(() => {
    if (parts.length <= 1) return [];
    let accumulated = '';
    return parts.slice(0, -1).map((segment) => {
      accumulated = accumulated ? `${accumulated}/${segment}` : segment;
      return { name: segment, path: accumulated };
    });
  });

  // Estado de Git normalizado para el archivo actual
  let fileGitStatus = $derived.by(() => {
    if (!isGitRepo || !gitStatuses || !cleanPath) return null;
    const raw = gitStatuses[cleanPath] ?? gitStatuses[cleanPath.replace(/^\.\//, '')];
    return parseGitStatusInfo(raw);
  });

  function startEditing() {
    if (readOnly || !fileName || path.startsWith('empty:') || isEditing) return;
    isEditing = true;
    editValue = fileName;

    tick().then(() => {
      if (inputEl) {
        inputEl.focus();
        // Seleccionar solo el nombre del archivo sin su extensión para facilitar el renombrado
        const lastDot = editValue.lastIndexOf('.');
        if (lastDot > 0) {
          inputEl.setSelectionRange(0, lastDot);
        } else {
          inputEl.select();
        }
      }
    });
  }

  async function handleConfirmRename() {
    if (!isEditing || isSubmitting) return;

    const trimmed = editValue.trim();
    if (!trimmed || trimmed === fileName) {
      isEditing = false;
      return;
    }

    try {
      isSubmitting = true;
      if (onRenameFile) {
        await onRenameFile(trimmed);
      }
    } catch (err) {
      console.error('Error al renombrar archivo desde breadcrumb:', err);
    } finally {
      isSubmitting = false;
      isEditing = false;
    }
  }

  function handleCancel() {
    if (isSubmitting) return;
    isEditing = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleConfirmRename();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      handleCancel();
    }
  }

  function handleCopyPath() {
    if (cleanPath) {
      navigator.clipboard.writeText(cleanPath);
    }
  }
</script>

{#if cleanPath || path.startsWith('empty:')}
  <nav aria-label="Breadcrumb" class="synapse-breadcrumb {customClass}">
    {#if isEditing}
      <!-- Modo edición: se oculta el path completo y solo se muestra el input del archivo -->
      <div class="breadcrumb-edit-wrapper">
        <FileIcon path={cleanPath} name={fileName} size={14} class="breadcrumb-icon" />
        <input
          bind:this={inputEl}
          type="text"
          class="breadcrumb-rename-input"
          bind:value={editValue}
          onkeydown={handleKeydown}
          onblur={handleConfirmRename}
          disabled={isSubmitting}
          spellcheck="false"
        />
        <span class="breadcrumb-edit-hint">Enter para guardar &bull; Esc para cancelar</span>
      </div>
    {:else}
      <!-- Modo normal: carpetas + separadores + nombre de archivo -->
      <ol class="breadcrumb-list">
        {#each folders as folder (folder.path)}
          <li class="breadcrumb-item">
            <button
              type="button"
              class="breadcrumb-btn folder-btn"
              onclick={() => onNavigateFolder?.(folder.path)}
              title={`Navegar y enfocar ${folder.name} en el explorador`}
            >
              <FolderIcon name={folder.name} size={14} class="breadcrumb-icon" />
              <span class="segment-text">{folder.name}</span>
            </button>
            <span class="separator" aria-hidden="true">
              <ChevronRight size={13} />
            </span>
          </li>
        {/each}

        {#if fileName}
          <li class="breadcrumb-item active">
            {#if path.startsWith('empty:')}
              <button
                type="button"
                class="breadcrumb-btn file-btn"
                disabled
              >
                <FileIcon path={cleanPath} name={fileName} size={14} class="breadcrumb-icon" />
                <span class="segment-text file-title">{fileName}</span>
              </button>
            {:else}
              <ContextMenu.Root>
                <ContextMenu.Trigger>
                  {#snippet child({ props })}
                    <div {...props} class="breadcrumb-file-trigger">
                      <button
                        type="button"
                        class="breadcrumb-btn file-btn"
                        class:editable={!readOnly}
                        onclick={startEditing}
                        title={!readOnly ? 'Clic para renombrar • Clic derecho para opciones' : fileName}
                        disabled={readOnly}
                      >
                        <FileIcon path={cleanPath} name={fileName} size={14} class="breadcrumb-icon" />
                        <span class="segment-text file-title">{fileName}</span>
                        {#if !readOnly}
                          <Edit2 size={11} class="edit-icon" />
                        {/if}
                      </button>

                      {#if fileGitStatus}
                        <div class="git-badges-group" title="Estado Git de este archivo">
                          {#if fileGitStatus.index === 'M'}
                            <span class="git-badge staged" title="Modificado y en stage (preparado)">M</span>
                          {:else if fileGitStatus.index === 'A'}
                            <span class="git-badge staged" title="Nuevo archivo en stage (preparado)">A</span>
                          {/if}

                          {#if fileGitStatus.worktree === 'M'}
                            <span class="git-badge modified" title="Modificaciones locales sin preparar">M</span>
                          {:else if fileGitStatus.worktree === '?'}
                            <span class="git-badge untracked" title="No controlado por Git (incógnita)">?</span>
                          {/if}

                          {#if fileGitStatus.is_stashed}
                            <span class="git-badge stashed" title="En Stash de Git">S</span>
                          {/if}
                        </div>
                      {/if}
                    </div>
                  {/snippet}
                </ContextMenu.Trigger>

                <ContextMenu.Portal>
                  <ContextMenu.Content class="breadcrumb-context-menu">
                    {#if onOpenInNewWindow}
                      <ContextMenu.Item
                        class="context-menu-item"
                        onSelect={() => onOpenInNewWindow(cleanPath)}
                      >
                        <ExternalLink size={14} class="context-menu-item-icon" />
                        <span>Abrir en nueva ventana</span>
                      </ContextMenu.Item>
                    {/if}

                    {#if fileGitStatus?.worktree === 'M' && onOpenDiff}
                      <ContextMenu.Item
                        class="context-menu-item"
                        onSelect={() => onOpenDiff(cleanPath, false)}
                      >
                        <GitCompare size={14} class="context-menu-item-icon" />
                        <span>Ver diferencias (Git Diff)</span>
                      </ContextMenu.Item>
                    {/if}

                    {#if (fileGitStatus?.index === 'M' || fileGitStatus?.index === 'A') && onOpenDiff}
                      <ContextMenu.Item
                        class="context-menu-item"
                        onSelect={() => onOpenDiff(cleanPath, true)}
                      >
                        <GitCompare size={14} class="context-menu-item-icon" />
                        <span>Ver diferencias Staged (Git Diff)</span>
                      </ContextMenu.Item>
                    {/if}

                    {#if onOpenFileWithDefaultApp}
                      <ContextMenu.Item
                        class="context-menu-item"
                        onSelect={() => onOpenFileWithDefaultApp(cleanPath)}
                      >
                        <ExternalLink size={14} class="context-menu-item-icon" />
                        <span>Abrir con la app por defecto</span>
                      </ContextMenu.Item>
                    {/if}

                    {#if onShowItemInFolder}
                      <ContextMenu.Item
                        class="context-menu-item"
                        onSelect={() => onShowItemInFolder(cleanPath)}
                      >
                        <FolderSearch size={14} class="context-menu-item-icon" />
                        <span>Mostrar en carpeta</span>
                      </ContextMenu.Item>
                    {/if}

                    <ContextMenu.Separator class="context-menu-divider" />

                    {#if !readOnly}
                      <ContextMenu.Item
                        class="context-menu-item"
                        onSelect={startEditing}
                      >
                        <Edit2 size={14} class="context-menu-item-icon" />
                        <span>Renombrar archivo</span>
                      </ContextMenu.Item>
                    {/if}

                    <ContextMenu.Item
                      class="context-menu-item"
                      onSelect={handleCopyPath}
                    >
                      <Copy size={14} class="context-menu-item-icon" />
                      <span>Copiar ruta</span>
                    </ContextMenu.Item>
                  </ContextMenu.Content>
                </ContextMenu.Portal>
              </ContextMenu.Root>
            {/if}
          </li>
        {/if}
      </ol>
    {/if}
  </nav>
{/if}

<style>
  .synapse-breadcrumb {
    display: flex;
    align-items: center;
    height: 32px;
    min-height: 32px;
    padding: 0 16px;
    background: var(--bg-primary, #ffffff);
    border-bottom: 1px solid var(--border-primary, rgba(0, 0, 0, 0.08));
    font-size: 12px;
    color: var(--text-secondary, #656d76);
    user-select: none;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
    flex-shrink: 0;
  }

  .synapse-breadcrumb::-webkit-scrollbar {
    display: none;
  }

  .breadcrumb-list {
    display: flex;
    align-items: center;
    list-style: none;
    margin: 0;
    padding: 0;
    gap: 2px;
    white-space: nowrap;
  }

  .breadcrumb-item {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }

  .breadcrumb-file-trigger {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .breadcrumb-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 6px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    color: var(--text-secondary, #656d76);
    font-size: 12px;
    font-family: inherit;
    line-height: 1;
    cursor: pointer;
    transition:
      background-color 0.15s ease,
      color 0.15s ease,
      border-color 0.15s ease;
  }

  .breadcrumb-btn:hover:not(:disabled) {
    background: var(--hover-bg, rgba(0, 0, 0, 0.05));
    color: var(--text-primary, #1f2328);
  }

  .breadcrumb-btn.folder-btn:hover {
    color: var(--accent, #0969da);
  }

  .breadcrumb-btn.file-btn {
    color: var(--text-primary, #1f2328);
    font-weight: 500;
  }

  .breadcrumb-btn.file-btn.editable:hover {
    background: var(--hover-bg, rgba(0, 0, 0, 0.05));
    border-color: var(--border-primary, rgba(0, 0, 0, 0.15));
  }

  .breadcrumb-btn.file-btn.editable:hover :global(.edit-icon) {
    opacity: 1;
  }

  :global(.synapse-breadcrumb .edit-icon) {
    opacity: 0;
    margin-left: 2px;
    color: var(--text-muted, #8c959f);
    transition: opacity 0.15s ease;
  }

  .breadcrumb-btn:disabled {
    cursor: default;
  }

  .separator {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted, #8c959f);
    opacity: 0.6;
    margin: 0 1px;
  }

  .segment-text {
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.breadcrumb-icon) {
    flex-shrink: 0;
  }

  /* Git Badges */
  .git-badges-group {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    flex-shrink: 0;
  }

  .git-badge {
    font-size: 10px;
    font-weight: 700;
    line-height: 1;
    padding: 2px 4px;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .git-badge.staged {
    color: #16a34a;
    background: rgba(22, 163, 74, 0.16);
  }

  .git-badge.modified {
    color: #dc2626;
    background: rgba(220, 38, 38, 0.16);
  }

  .git-badge.untracked {
    color: #ca8a04;
    background: rgba(202, 138, 4, 0.16);
  }

  .git-badge.stashed {
    color: #7c3aed;
    background: rgba(124, 58, 237, 0.16);
  }

  /* Context Menu */
  :global(.breadcrumb-context-menu) {
    min-width: 190px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    padding: 4px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12), 0 2px 6px rgba(0, 0, 0, 0.08);
    z-index: 9999;
    display: flex;
    flex-direction: column;
    user-select: none;
    outline: none;
    animation: breadcrumb-menu-fade 0.1s ease-out;
  }

  @keyframes breadcrumb-menu-fade {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  :global(.breadcrumb-context-menu .context-menu-divider) {
    height: 1px;
    background: var(--border-primary, #d0d7de);
    margin: 4px 0;
  }

  :global(.breadcrumb-context-menu .context-menu-item) {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border: none;
    background: transparent;
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-primary, #1f2328);
    cursor: pointer;
    text-align: left;
    width: 100%;
    transition: background 0.1s ease, color 0.1s ease;
    font-family: inherit;
    outline: none;
    box-sizing: border-box;
  }

  :global(.breadcrumb-context-menu .context-menu-item:hover),
  :global(.breadcrumb-context-menu .context-menu-item[data-highlighted]) {
    background: var(--accent-bg, rgba(9, 105, 218, 0.08));
    color: var(--accent, #0969da);
  }

  :global(.breadcrumb-context-menu .context-menu-item-icon) {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
  }

  /* Modo edición */
  .breadcrumb-edit-wrapper {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    flex: 1;
    min-width: 0;
  }

  .breadcrumb-rename-input {
    flex: 1;
    width: 100%;
    min-width: 0;
    height: 24px;
    padding: 2px 8px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--accent, #0969da);
    border-radius: 4px;
    color: var(--text-primary, #1f2328);
    font-size: 12px;
    font-family: inherit;
    font-weight: 500;
    outline: none;
    box-shadow: 0 0 0 2px var(--accent-bg, rgba(9, 105, 218, 0.2));
  }

  .breadcrumb-edit-hint {
    font-size: 11px;
    color: var(--text-muted, #8c959f);
    white-space: nowrap;
    opacity: 0.8;
  }
</style>
