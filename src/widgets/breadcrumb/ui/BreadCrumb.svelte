<script lang="ts">
  import { tick } from 'svelte';
  import { flushPendingWrites } from '@shared/lib/pending-writes';
  import {
    ChevronRight,
    Edit2,
    ExternalLink,
    FolderSearch,
    Copy,
    GitCompare,
    GitCompareArrows,
    GitCommitHorizontal,
    Plus,
    Undo2,
    RotateCcw,
    History,
    Columns,
    Check,
  } from 'lucide-svelte';
  import { ContextMenu } from 'bits-ui';
  import { FolderIcon } from '@shared/ui/icons';
  import { FileIcon, isImageFile, isPdfFile, isDrawingFile, isDiagramFile } from '@entities/file-type';
  import { parseGitStatusInfo, gitDialogState, gitApi, GitStatusBadge, type GitFileStatusKind } from '@entities/git';
  import { GitRestoreDialog } from '@shared/ui/git-restore-dialog';

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
    onRefreshGit?: () => Promise<void> | void;
    onGitRestore?: (paths: string[]) => Promise<void> | void;
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
    onOpenDiff,
    onRefreshGit,
    onGitRestore,
  }: Props = $props();

  interface FolderSegment {
    name: string;
    path: string;
  }

  let isEditing = $state(false);
  let editValue = $state('');
  let inputEl = $state<HTMLInputElement | null>(null);
  let isSubmitting = $state(false);

  // Estado para diálogo de Git Restore y Toast
  let isRestoreDialogOpen = $state(false);
  let isRestoring = $state(false);
  let toastMessage = $state<string | null>(null);
  let toastTimeout: ReturnType<typeof setTimeout> | null = null;

  function showToast(msg: string) {
    if (toastTimeout) clearTimeout(toastTimeout);
    toastMessage = msg;
    toastTimeout = setTimeout(() => {
      toastMessage = null;
      toastTimeout = null;
    }, 2500);
  }

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

  // Capacidades de Git para el archivo actual
  let canGitAdd = $derived.by(() => {
    if (!isGitRepo || !fileGitStatus) return false;
    const isModifiedOrDeletedNotStashed =
      (fileGitStatus.worktree === 'M' || fileGitStatus.worktree === 'D') && !fileGitStatus.is_stashed;
    const isUntracked = fileGitStatus.worktree === '?' && !fileGitStatus.is_stashed;
    return Boolean(isModifiedOrDeletedNotStashed || isUntracked);
  });

  let canGitRestoreStaged = $derived.by(() => {
    if (!isGitRepo || !fileGitStatus) return false;
    return Boolean(fileGitStatus.index);
  });

  let canGitRestore = $derived.by(() => {
    if (!isGitRepo || !fileGitStatus) return false;
    return fileGitStatus.worktree === 'M' || fileGitStatus.worktree === 'D';
  });

  let canGitCommit = $derived.by(() => {
    if (!isGitRepo || !fileGitStatus) return false;
    const hasStage = Boolean(fileGitStatus.index);
    const isModifiedOrDeletedNotStashed =
      (fileGitStatus.worktree === 'M' || fileGitStatus.worktree === 'D') && !fileGitStatus.is_stashed;
    const isUntracked = fileGitStatus.worktree === '?' && !fileGitStatus.is_stashed;
    return Boolean(hasStage || isModifiedOrDeletedNotStashed || isUntracked);
  });

  let canGitDiff = $derived.by(() => {
    if (!isGitRepo || !cleanPath) return false;
    return Boolean(fileGitStatus?.worktree || (!fileGitStatus?.index && isGitRepo));
  });

  let canGitDiffStaged = $derived.by(() => {
    if (!isGitRepo || !cleanPath) return false;
    return Boolean(fileGitStatus?.index);
  });

  let isVisualCandidateForDiff = $derived.by(() => {
    if (!cleanPath) return false;
    return (
      isImageFile(cleanPath) ||
      isPdfFile(cleanPath) ||
      isDrawingFile(cleanPath) ||
      isDiagramFile(cleanPath)
    );
  });

  // Acciones de Git
  async function handleGitAdd() {
    if (!cleanPath) return;
    try {
      await flushPendingWrites([cleanPath]);
      await gitApi.add([cleanPath]);
      showToast('Elemento añadido al stage (git add)');
      if (onRefreshGit) onRefreshGit();
    } catch (err: unknown) {
      console.error('Error al ejecutar git add desde breadcrumb:', err);
      const msg = err instanceof Error ? err.message : String(err);
      showToast(`Error en git add: ${msg}`);
    }
  }

  async function handleGitRestoreStaged() {
    if (!cleanPath) return;
    try {
      await gitApi.restoreStaged([cleanPath]);
      showToast('Elemento desmarcado de stage (git restore --staged)');
      if (onRefreshGit) onRefreshGit();
    } catch (err: unknown) {
      console.error('Error al ejecutar git restore --staged desde breadcrumb:', err);
      const msg = err instanceof Error ? err.message : String(err);
      showToast(`Error en git restore --staged: ${msg}`);
    }
  }

  function handlePromptGitRestore() {
    if (!cleanPath) return;
    isRestoreDialogOpen = true;
  }

  function cancelGitRestore() {
    if (isRestoring) return;
    isRestoreDialogOpen = false;
  }

  async function confirmGitRestore() {
    if (!cleanPath || isRestoring) return;
    isRestoring = true;
    try {
      await gitApi.restore([cleanPath]);
      showToast('Elemento restaurado (git restore)');
      if (onGitRestore) {
        await onGitRestore([cleanPath]);
      }
      if (onRefreshGit) {
        onRefreshGit();
      }
      isRestoreDialogOpen = false;
    } catch (err: unknown) {
      console.error('Error al restaurar con git restore desde breadcrumb:', err);
      const msg = err instanceof Error ? err.message : String(err);
      showToast(`Error en git restore: ${msg}`);
    } finally {
      isRestoring = false;
    }
  }

  function handlePromptGitCommit() {
    if (!cleanPath) return;
    gitDialogState.openCommitDialog([cleanPath]);
  }

  function handleOpenFileLog() {
    if (!cleanPath) return;
    gitDialogState.openFileLog(cleanPath);
  }

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
                        <GitStatusBadge status={fileGitStatus} class="breadcrumb-git-badges" />
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

                    <!-- Opciones de Git -->
                    {#if isGitRepo}
                      <ContextMenu.Separator class="context-menu-divider" />

                      {#if canGitCommit}
                        <ContextMenu.Item
                          class="context-menu-item"
                          onSelect={handlePromptGitCommit}
                        >
                          <GitCommitHorizontal size={14} class="context-menu-item-icon" />
                          <span>Hacer commit...</span>
                        </ContextMenu.Item>
                      {/if}

                      {#if onOpenDiff}
                        <ContextMenu.Item
                          class="context-menu-item"
                          disabled={!canGitDiff}
                          onSelect={() => onOpenDiff(cleanPath, false)}
                        >
                          {#if isVisualCandidateForDiff}
                            <Columns size={14} class="context-menu-item-icon" />
                            <span>Diff visual (2 paneles)</span>
                          {:else}
                            <GitCompare size={14} class="context-menu-item-icon" />
                            <span>Diff</span>
                          {/if}
                        </ContextMenu.Item>

                        <ContextMenu.Item
                          class="context-menu-item"
                          disabled={!canGitDiffStaged}
                          onSelect={() => onOpenDiff(cleanPath, true)}
                        >
                          {#if isVisualCandidateForDiff}
                            <Columns size={14} class="context-menu-item-icon" />
                            <span>Diff visual --staged</span>
                          {:else}
                            <GitCompareArrows size={14} class="context-menu-item-icon" />
                            <span>Diff --staged</span>
                          {/if}
                        </ContextMenu.Item>
                      {/if}

                      <ContextMenu.Item
                        class="context-menu-item"
                        onSelect={handleOpenFileLog}
                      >
                        <History size={14} class="context-menu-item-icon" />
                        <span>Historial de Git (Log)...</span>
                      </ContextMenu.Item>

                      {#if canGitAdd}
                        <ContextMenu.Item
                          class="context-menu-item"
                          onSelect={handleGitAdd}
                        >
                          <Plus size={14} class="context-menu-item-icon" />
                          <span>Git Add</span>
                        </ContextMenu.Item>
                      {/if}

                      {#if canGitRestoreStaged}
                        <ContextMenu.Item
                          class="context-menu-item"
                          onSelect={handleGitRestoreStaged}
                        >
                          <Undo2 size={14} class="context-menu-item-icon" />
                          <span>Git Restore --staged</span>
                        </ContextMenu.Item>
                      {/if}

                      {#if canGitRestore}
                        <ContextMenu.Item
                          class="context-menu-item"
                          onSelect={handlePromptGitRestore}
                        >
                          <RotateCcw size={14} class="context-menu-item-icon" />
                          <span>Git Restore</span>
                        </ContextMenu.Item>
                      {/if}
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

  <GitRestoreDialog
    bind:open={isRestoreDialogOpen}
    items={cleanPath ? [cleanPath] : []}
    loading={isRestoring}
    onConfirm={confirmGitRestore}
    onCancel={cancelGitRestore}
  />

  {#if toastMessage}
    <div class="breadcrumb-toast">
      <Check class="breadcrumb-toast-icon" />
      <span>{toastMessage}</span>
    </div>
  {/if}
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
    gap: 6px;
    padding: 3px 6px;
    border: none;
    background: transparent;
    border-radius: 4px;
    font-size: 12px;
    font-family: inherit;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    transition: background-color 0.15s ease, color 0.15s ease;
    max-width: 260px;
    outline: none;
  }

  .breadcrumb-btn:hover:not(:disabled) {
    background: var(--bg-secondary, rgba(0, 0, 0, 0.05));
    color: var(--text-primary, #1f2328);
  }

  .breadcrumb-btn:focus-visible {
    box-shadow: 0 0 0 2px var(--accent, #0969da);
  }

  .breadcrumb-btn.file-btn {
    font-weight: 600;
    color: var(--text-primary, #1f2328);
  }

  .breadcrumb-btn.file-btn.editable {
    cursor: pointer;
  }

  .breadcrumb-btn:disabled {
    cursor: default;
    opacity: 0.9;
  }

  :global(.breadcrumb-icon) {
    flex-shrink: 0;
    opacity: 0.85;
  }

  .segment-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-title {
    max-width: 220px;
  }

  :global(.edit-icon) {
    opacity: 0;
    transition: opacity 0.15s ease;
    margin-left: 2px;
    flex-shrink: 0;
    color: var(--text-muted, #8c959f);
  }

  .breadcrumb-btn:hover :global(.edit-icon) {
    opacity: 0.8;
  }

  .separator {
    display: inline-flex;
    align-items: center;
    color: var(--text-muted, #8c959f);
    opacity: 0.6;
    margin: 0 1px;
    user-select: none;
  }

  :global(.breadcrumb-git-badges) {
    margin-left: 4px;
  }

  /* Menú Contextual Bits-UI */
  :global(.breadcrumb-context-menu) {
    min-width: 210px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    box-shadow: var(--popover-shadow, 0 8px 24px rgba(0, 0, 0, 0.12));
    border-radius: 8px;
    padding: 4px;
    z-index: 9999;
    animation: scaleIn 0.1s ease-out;
    outline: none;
  }

  @keyframes scaleIn {
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

  :global(.breadcrumb-context-menu .context-menu-item[data-disabled]),
  :global(.breadcrumb-context-menu .context-menu-item:disabled) {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
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

  /* Notificación Toast rápida */
  .breadcrumb-toast {
    position: fixed;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--bg-tertiary, #1f2328);
    color: var(--text-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 6px;
    box-shadow: var(--popover-shadow, 0 4px 14px rgba(0, 0, 0, 0.25));
    z-index: 10000;
    pointer-events: none;
    animation: toast-fade-in 0.15s ease-out;
  }

  :global(.breadcrumb-toast-icon) {
    width: 14px;
    height: 14px;
    color: #2da44e;
    flex-shrink: 0;
  }

  @keyframes toast-fade-in {
    from {
      opacity: 0;
      transform: translate(-50%, 6px);
    }
    to {
      opacity: 1;
      transform: translate(-50%, 0);
    }
  }
</style>
