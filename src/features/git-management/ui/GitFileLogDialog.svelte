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
    Bot,
  } from 'lucide-svelte';
  import {
    vaultRepository,
    type GitCommitLogItem,
  } from '@shared/repositories';
  import { buildFileLogGraph, type GraphRow } from '../lib/file-log-graph';

  export interface GitFileLogDialogProps {
    open?: boolean;
    filePath?: string | null;
    onViewRevision?: (filePath: string, commitHash: string) => void;
    onDiffRevision?: (filePath: string, commitHash: string) => void;
    /** Abre el parche del commit: el archivo en ese commit contra su commit padre. */
    onPatchRevision?: (filePath: string, commitHash: string) => void;
    /** Diff entre dos versiones elegidas del archivo (`from` la más antigua). */
    onCompareRevisions?: (filePath: string, from: string, to: string) => void;
    /** Prepara en el panel del agente la pregunta «qué cambió» entre dos versiones. */
    /** `message`: el del commit, cuando se pregunta por un solo commit (`from` = su padre). */
    onAskAgent?: (filePath: string, from: string, to: string, message?: string) => void;
    onClose?: () => void;
  }

  let {
    open = $bindable(false),
    filePath = null,
    onViewRevision,
    onDiffRevision,
    onPatchRevision,
    onCompareRevisions,
    onAskAgent,
    onClose,
  }: GitFileLogDialogProps = $props();

  // Modo «Diff entre versiones»: en vez de las acciones, casillas para elegir exactamente 2 commits
  let compareMode = $state(false);
  let selected = $state<string[]>([]);

  function toggleCompareMode() {
    compareMode = !compareMode;
    selected = [];
  }

  function toggleSelected(hash: string) {
    if (selected.includes(hash)) selected = selected.filter((h) => h !== hash);
    else if (selected.length < 2) selected = [...selected, hash];
  }

  function handleCompare() {
    if (!filePath || selected.length !== 2) return;
    // De la versión más antigua a la más nueva (el log viene de la más nueva a la más antigua)
    const [to, from] = commits.filter((c) => selected.includes(c.commit_hash)).map((c) => c.commit_hash);
    open = false;
    onCompareRevisions?.(filePath, from, to);
  }

  let loading = $state(false);
  let error = $state<string | null>(null);
  let commits = $state<GitCommitLogItem[]>([]);
  let filterText = $state('');
  let copiedHash = $state<string | null>(null);

  // Selección de commits en la tabla: clic, Ctrl+clic (sumar/quitar) y Shift+clic (rango)
  let picked = $state<string[]>([]);
  let anchor: string | null = null;
  let rowMenu = $state<{ x: number; y: number } | null>(null);
  let copiedNotice = $state<string | null>(null);

  function handleRowClick(e: MouseEvent, hash: string) {
    if ((e.target as HTMLElement).closest('button, input, a')) return;
    if (e.ctrlKey || e.metaKey) {
      picked = picked.includes(hash) ? picked.filter((h) => h !== hash) : [...picked, hash];
      anchor = hash;
    } else if (e.shiftKey && anchor) {
      const hashes = filteredCommits.map((c) => c.commit_hash);
      const a = hashes.indexOf(anchor);
      const b = hashes.indexOf(hash);
      if (a !== -1 && b !== -1) picked = hashes.slice(Math.min(a, b), Math.max(a, b) + 1);
    } else {
      picked = [hash];
      anchor = hash;
    }
  }

  function handleRowContextMenu(e: MouseEvent, hash: string) {
    e.preventDefault();
    if (!picked.includes(hash)) {
      picked = [hash];
      anchor = hash;
    }
    // El diálogo está centrado con `transform`: el menú se ubica relativo a él
    const dialogEl = (e.currentTarget as HTMLElement).closest('.git-log-dialog') as HTMLElement | null;
    const r = dialogEl?.getBoundingClientRect() ?? { left: 0, top: 0 };
    rowMenu = { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  /** Commits elegidos en el orden de la lista (del más nuevo al más antiguo). */
  const pickedInOrder = $derived(commits.filter((c) => picked.includes(c.commit_hash)));

  async function copyPicked(short: boolean) {
    const text = pickedInOrder.map((c) => (short ? c.short_hash : c.commit_hash)).join(',');
    rowMenu = null;
    try {
      await navigator.clipboard.writeText(text);
      copiedNotice = pickedInOrder.length === 1 ? 'Hash copiado' : `${pickedInOrder.length} hashes copiados`;
      setTimeout(() => (copiedNotice = null), 2000);
    } catch (err) {
      console.error('Error al copiar hashes:', err);
    }
  }

  /** Con 2 commits: qué cambió entre ellos. Con 1: qué cambió en ese commit (contra su padre). */
  function askAgentPicked() {
    if (!filePath || pickedInOrder.length < 1 || pickedInOrder.length > 2) return;
    rowMenu = null;
    open = false;
    if (pickedInOrder.length === 1) {
      const c = pickedInOrder[0];
      onAskAgent?.(filePath, `${c.commit_hash}^`, c.commit_hash, c.message);
      return;
    }
    const [to, from] = pickedInOrder.map((c) => c.commit_hash);
    onAskAgent?.(filePath, from, to);
  }

  function comparePicked() {
    if (!filePath || pickedInOrder.length !== 2) return;
    // De la versión más antigua a la más nueva
    const [to, from] = pickedInOrder.map((c) => c.commit_hash);
    rowMenu = null;
    open = false;
    onCompareRevisions?.(filePath, from, to);
  }

  $effect(() => {
    if (!rowMenu) return;
    const close = (e: PointerEvent) => {
      if (!(e.target as HTMLElement).closest('.git-log-row-menu')) rowMenu = null;
    };
    window.addEventListener('pointerdown', close, true);
    return () => window.removeEventListener('pointerdown', close, true);
  });



  const fileName = $derived(filePath ? filePath.split('/').pop() || filePath : '');

  // Grafo de ramas: solo con la lista completa (al filtrar, las líneas no tendrían sentido)
  const GRAPH_LANE = 14;
  const GRAPH_COLORS = ['#378add', '#ba7517', '#1d9e75', '#d4537e', '#7f77dd', '#d85a30'];
  const graphColor = (i: number) => GRAPH_COLORS[i % GRAPH_COLORS.length];
  const graph = $derived.by(() => {
    const rows = buildFileLogGraph(commits);
    return new Map<string, GraphRow>(commits.map((c, i) => [c.commit_hash, rows[i]]));
  });
  const showGraph = $derived(!filterText.trim() && [...graph.values()].some((r) => r.width > 1 || r.segments.length > 0));
  const graphWidth = $derived(Math.max(1, ...[...graph.values()].map((r) => r.width)) * GRAPH_LANE);
  const laneX = (j: number) => j * GRAPH_LANE + GRAPH_LANE / 2;

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
      compareMode = false;
      selected = [];
      picked = [];
      anchor = null;
      rowMenu = null;
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
    if (e.key === 'Escape' && rowMenu) {
      e.preventDefault();
      rowMenu = null;
      return;
    }
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
    {#if compareMode}
      <span class="git-log-compare-info" aria-live="polite">
        {selected.length} de 2 versiones elegidas{selected.length < 2 ? ' — marca dos para comparar' : ''}
      </span>
      <button
        type="button"
        class="compare-go-btn"
        disabled={selected.length !== 2}
        onclick={handleCompare}
        title="Abrir el diff entre las dos versiones elegidas (de la más antigua a la más nueva)"
      >
        <GitCompare size={13} />
        Comparar
      </button>
    {/if}
    {#if copiedNotice}
      <span class="git-log-copied" aria-live="polite"><Check size={12} /> {copiedNotice}</span>
    {:else if !compareMode && picked.length > 1}
      <span class="git-log-picked-info">{picked.length} seleccionados · clic derecho para comparar o copiar</span>
    {/if}
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

      {#if rowMenu}
        <div class="git-log-row-menu" role="menu" style="left: {rowMenu.x}px; top: {rowMenu.y}px;">
          {#if onCompareRevisions}
            <button
              type="button"
              role="menuitem"
              disabled={pickedInOrder.length !== 2}
              title={pickedInOrder.length === 2 ? 'Abrir el diff de la versión más antigua a la más nueva' : 'Elige exactamente 2 versiones (Ctrl+clic)'}
              onclick={comparePicked}
            >
              <GitCompare size={13} /> Diff entre estas 2 versiones
            </button>
          {/if}
          {#if onAskAgent}
            <button
              type="button"
              role="menuitem"
              disabled={pickedInOrder.length > 2}
              title={pickedInOrder.length <= 2
                ? 'Abre el panel del agente con el archivo adjunto y la pregunta escrita'
                : 'Elige 1 commit (qué cambió en él) o 2 (qué cambió entre ellos)'}
              onclick={askAgentPicked}
            >
              <Bot size={13} />
              {pickedInOrder.length === 1 ? 'Preguntar al agente qué cambió en este commit' : 'Preguntar al agente qué cambió'}
            </button>
          {/if}
          <button type="button" role="menuitem" onclick={() => copyPicked(false)}>
            <Copy size={13} />
            {pickedInOrder.length === 1 ? 'Copiar hash' : `Copiar ${pickedInOrder.length} hashes (separados por coma)`}
          </button>
          <button type="button" role="menuitem" onclick={() => copyPicked(true)}>
            <Copy size={13} />
            {pickedInOrder.length === 1 ? 'Copiar hash corto' : `Copiar ${pickedInOrder.length} hashes cortos`}
          </button>
        </div>
      {/if}

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
        {#if onCompareRevisions && commits.length >= 2}
          <button
            type="button"
            class="compare-toggle"
            class:active={compareMode}
            aria-pressed={compareMode}
            onclick={toggleCompareMode}
            title={compareMode ? 'Volver a las acciones de cada commit' : 'Elegir dos versiones del archivo y ver sus diferencias'}
          >
            <GitCompare size={13} />
            Diff entre versiones
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
                  {#if showGraph}<th class="col-graph" aria-label="Ramas"></th>{/if}
                  <th class="col-hash">Hash</th>
                  <th class="col-date">Fecha</th>
                  <th class="col-author">Autor</th>
                  <th class="col-message">Mensaje</th>
                  {#if compareMode}
                    <th class="col-select">Comparar</th>
                  {:else}
                    <th class="col-actions">Acciones</th>
                  {/if}
                </tr>
              </thead>
              <tbody>
                {#each filteredCommits as commit (commit.commit_hash)}
                  <tr
                    class="git-log-row"
                    class:selected={compareMode && selected.includes(commit.commit_hash)}
                    class:picked={!compareMode && picked.includes(commit.commit_hash)}
                    data-hash={commit.commit_hash}
                    onclick={(e) => handleRowClick(e, commit.commit_hash)}
                    oncontextmenu={(e) => handleRowContextMenu(e, commit.commit_hash)}
                  >
                    {#if showGraph}
                      {@const g = graph.get(commit.commit_hash)}
                      <td class="col-graph" style="width: {graphWidth}px; min-width: {graphWidth}px">
                        {#if g}
                          <svg class="graph-lines" viewBox="0 0 {graphWidth} 100" preserveAspectRatio="none" aria-hidden="true">
                            {#each g.segments as seg, k (k)}
                              <line
                                x1={laneX(seg.from)}
                                y1={seg.half === 'top' ? 0 : 50}
                                x2={laneX(seg.to)}
                                y2={seg.half === 'top' ? 50 : 100}
                                stroke={graphColor(seg.color)}
                                stroke-width="2"
                                vector-effect="non-scaling-stroke"
                              />
                            {/each}
                          </svg>
                          <span
                            class="graph-dot"
                            class:origin={commit.change === 'A'}
                            style="left: {laneX(g.col)}px; --dot: {graphColor(g.color)}"
                          ></span>
                        {/if}
                      </td>
                    {/if}
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
                          >{/each}{#if commit.change === 'A'}<span class="change-badge added" title="Este commit creó el archivo: no hay versión anterior en su rama">nuevo</span
                          >{:else if commit.change === 'D'}<span class="change-badge deleted" title="Este commit borró el archivo">borrado</span
                          >{:else if commit.change === 'R'}<span class="change-badge renamed" title="Antes: {commit.previous_path}">renombrado</span
                          >{:else if (commit.parents?.length ?? 0) > 1}<span class="change-badge merge" title="Fusión de ramas que tocaron el archivo">fusión</span
                          >{/if}{#if showGraph && (graph.get(commit.commit_hash)?.col ?? 0) > 0}<span class="change-badge branch" title="Viene de otra rama: no es la versión anterior del commit de arriba">otra rama</span
                          >{/if}{commit.message}</span
                      >
                    </td>

                    {#if compareMode}
                      <!-- Columna de selección (modo «Diff entre versiones») -->
                      <td class="col-select">
                        <input
                          type="checkbox"
                          class="compare-check"
                          aria-label="Comparar la versión {commit.short_hash}"
                          checked={selected.includes(commit.commit_hash)}
                          disabled={!selected.includes(commit.commit_hash) && selected.length >= 2}
                          onchange={() => toggleSelected(commit.commit_hash)}
                        />
                      </td>
                    {:else}
                    <!-- Columna Acciones -->
                    <td class="col-actions">
                      <div class="actions-group">
                        {#if commit.change !== 'D'}
                        <button
                          type="button"
                          class="action-btn view-btn"
                          onclick={() => handleView(commit.commit_hash)}
                          title="Ver archivo en memoria en esta revisión histórica"
                        >
                          <Eye size={13} />
                          <span>Ver archivo</span>
                        </button>
                        {/if}
                        <div class="diff-stack">
                        {#if onPatchRevision && commit.change !== 'A'}
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
                    {/if}
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

  .git-log-table th.col-graph,
  .git-log-table td.col-graph {
    position: relative;
    padding: 0;
  }
  .graph-lines {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: calc(100% + 1px);
    overflow: visible;
  }
  .graph-dot {
    position: absolute;
    top: 50%;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--dot);
    transform: translate(-50%, -50%);
  }
  /* Origen del archivo en su rama: punto hueco */
  .graph-dot.origin {
    width: 11px;
    height: 11px;
    background: var(--bg-primary, #fff);
    border: 2.5px solid var(--dot);
    box-sizing: border-box;
  }
  .change-badge {
    display: inline-block;
    margin-right: 6px;
    padding: 0 7px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
    line-height: 16px;
    vertical-align: 1px;
  }
  .change-badge.added { color: #1a7f37; background: rgba(26, 127, 55, 0.12); }
  .change-badge.deleted { color: #cf222e; background: rgba(207, 34, 46, 0.12); }
  .change-badge.renamed, .change-badge.merge { color: #6639ba; background: rgba(102, 57, 186, 0.12); }
  .change-badge.branch { color: #9a6700; background: rgba(154, 103, 0, 0.14); }

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
  .compare-toggle {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-left: 8px;
    flex-shrink: 0;
    padding: 5px 10px;
    border-radius: 6px;
    border: 1px solid var(--border-primary, #d0d7de);
    background: var(--bg-primary, #fff);
    color: var(--text-secondary, #656d76);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
  }
  .compare-toggle:hover {
    color: var(--text-primary, #1f2328);
    border-color: var(--accent, #0969da);
  }
  .compare-toggle.active {
    color: var(--accent, #0969da);
    border-color: var(--accent, #0969da);
    background: color-mix(in srgb, var(--accent, #0969da) 10%, transparent);
  }
  .col-select {
    width: 90px;
    text-align: center;
  }
  .compare-check {
    width: 16px;
    height: 16px;
    cursor: pointer;
    accent-color: var(--accent, #0969da);
  }
  .compare-check:disabled {
    cursor: not-allowed;
    opacity: 0.4;
  }
  .git-log-row.selected {
    background: color-mix(in srgb, var(--accent, #0969da) 8%, transparent);
  }
  .git-log-compare-info {
    font-size: 12px;
    color: var(--text-secondary, #656d76);
    margin-right: auto;
  }
  .compare-go-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 6px 14px;
    border-radius: 6px;
    border: 1px solid var(--accent, #0969da);
    background: var(--accent, #0969da);
    color: var(--accent-contrast, #fff);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .compare-go-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .git-log-row {
    cursor: default;
  }
  .git-log-row.picked {
    background: color-mix(in srgb, var(--accent, #0969da) 12%, transparent);
  }
  .git-log-row-menu {
    position: absolute;
    z-index: 50;
    display: flex;
    flex-direction: column;
    min-width: 240px;
    padding: 4px;
    border-radius: 8px;
    border: 1px solid var(--border-primary, #d0d7de);
    background: var(--bg-primary, #fff);
    box-shadow: var(--modal-shadow, 0 8px 24px rgba(0, 0, 0, 0.2));
  }
  .git-log-row-menu button {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border: none;
    border-radius: 5px;
    background: none;
    color: var(--text-primary, #1f2328);
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .git-log-row-menu button:hover:not(:disabled) {
    background: var(--bg-tertiary, #eaeef2);
  }
  .git-log-row-menu button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .git-log-picked-info,
  .git-log-copied {
    font-size: 12px;
    color: var(--text-secondary, #656d76);
    margin-right: auto;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
</style>
