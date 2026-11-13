<script lang="ts">
  import { ResizableDialog } from '@shared/ui/dialog-resize';
  import {
    Tag,
    History,
    Search,
    Loader2,
    AlertCircle,
    Eye,
    GitCompare,
    Copy,
    Check,
    X,
    FileText,
    GitCommitHorizontal,
  } from 'lucide-svelte';
  import {
    vaultRepository,
    type GitCommitLogItem,
  } from '@shared/repositories';

  export interface GitFileLogDialogProps {
    open?: boolean;
    filePath?: string | null;
    onViewRevision?: (filePath: string, commitHash: string) => void;
    onDiffRevision?: (filePath: string, commitHash: string) => void;
    /** Abre el parche del commit: el archivo en ese commit contra su commit padre. */
    onPatchRevision?: (filePath: string, commitHash: string) => void;
    onClose?: () => void;
  }

  let {
    open = $bindable(false),
    filePath = null,
    onViewRevision,
    onDiffRevision,
    onPatchRevision,
    onClose,
  }: GitFileLogDialogProps = $props();

  let loading = $state(false);
  let error = $state<string | null>(null);
  let commits = $state<GitCommitLogItem[]>([]);
  let filterText = $state('');
  let copiedHash = $state<string | null>(null);


  const fileName = $derived(filePath ? filePath.split('/').pop() || filePath : '');

  const filteredCommits = $derived.by(() => {
    const q = filterText.trim().toLowerCase();
    if (!q) return commits;
    return commits.filter(
      (c) =>
        c.message.toLowerCase().includes(q) ||
        c.author_name.toLowerCase().includes(q) ||
        c.short_hash.toLowerCase().includes(q) ||
        c.commit_hash.toLowerCase().includes(q)
    );
  });

  async function loadLog() {
    if (!filePath) {
      commits = [];
      return;
    }
    loading = true;
    error = null;
    try {
      const res = await vaultRepository.getGitFileLog(filePath, 100);
      commits = res.commits || [];
    } catch (err: unknown) {
      console.error('Error cargando historial de archivo:', err);
      error = err instanceof Error ? err.message : String(err);
      commits = [];
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (open && filePath) {
      filterText = '';
      loadLog();
    }
  });

  function handleClose() {
    open = false;
    onClose?.();
  }

  function handleView(commitHash: string) {
    if (!filePath) return;
    open = false;
    onViewRevision?.(filePath, commitHash);
  }

  function handlePatch(commitHash: string) {
    if (!filePath) return;
    open = false;
    onPatchRevision?.(filePath, commitHash);
  }

  function handleDiff(commitHash: string) {
    if (!filePath) return;
    open = false;
    onDiffRevision?.(filePath, commitHash);
  }

  async function copyCommitHash(hash: string) {
    try {
      await navigator.clipboard.writeText(hash);
      copiedHash = hash;
      setTimeout(() => {
        if (copiedHash === hash) copiedHash = null;
      }, 2000);
    } catch (err) {
      console.error('Error al copiar hash:', err);
    }
  }

  function formatTime(isoStr: string, timestamp?: number): string {
    try {
      // La fecha ISO o, si no llega, el timestamp UNIX (segundos)
      let d = new Date(isoStr);
      if (Number.isNaN(d.getTime()) && timestamp) d = new Date(timestamp * 1000);
      if (Number.isNaN(d.getTime())) return isoStr || '—';
      return d.toLocaleDateString(undefined, {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
      });
    } catch {
      return isoStr;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      handleClose();
    }
  }
</script>

<ResizableDialog
  {open}
  onClose={handleClose}
  title="Historial de cambios (Git Log)"
  description="Visualiza las revisiones históricas del archivo y compara diferencias contra la versión local."
  sizeKey="synapse.gitLogDialog.size"
  defaultSize={{ w: 1200, h: 680 }}
  minSize={{ w: 560, h: 320 }}
  class="git-log-dialog"
  bodyPadding="0"
  onkeydown={handleKeyDown}
