<script lang="ts">
  import { ResizableDialog } from '@shared/ui/dialog-resize';
  import {
    GitCommitHorizontal,
    GitBranch,
    AlertCircle,
    Loader2,
    Search,
    Trash2,
  } from 'lucide-svelte';

  export interface GitCommitFileItem {
    path: string;
    statusKind?: 'modified' | 'deleted' | 'untracked' | 'added' | 'staged';
    statusLabel?: string;
    isDeleted?: boolean;
  }

  export interface GitCommitDialogProps {
    open?: boolean;
    paths?: string[];
    allFiles?: GitCommitFileItem[];
    branchName?: string | null;
    loading?: boolean;
    errorMessage?: string | null;
    onConfirm: (message: string, selectedPaths: string[]) => void | Promise<void>;
    onCancel?: () => void;
  }


  let {
    open = $bindable(false),
    paths = [],
    allFiles = [],
    branchName = null,
    loading = false,
    errorMessage = null,
    onConfirm,
    onCancel,
  }: GitCommitDialogProps = $props();

  let message = $state('');
  let filterText = $state('');
  let selectedSet = $state<Set<string>>(new Set());
  let textareaEl = $state<HTMLTextAreaElement | null>(null);

  const effectiveFiles = $derived.by<GitCommitFileItem[]>(() => {
    if (allFiles && allFiles.length > 0) {
      return allFiles;
    }
    return paths.map((p) => {
      const clean = p.replace(/^\.\//, '').replace(/\\/g, '/');
      return {
        path: clean,
        statusKind: 'modified',
        statusLabel: 'Modificado',
        isDeleted: false,
      };
    });
  });

  const filteredFiles = $derived.by(() => {
    const q = filterText.trim().toLowerCase();
    if (!q) return effectiveFiles;
    return effectiveFiles.filter((f) => f.path.toLowerCase().includes(q));
  });

  const selectedPaths = $derived(
    effectiveFiles.filter((f) => selectedSet.has(f.path)).map((f) => f.path)
  );

  const allFilteredSelected = $derived(
    filteredFiles.length > 0 && filteredFiles.every((f) => selectedSet.has(f.path))
  );

  const canSubmit = $derived(!loading && message.trim().length > 0 && selectedPaths.length > 0);

  // Auto-enfoque y reseteo al abrir el diálogo
  $effect(() => {
    if (open) {
      message = '';
      filterText = '';

      const initial = new Set<string>();
      if (paths.length > 0) {
        for (const file of effectiveFiles) {
          const filePath = file.path;
          const isMatched = paths.some((p) => {
            const cleanP = p.replace(/^\.\//, '').replace(/\\/g, '/');
            if (filePath === cleanP) return true;
            const prefix = cleanP.endsWith('/') ? cleanP : `${cleanP}/`;
            return filePath.startsWith(prefix);
          });
          if (isMatched) {
            initial.add(filePath);
          }
        }
      }

      // Si no hubo coincidencia con las rutas o no se especificaron rutas previas,
      // seleccionar por defecto todos los archivos con cambios
      if (initial.size === 0 && effectiveFiles.length > 0) {
        for (const file of effectiveFiles) {
          initial.add(file.path);
        }
      }

      selectedSet = initial;

      requestAnimationFrame(() => {
        textareaEl?.focus();
      });
    }
  });

  function toggleFile(path: string) {
    if (loading) return;
    const next = new Set(selectedSet);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    selectedSet = next;
  }

  function toggleSelectAllFiltered() {
    if (loading) return;
    const next = new Set(selectedSet);
    if (allFilteredSelected) {
      for (const f of filteredFiles) {
        next.delete(f.path);
      }
    } else {
      for (const f of filteredFiles) {
        next.add(f.path);
      }
    }
    selectedSet = next;
  }

  function getFilename(fullPath: string): string {
    const parts = fullPath.split('/');
    return parts[parts.length - 1] || fullPath;
  }

  function getDirname(fullPath: string): string {
    const parts = fullPath.split('/');
    if (parts.length <= 1) return '';
    return parts.slice(0, parts.length - 1).join('/');
  }

  function handleCancel() {
    if (loading) return;
    open = false;
    onCancel?.();
  }

  async function handleSubmit() {
    if (!canSubmit) return;
    await onConfirm(message.trim(), selectedPaths);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      handleCancel();
    } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      if (canSubmit) {
        handleSubmit();
      }
    }
  }
