<script lang="ts">
  import { tick } from 'svelte';
  import { ChevronRight, Edit2 } from 'lucide-svelte';
  import { FolderIcon } from '@shared/ui/icons';
  import { FileIcon } from '@entities/file-type';

  interface Props {
    path?: string;
    readOnly?: boolean;
    class?: string;
    onNavigateFolder?: (folderPath: string) => void;
    onRenameFile?: (newName: string) => Promise<string | void> | string | void;
  }

  let {
    path = '',
    readOnly = false,
    class: customClass = '',
    onNavigateFolder,
    onRenameFile
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
            <button
              type="button"
              class="breadcrumb-btn file-btn"
              class:editable={!readOnly && !path.startsWith('empty:')}
              onclick={startEditing}
              title={!readOnly && !path.startsWith('empty:') ? 'Clic para renombrar' : fileName}
              disabled={readOnly || path.startsWith('empty:')}
            >
              <FileIcon path={cleanPath} name={fileName} size={14} class="breadcrumb-icon" />
              <span class="segment-text file-title">{fileName}</span>
              {#if !readOnly && !path.startsWith('empty:')}
                <Edit2 size={11} class="edit-icon" />
              {/if}
            </button>
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

  /* Modo edición */
  .breadcrumb-edit-wrapper {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    max-width: 480px;
  }

  .breadcrumb-rename-input {
    flex: 1;
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