>
  {#snippet icon()}<History size={20} />{/snippet}
  {#snippet badge()}
    <span class="git-log-file-badge" title={filePath || ''}>
      <FileText size={11} />
      <span>{fileName}</span>
    </span>
  {/snippet}
  {#snippet footer()}
    <span class="git-log-count-info">
      {filteredCommits.length} {filteredCommits.length === 1 ? 'revisión' : 'revisiones'}
      {#if filterText && filteredCommits.length !== commits.length}
        (de {commits.length})
      {/if}
    </span>
    <button type="button" class="close-btn" onclick={handleClose}>
      Cerrar
    </button>
  {/snippet}

      <!-- Barra de Filtro / Búsqueda -->
      <div class="git-log-search-bar">
        <Search size={14} class="git-log-search-icon" />
        <input
          type="text"
          bind:value={filterText}
          placeholder="Buscar por mensaje, hash o autor..."
          class="git-log-search-input"
        />
        {#if filterText}
          <button
            type="button"
            class="git-log-search-clear"
            onclick={() => (filterText = '')}
            title="Limpiar búsqueda"
          >
            <X size={12} />
          </button>
        {/if}
      </div>

      <!-- Cuerpo del Historial -->
      <div class="git-log-body">
        {#if loading}
          <div class="git-log-state-box">
            <Loader2 size={24} class="spinner" />
            <p>Consultando historial de Git...</p>
          </div>
        {:else if error}
          <div class="git-log-state-box error">
            <AlertCircle size={28} />
            <p class="error-msg">{error}</p>
            <button type="button" class="retry-btn" onclick={loadLog}>
              Reintentar
            </button>
          </div>
        {:else if commits.length === 0}
          <div class="git-log-state-box empty">
            <History size={32} />
            <p>No se encontraron commits en el historial para este archivo.</p>
          </div>
        {:else if filteredCommits.length === 0}
          <div class="git-log-state-box empty">
            <Search size={32} />
            <p>Ningún commit coincide con "<strong>{filterText}</strong>".</p>
          </div>
        {:else}
          <div class="git-log-table-container">
            <table class="git-log-table">
              <thead>
                <tr>
                  <th class="col-hash">Hash</th>
                  <th class="col-date">Fecha</th>
                  <th class="col-author">Autor</th>
                  <th class="col-message">Mensaje</th>
                  <th class="col-actions">Acciones</th>
                </tr>
              </thead>
              <tbody>
                {#each filteredCommits as commit (commit.commit_hash)}
                  <tr class="git-log-row">
                    <!-- Columna Hash -->
                    <td class="col-hash">
                      <div class="hash-wrap">
                        <span class="commit-short-hash" title={commit.commit_hash}>
                          {commit.short_hash}
                        </span>
                        <button
                          type="button"
                          class="copy-hash-btn"
                          onclick={() => copyCommitHash(commit.commit_hash)}
                          title="Copiar hash completo"
                        >
                          {#if copiedHash === commit.commit_hash}
                            <Check size={12} class="copy-success" />
                          {:else}
                            <Copy size={12} />
                          {/if}
                        </button>
                      </div>
                    </td>

                    <!-- Columna Fecha -->
                    <td class="col-date" title={commit.date}>
                      <span class="commit-date">{formatTime(commit.date, commit.timestamp)}</span>
                    </td>

                    <!-- Columna Autor -->
                    <td class="col-author" title="{commit.author_name} <{commit.author_email}>">
                      <span class="commit-author">{commit.author_name}</span>
                    </td>

                    <!-- Columna Mensaje -->
                    <td class="col-message" title={commit.message}>
                      <span class="commit-message-text"
                        >{#each commit.tags ?? [] as tag (tag)}<span class="commit-tag" title="Tag {tag}"
                            ><Tag size={10} />{tag}</span
                          >{/each}{commit.message}</span
                      >
                    </td>

                    <!-- Columna Acciones -->
                    <td class="col-actions">
                      <div class="actions-group">
                        <button
                          type="button"
                          class="action-btn view-btn"
                          onclick={() => handleView(commit.commit_hash)}
                          title="Ver archivo en memoria en esta revisión histórica"
                        >
                          <Eye size={13} />
                          <span>Ver archivo</span>
                        </button>
                        <div class="diff-stack">
                        {#if onPatchRevision}
                          <button
                            type="button"
                            class="action-btn diff-btn patch-btn"
                            onclick={() => handlePatch(commit.commit_hash)}
                            title="Ver qué cambió este commit en el archivo (contra su commit padre)"
                          >
                            <GitCommitHorizontal size={13} />
                            <span>Ver cambios</span>
                          </button>
                        {/if}
                        <button
                          type="button"
                          class="action-btn diff-btn"
                          onclick={() => handleDiff(commit.commit_hash)}
                          title="Comparar esta revisión con el archivo actual en local"
                        >
                          <GitCompare size={13} />
                          <span>Diff con local</span>
                        </button>
                        </div>
                      </div>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>

</ResizableDialog>

<style>
  .git-log-file-badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 7px;
    font-size: 11.5px;
    font-family: monospace;
    font-weight: 500;
    color: var(--text-secondary, #656d76);
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 4px;
    max-width: 320px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-log-search-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    border-bottom: 1px solid var(--border-secondary, #e1e4e8);
    background: var(--bg-primary, #ffffff);
  }

  :global(.git-log-search-icon) {
    color: var(--text-secondary, #656d76);
    flex-shrink: 0;
  }

  .git-log-search-input {
    flex: 1;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 12.5px;
    font-family: inherit;
    color: var(--text-primary, #1f2328);
    background: var(--bg-secondary, #f6f8fa);
    outline: none;
    transition: all 0.15s ease;
  }

  .git-log-search-input:focus {
    border-color: var(--accent, #0969da);
    background: var(--bg-primary, #ffffff);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 15%, transparent);
  }

  .git-log-search-clear {
    border: none;
    background: none;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    padding: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
  }

  .git-log-search-clear:hover {
    background: var(--bg-tertiary, #eaeef2);
    color: var(--text-primary, #1f2328);
  }

  .git-log-body {
    flex: 1;
    overflow-y: auto;
    position: relative;
    background: var(--bg-primary, #ffffff);
  }

  .git-log-state-box {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 240px;
    gap: 12px;
    color: var(--text-secondary, #656d76);
    font-size: 13px;
    text-align: center;
    padding: 24px;
  }

  .git-log-state-box.error {
    color: var(--danger);
  }

  .retry-btn {
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 500;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-primary, #1f2328);
    cursor: pointer;
  }

  .retry-btn:hover {
    background: var(--bg-tertiary, #eaeef2);
  }

  .git-log-table-container {
    width: 100%;
  }

  .git-log-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12.5px;
    text-align: left;
  }

  .git-log-table thead {
    position: sticky;
    top: 0;
    z-index: 2;
    background: var(--bg-secondary, #f6f8fa);
    box-shadow: 0 1px 0 var(--border-primary, #d0d7de);
  }

  .git-log-table th {
    padding: 8px 12px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: var(--text-secondary, #656d76);
  }

  .git-log-table td {
    padding: 9px 12px;
    border-bottom: 1px solid var(--border-secondary, #f0f2f5);
    vertical-align: middle;
  }

  .git-log-row:hover {
    background: color-mix(in srgb, var(--accent) 4%, transparent);
  }

  .col-hash {
    width: 95px;
    white-space: nowrap;
  }

  .hash-wrap {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .commit-short-hash {
    font-family: monospace;
    font-size: 12px;
    font-weight: 600;
    color: var(--accent, #0969da);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    padding: 2px 5px;
    border-radius: 4px;
  }

  .copy-hash-btn {
    border: none;
    background: none;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    padding: 2px;
    border-radius: 3px;
    display: flex;
    align-items: center;
    opacity: 0.6;
    transition: opacity 0.1s ease;
  }

  .copy-hash-btn:hover {
    opacity: 1;
    background: var(--bg-tertiary, #eaeef2);
  }

  :global(.copy-success) {
    color: var(--success);
  }

  .col-date {
    width: 140px;
    white-space: nowrap;
    color: var(--text-secondary, #656d76);
    font-size: 12px;
  }

  .col-author {
    width: 130px;
    max-width: 130px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-secondary, #656d76);
    font-weight: 500;
  }

  .col-message {
    min-width: 200px;
  }

  /* Hasta dos líneas de mensaje; el resto se ve en el tooltip */
  .commit-message-text {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    overflow-wrap: anywhere;
    line-height: 1.35;
    color: var(--text-primary, #1f2328);
    font-weight: 500;
  }

  .commit-tag {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    margin-right: 6px;
    padding: 0 6px;
    border-radius: 999px;
    font-family: monospace;
    font-size: 11px;
    font-weight: 600;
    line-height: 16px;
    vertical-align: 1px;
    color: var(--warning, #9a6700);
    background: color-mix(in srgb, var(--warning, #9a6700) 14%, transparent);
    border: 1px solid color-mix(in srgb, var(--warning, #9a6700) 45%, transparent);
  }

  .col-actions {
    width: 1%;
    text-align: right;
    white-space: nowrap;
  }

  .actions-group {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  /* Los dos diffs, uno sobre otro, para dejarle ancho al mensaje */
  .diff-stack {
    display: inline-flex;
    flex-direction: column;
    align-items: stretch;
    gap: 4px;
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 8px;
    font-size: 11.5px;
    font-weight: 500;
    font-family: inherit;
    border-radius: 5px;
    cursor: pointer;
    transition: all 0.15s ease;
    border: 1px solid var(--border-primary, #d0d7de);
  }

  .view-btn {
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-primary, #1f2328);
  }

  .view-btn:hover {
    background: var(--bg-tertiary, #eaeef2);
    border-color: var(--accent, #0969da);
    color: var(--accent, #0969da);
  }

  .diff-btn {
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    color: var(--accent, #0969da);
    border-color: color-mix(in srgb, var(--accent) 30%, transparent);
  }

  .diff-btn:hover {
    background: var(--accent, #0969da);
    color: var(--on-accent);
  }

  .git-log-count-info {
    margin-right: auto;
    font-size: 12px;
    color: var(--text-secondary, #656d76);
  }

  .close-btn {
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 500;
    font-family: inherit;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    background: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .close-btn:hover {
    background: var(--bg-tertiary, #eaeef2);
  }

  :global(.spinner) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