</script>

<ResizableDialog
  {open}
  onClose={handleCancel}
  busy={loading}
  title="Hacer commit en Git"
  description="Confirma los cambios de los archivos seleccionados en el historial de Git."
  sizeKey="synapse.gitCommitDialog.size"
  defaultSize={{ w: 820, h: 640 }}
  minSize={{ w: 420, h: 380 }}
  tone="var(--success)"
  class="git-commit-dialog"
  onkeydown={handleKeyDown}
  onOpenAutoFocus={(e) => {
    e.preventDefault();
    requestAnimationFrame(() => textareaEl?.focus());
  }}
>
  {#snippet icon()}<GitCommitHorizontal size={20} />{/snippet}
  {#snippet badge()}
    {#if branchName}
      <span class="git-commit-branch-badge" title="Rama actual: {branchName}">
        <GitBranch size={11} />
        <span>{branchName}</span>
      </span>
    {/if}
  {/snippet}
  {#snippet footer()}
        <button
          type="button"
          class="git-commit-btn cancel"
          onclick={handleCancel}
          disabled={loading}
        >
          Cancelar
        </button>
        <button
          type="button"
          class="git-commit-btn submit"
          onclick={handleSubmit}
          disabled={!canSubmit}
        >
          {#if loading}
            <Loader2 size={14} class="spin-icon" />
            <span>Haciendo commit...</span>
          {:else}
            <GitCommitHorizontal size={14} />
            <span>Hacer commit ({selectedPaths.length})</span>
          {/if}
        </button>
  {/snippet}

      <!-- Lista de archivos modificados/borrados con selector -->
      <div class="git-commit-files-box">
        <div class="git-commit-files-header">
          <span class="git-commit-files-title">
            Archivos a confirmar ({selectedPaths.length} de {effectiveFiles.length} seleccionados):
          </span>
          {#if effectiveFiles.length > 0}
            <button
              type="button"
              class="git-commit-toggle-all-btn"
              onclick={toggleSelectAllFiltered}
              disabled={loading}
            >
              {allFilteredSelected ? 'Deseleccionar todos' : 'Seleccionar todos'}
            </button>
          {/if}
        </div>

        {#if effectiveFiles.length > 6}
          <div class="git-commit-search-bar">
            <Search size={12} class="git-commit-search-icon" />
            <input
              type="text"
              bind:value={filterText}
              placeholder="Filtrar archivos..."
              class="git-commit-search-input"
              disabled={loading}
            />
            {#if filterText}
              <button
                type="button"
                class="git-commit-search-clear"
                onclick={() => (filterText = '')}
                title="Limpiar filtro"
              >
                ✕
              </button>
            {/if}
          </div>
        {/if}

        <div class="git-commit-files-list">
          {#each filteredFiles as file (file.path)}
            {@const isChecked = selectedSet.has(file.path)}
            {@const isDeleted = file.isDeleted || file.statusKind === 'deleted'}
            <label
              class="git-commit-file-item"
              class:checked={isChecked}
              class:is-deleted={isDeleted}
              title={file.path}
            >
              <input
                type="checkbox"
                checked={isChecked}
                onchange={() => toggleFile(file.path)}
                disabled={loading}
                class="git-commit-checkbox"
              />

              <span
                class="git-status-badge status-{file.statusKind || 'modified'}"
                title={file.statusLabel || file.statusKind}
              >
                {#if isDeleted}
                  <Trash2 size={10} />
                  <span>Eliminado</span>
                {:else if file.statusKind === 'untracked'}
                  <span>Nuevo</span>
                {:else if file.statusKind === 'added'}
                  <span>Añadido</span>
                {:else if file.statusKind === 'staged'}
                  <span>Staged</span>
                {:else}
                  <span>Modificado</span>
                {/if}
              </span>

              <span class="git-commit-file-path">
                <span class="git-commit-file-name" class:strikethrough={isDeleted}>
                  {getFilename(file.path)}
                </span>
                {#if getDirname(file.path)}
                  <span class="git-commit-file-dir">{getDirname(file.path)}/</span>
                {/if}
              </span>
            </label>
          {:else}
            <div class="git-commit-empty-files">
              {filterText ? 'No hay archivos que coincidan con el filtro' : 'No hay archivos con cambios'}
            </div>
          {/each}
        </div>
      </div>

      <!-- Campo para el mensaje de commit -->
      <div class="git-commit-input-section">
        <label for="git-commit-message-input" class="git-commit-input-label">
          Mensaje del commit <span class="required">*</span>
        </label>
        <textarea
          id="git-commit-message-input"
          bind:this={textareaEl}
          bind:value={message}
          class="git-commit-textarea"
          rows="3"
          placeholder="Escribe un mensaje descriptivo para el commit..."
          disabled={loading}
        ></textarea>
        <div class="git-commit-hint">
          <span>Presiona <strong>Ctrl+Enter</strong> para confirmar directamente</span>
        </div>
      </div>

      <!-- Mensaje de error (si ocurre alguno) -->
      {#if errorMessage}
        <div class="git-commit-error-box" role="alert">
          <AlertCircle size={15} class="git-commit-error-icon" />
          <span class="git-commit-error-text">{errorMessage}</span>
        </div>
      {/if}

</ResizableDialog>

<style>
  .git-commit-branch-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 7px;
    font-size: 11px;
    font-weight: 500;
    border-radius: 12px;
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    color: var(--text-secondary, #656d76);
  }

  /* Cuadro de archivos */
  .git-commit-files-box {
    background-color: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    /* La lista de archivos ocupa el alto que sobra al agrandar el diálogo */
    flex: 1;
    min-height: 0;
  }

  .git-commit-files-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .git-commit-files-title {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-secondary, #656d76);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .git-commit-toggle-all-btn {
    background: none;
    border: none;
    padding: 0;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--accent, #0969da);
    cursor: pointer;
    font-family: inherit;
    outline: none;
    transition: opacity 0.15s ease;
  }

  .git-commit-toggle-all-btn:hover:not(:disabled) {
    text-decoration: underline;
  }

  .git-commit-toggle-all-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Buscador de archivos */
  .git-commit-search-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 5px;
  }

  :global(.git-commit-search-icon) {
    color: var(--text-secondary, #656d76);
    flex-shrink: 0;
  }

  .git-commit-search-input {
    border: none;
    outline: none;
    background: transparent;
    width: 100%;
    font-size: 12px;
    color: var(--text-primary, #1f2328);
    font-family: inherit;
  }

  .git-commit-search-clear {
    border: none;
    background: none;
    padding: 0;
    cursor: pointer;
    font-size: 11px;
    color: var(--text-secondary, #656d76);
    line-height: 1;
  }

  .git-commit-files-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-right: 2px;
  }

  .git-commit-file-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    border-radius: 4px;
    color: var(--text-primary, #1f2328);
    cursor: pointer;
    user-select: none;
    transition: background 0.1s ease;
    border: 1px solid transparent;
  }

  .git-commit-file-item:hover {
    background: var(--bg-tertiary, #eaeef2);
  }

  .git-commit-file-item.checked {
    background: color-mix(in srgb, var(--accent) 6%, transparent);
    border-color: color-mix(in srgb, var(--accent) 15%, transparent);
  }

  .git-commit-checkbox {
    cursor: pointer;
    accent-color: var(--accent, #0969da);
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    margin: 0;
  }

  /* Badges de estado Git */
  .git-status-badge {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 1px 6px;
    font-size: 10px;
    font-weight: 600;
    border-radius: 10px;
    line-height: 1.3;
    flex-shrink: 0;
  }

  .git-status-badge.status-deleted {
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    color: var(--danger);
    border: 1px solid color-mix(in srgb, var(--danger) 28%, transparent);
  }

  .git-status-badge.status-modified {
    background: color-mix(in srgb, var(--warning) 12%, transparent);
    color: var(--warning);
    border: 1px solid color-mix(in srgb, var(--warning) 28%, transparent);
  }

  .git-status-badge.status-untracked {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent);
    border: 1px solid color-mix(in srgb, var(--accent) 28%, transparent);
  }

  .git-status-badge.status-added,
  .git-status-badge.status-staged {
    background: color-mix(in srgb, var(--success) 12%, transparent);
    color: var(--success);
    border: 1px solid color-mix(in srgb, var(--success) 28%, transparent);
  }

  .git-commit-file-path {
    display: flex;
    align-items: baseline;
    gap: 6px;
    overflow: hidden;
    min-width: 0;
    flex: 1;
  }

  .git-commit-file-name {
    font-family: monospace;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary, #1f2328);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .git-commit-file-name.strikethrough {
    text-decoration: line-through;
    color: var(--text-secondary, #656d76);
  }

  .git-commit-file-dir {
    font-family: monospace;
    font-size: 11px;
    color: var(--text-muted, #8c959f);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .git-commit-empty-files {
    font-size: 12px;
    color: var(--text-secondary, #656d76);
    font-style: italic;
    padding: 8px 4px;
    text-align: center;
  }

  /* Sección de entrada */
  .git-commit-input-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .git-commit-input-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary, #1f2328);
  }

  .git-commit-input-label .required {
    color: var(--danger);
  }

  .git-commit-textarea {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    padding: 8px 10px;
    font-family: inherit;
    font-size: 13px;
    color: var(--text-primary, #1f2328);
    line-height: 1.45;
    resize: vertical;
    min-height: 72px;
    max-height: 180px;
    outline: none;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .git-commit-textarea:focus {
    border-color: var(--accent, #0969da);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 18%, transparent);
  }

  .git-commit-textarea:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .git-commit-hint {
    display: flex;
    justify-content: flex-end;
    font-size: 11px;
    color: var(--text-secondary, #656d76);
  }

  .git-commit-hint strong {
    font-weight: 600;
  }

  /* Caja de error */
  .git-commit-error-box {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 12px;
    border-radius: 6px;
    background: color-mix(in srgb, var(--danger) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--danger) 35%, transparent);
    color: var(--danger);
    font-size: 12px;
    line-height: 1.4;
  }

  :global(.git-commit-error-icon) {
    flex-shrink: 0;
    margin-top: 1px;
  }

  .git-commit-error-text {
    word-break: break-word;
  }

  /* Botones de acción */
  .git-commit-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 14px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    border: 1px solid transparent;
    font-family: inherit;
    outline: none;
  }

  .git-commit-btn.cancel {
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-primary, #1f2328);
    border-color: var(--border-primary, #d0d7de);
  }

  .git-commit-btn.cancel:hover:not(:disabled) {
    background: var(--border-primary, #d0d7de);
  }

  .git-commit-btn.submit {
    background: var(--success);
    color: var(--on-accent);
    border-color: color-mix(in srgb, var(--text-primary) 15%, transparent);
  }

  .git-commit-btn.submit:hover:not(:disabled) {
    background: color-mix(in srgb, var(--success) 85%, black);
  }

  .git-commit-btn.submit:focus-visible {
    outline: 2px solid var(--success);
    outline-offset: 2px;
  }

  .git-commit-btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  :global(.spin-icon) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>
