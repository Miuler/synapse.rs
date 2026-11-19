<script lang="ts">
  // Log de todas las ramas del repositorio, al estilo de la ventana Git Log de JetBrains:
  // grafo de commits, mensaje con etiquetas de ramas y tags, autor y fecha; al elegir un commit,
  // abajo sus detalles y archivos cambiados (doble clic o Enter abre el diff del archivo).
  import { ResizableDialog } from '@shared/ui/dialog-resize';
  import {
    GitGraph, Search, Loader2, AlertCircle, Tag, GitBranch, Cloud, FileText, RefreshCw,
    GitBranchPlus, LogIn, Trash2, Copy, Crosshair,
  } from 'lucide-svelte';
  import {
    gitApi,
    gitDialogState,
    type GitRepoCommit,
    type GitCommitDetails,
    type GitCommitFileChange,
  } from '@entities/git';
  import { buildRepoLogGraph, reachableFrom } from '../lib/repo-log-graph';
  import type { GraphRow } from '../lib/file-log-graph';
  import GitRepoSelect from './GitRepoSelect.svelte';

  interface Props {
    open?: boolean;
    /** Abre el diff de un archivo (ruta relativa a la bóveda) en un commit contra su padre. */
    onOpenFileDiff?: (filePath: string, commitHash: string) => void;
    /** Tras un checkout o un cambio de ramas: refrescar el estado de git de la app. */
    onRepoChanged?: () => void | Promise<void>;
    onToast?: (message: string) => void;
    onClose?: () => void;
  }

  let { open = $bindable(false), onOpenFileDiff, onRepoChanged, onToast, onClose }: Props = $props();


  let commits = $state<GitRepoCommit[]>([]);
  /** Commit en el que está HEAD. */
  let head = $state<string | null>(null);
  /** Rama de HEAD; `null` = HEAD separado. */
  const headBranch = $derived(
    commits.find((c) => c.commit_hash === head)?.refs.find((r) => r.is_head)?.name ?? null
  );
  let loading = $state(false);
  let error = $state<string | null>(null);
  let filterText = $state('');
  /** `''` = todas las ramas; si no, el nombre de la ref (`main`, `origin/main`, `v1`). */
  let branchFilter = $state('');
  let selectedHash = $state<string | null>(null);
  let details = $state<GitCommitDetails | null>(null);
  let detailsLoading = $state(false);
  let detailsError = $state<string | null>(null);
  let loadedRepo: string | null = null;

  const repo = $derived(gitDialogState.repo);

  async function loadLog() {
    loading = true;
    error = null;
    const target = repo;
    try {
      const res = await gitApi.getGitRepoLog(target);
      if (target !== repo) return;
      commits = res.commits ?? [];
      head = res.head ?? null;
      loadedRepo = target;
      if (!selectedHash || !commits.some((c) => c.commit_hash === selectedHash)) {
        selectCommit(res.head ?? commits[0]?.commit_hash ?? null);
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
      commits = [];
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (open && repo !== loadedRepo) {
      branchFilter = '';
      selectedHash = null;
      details = null;
      void loadLog();
    }
    if (!open) loadedRepo = null;
  });

  let detailsToken = 0;
  async function selectCommit(hash: string | null) {
    selectedHash = hash;
    details = null;
    detailsError = null;
    if (!hash) return;
    const token = ++detailsToken;
    detailsLoading = true;
    try {
      const d = await gitApi.getGitCommitDetails(hash, repo);
      if (token === detailsToken) details = d;
    } catch (err) {
      if (token === detailsToken) detailsError = err instanceof Error ? err.message : String(err);
    } finally {
      if (token === detailsToken) detailsLoading = false;
    }
  }

  // Ramas y tags para el filtro
  const allRefs = $derived.by(() => {
    const out: { name: string; kind: string; tip: string }[] = [];
    for (const c of commits) for (const r of c.refs) out.push({ name: r.name, kind: r.kind, tip: c.commit_hash });
    const order = { local: 0, remote: 1, tag: 2 } as Record<string, number>;
    return out.sort((a, b) => order[a.kind] - order[b.kind] || a.name.localeCompare(b.name));
  });

  const branchCommits = $derived.by(() => {
    if (!branchFilter) return commits;
    const tip = allRefs.find((r) => r.name === branchFilter)?.tip;
    return tip ? reachableFrom(commits, tip) : commits;
  });

  const visibleCommits = $derived.by(() => {
    const q = filterText.trim().toLowerCase();
    if (!q) return branchCommits;
    return branchCommits.filter(
      (c) =>
        c.message.toLowerCase().includes(q) ||
        c.author_name.toLowerCase().includes(q) ||
        c.commit_hash.startsWith(q) ||
        c.refs.some((r) => r.name.toLowerCase().includes(q))
    );
  });

  // El grafo solo tiene sentido sin filtro de texto (con el de rama el conjunto sigue cerrado)
  const LANE = 14;
  const COLORS = ['#378add', '#ba7517', '#1d9e75', '#d4537e', '#7f77dd', '#d85a30', '#3b9fa0', '#9a8a20'];
  const color = (i: number) => COLORS[i % COLORS.length];
  const showGraph = $derived(!filterText.trim());
  const graph = $derived.by(() => {
    const rows = buildRepoLogGraph(branchCommits);
    return new Map<string, GraphRow>(branchCommits.map((c, i) => [c.commit_hash, rows[i]]));
  });
  const graphWidth = $derived(Math.max(1, ...[...graph.values()].map((r) => r.width)) * LANE);
  const laneX = (j: number) => j * LANE + LANE / 2;

  // ── Menú contextual de un commit ──────────────────────────────────────────
  let menu = $state<{ x: number; y: number; commit: GitRepoCommit } | null>(null);
  /** Pide un nombre (rama o tag) antes de ejecutar la acción. */
  let prompt = $state<{
    kind: 'branch' | 'tag';
    commit: GitRepoCommit;
    name: string;
    message: string;
    checkout: boolean;
  } | null>(null);
  /** Confirmación de una acción destructiva. */
  let confirmAction = $state<{ text: string; run: () => Promise<string> } | null>(null);
  let busy = $state(false);
  let notice = $state<{ text: string; error: boolean } | null>(null);

  function openMenu(e: MouseEvent, c: GitRepoCommit) {
    e.preventDefault();
    void selectCommit(c.commit_hash);
    menu = { x: e.clientX, y: e.clientY, commit: c };
  }

  $effect(() => {
    if (!menu) return;
    const close = (e: PointerEvent) => {
      if (!(e.target as HTMLElement).closest('.repo-log-menu')) menu = null;
    };
    window.addEventListener('pointerdown', close, true);
    return () => window.removeEventListener('pointerdown', close, true);
  });

  /** Ejecuta una acción de git, muestra el resultado y recarga el log. */
  async function run(action: () => Promise<string>, changesWorktree = false) {
    menu = null;
    busy = true;
    notice = null;
    try {
      const msg = await action();
      notice = { text: msg, error: false };
      onToast?.(msg);
      if (changesWorktree) await onRepoChanged?.();
      await loadLog();
    } catch (err) {
      notice = { text: err instanceof Error ? err.message : String(err), error: true };
    } finally {
      busy = false;
    }
  }

  function askName(kind: 'branch' | 'tag', commit: GitRepoCommit) {
    menu = null;
    prompt = { kind, commit, name: '', message: '', checkout: kind === 'branch' };
    queueMicrotask(() => (document.querySelector('.repo-log-prompt input') as HTMLInputElement | null)?.focus());
  }

  async function submitPrompt() {
    if (!prompt || !prompt.name.trim()) return;
    const p = prompt;
    prompt = null;
    if (p.kind === 'tag') {
      await run(() => gitApi.gitCreateTag({ name: p.name.trim(), commit: p.commit.commit_hash, message: p.message, repo }));
    } else {
      await run(
        () => gitApi.gitCreateBranch({ newBranch: p.name.trim(), baseBranch: p.commit.commit_hash, checkout: p.checkout, repo }),
        p.checkout
      );
    }
  }

  function askConfirm(text: string, action: () => Promise<string>) {
    menu = null;
    confirmAction = { text, run: action };
  }

  async function copyHash(hash: string) {
    menu = null;
    try {
      await navigator.clipboard.writeText(hash);
      notice = { text: `Hash copiado: ${hash.slice(0, 7)}`, error: false };
    } catch {
      notice = { text: 'No se pudo copiar el hash', error: true };
    }
  }

  function formatDate(iso: string): string {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return iso;
    return d.toLocaleString(undefined, { year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  }

  /** Ruta del archivo relativa a la bóveda (el log puede ser de un repositorio en una subcarpeta). */
  const vaultPath = (p: string) => (repo ? `${repo}/${p}` : p);

  function openFile(f: GitCommitFileChange) {
    if (!details) return;
    open = false;
    onOpenFileDiff?.(vaultPath(f.path), details.commit_hash);
  }

  const changeLabel: Record<string, string> = {
    A: 'Agregado',
    M: 'Modificado',
    D: 'Eliminado',
    R: 'Renombrado',
    C: 'Copiado',
    T: 'Cambio de tipo',
  };

  function handleClose() {
    open = false;
    onClose?.();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      if (menu) menu = null;
      else if (prompt) prompt = null;
      else if (confirmAction) confirmAction = null;
      else handleClose();
      return;
    }
    if (prompt || confirmAction) return;
    const target = e.target as HTMLElement;
    if (target.closest('input, select')) return;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const i = visibleCommits.findIndex((c) => c.commit_hash === selectedHash);
      const next = visibleCommits[Math.max(0, Math.min(visibleCommits.length - 1, i + (e.key === 'ArrowDown' ? 1 : -1)))];
      if (next && next.commit_hash !== selectedHash) {
        void selectCommit(next.commit_hash);
        queueMicrotask(() =>
          document.querySelector(`.repo-log-row[data-hash="${next.commit_hash}"]`)?.scrollIntoView({ block: 'nearest' })
        );
      }
    }
  }
</script>

<ResizableDialog
  {open}
  onClose={handleClose}
  title="Git Log"
  description="Commits de todas las ramas del repositorio, con el grafo de ramas."
  sizeKey="synapse.gitRepoLogDialog.size"
  defaultSize={{ w: 1200, h: 760 }}
  minSize={{ w: 640, h: 400 }}
  class="git-repo-log-dialog"
  bodyPadding="0"
  onkeydown={handleKeyDown}
>
  {#snippet icon()}<GitGraph size={20} />{/snippet}
  {#snippet footer()}
    {#if busy}
      <span class="repo-log-notice"><Loader2 size={12} class="spin" /> Ejecutando…</span>
    {:else if notice}
      <span class="repo-log-notice" class:error={notice.error} aria-live="polite">{notice.text}</span>
    {/if}
    <span class="repo-log-count">
      {visibleCommits.length} {visibleCommits.length === 1 ? 'commit' : 'commits'}
      {#if visibleCommits.length !== commits.length}(de {commits.length}){/if}
    </span>
    <button type="button" class="close-btn" onclick={handleClose}>Cerrar</button>
  {/snippet}

  <div class="repo-log">
    <div class="repo-log-toolbar">
      <GitRepoSelect disabled={loading} />
      <label class="repo-log-search">
        <Search size={13} />
        <input
          type="text"
          placeholder="Filtrar por mensaje, autor, hash o rama…"
          bind:value={filterText}
          aria-label="Filtrar commits"
        />
      </label>
      <select class="repo-log-branch" bind:value={branchFilter} aria-label="Rama">
        <option value="">Todas las ramas</option>
        {#each allRefs as r (r.kind + r.name)}
          <option value={r.name}>{r.kind === 'tag' ? 'Tag: ' : r.kind === 'remote' ? 'Remota: ' : ''}{r.name}</option>
        {/each}
      </select>
      <button type="button" class="repo-log-refresh" onclick={loadLog} title="Recargar" aria-label="Recargar" disabled={loading}>
        <RefreshCw size={13} />
      </button>
    </div>

    <div class="repo-log-list">
      {#if loading && commits.length === 0}
        <div class="repo-log-state"><Loader2 size={18} class="spin" /> Cargando log…</div>
      {:else if error}
        <div class="repo-log-state error"><AlertCircle size={18} /> {error}</div>
      {:else if visibleCommits.length === 0}
        <div class="repo-log-state">{commits.length ? 'Ningún commit coincide con el filtro.' : 'El repositorio no tiene commits.'}</div>
      {:else}
        <table class="repo-log-table">
          <thead>
            <tr>
              <th class="col-message">Mensaje</th>
              <th class="col-author">Autor</th>
              <th class="col-date">Fecha</th>
              <th class="col-hash">Hash</th>
            </tr>
          </thead>
          <tbody>
            {#each visibleCommits as c (c.commit_hash)}
              {@const g = graph.get(c.commit_hash)}
              <tr
                class="repo-log-row"
                class:selected={c.commit_hash === selectedHash}
                class:merge={c.parents.length > 1}
                class:head={c.commit_hash === head}
                data-hash={c.commit_hash}
                onclick={() => selectCommit(c.commit_hash)}
                oncontextmenu={(e) => openMenu(e, c)}
              >
                <td class="col-message">
                  <div class="message-cell" style="padding-left: {showGraph && g ? graphWidth + 4 : 0}px">
                    {#if showGraph && g}
                      <span class="graph" style="width: {graphWidth}px">
                        <svg viewBox="0 0 {graphWidth} 100" preserveAspectRatio="none" aria-hidden="true">
                          {#each g.segments as seg, k (k)}
                            <line
                              x1={laneX(seg.from)}
                              y1={seg.half === 'top' ? 0 : 50}
                              x2={laneX(seg.to)}
                              y2={seg.half === 'top' ? 50 : 100}
                              stroke={color(seg.color)}
                              stroke-width="2"
                              vector-effect="non-scaling-stroke"
                            />
                          {/each}
                        </svg>
                        <span
                          class="graph-dot"
                          class:head={c.commit_hash === head}
                          style="left: {laneX(g.col)}px; --dot: {color(g.color)}"
                        ></span>
                      </span>
                    {/if}
                    {#if c.commit_hash === head}
                      <span
                        class="head-label"
                        title={headBranch ? `Estás aquí: HEAD en la rama ${headBranch}` : 'Estás aquí: HEAD separado (sin rama)'}
                      >
                        HEAD{headBranch ? '' : ' (separado)'}
                      </span>
                    {/if}
                    {#each c.refs as r (r.kind + r.name)}
                      <span class="ref-label {r.kind}" class:head={r.is_head} title={r.is_head ? `HEAD → ${r.name}` : r.name}>
                        {#if r.kind === 'tag'}<Tag size={10} />{:else if r.kind === 'remote'}<Cloud size={10} />{:else}<GitBranch size={10} />{/if}
                        {r.name}
                      </span>
                    {/each}
                    <span class="message-text" title={c.message}>{c.message}</span>
                  </div>
                </td>
                <td class="col-author" title={c.author_email}>{c.author_name}</td>
                <td class="col-date">{formatDate(c.date)}</td>
                <td class="col-hash">{c.short_hash}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>

    <div class="repo-log-details" aria-live="polite">
      {#if detailsLoading}
        <div class="repo-log-state"><Loader2 size={16} class="spin" /> Cargando commit…</div>
      {:else if detailsError}
        <div class="repo-log-state error"><AlertCircle size={16} /> {detailsError}</div>
      {:else if details}
        <div class="details-files">
          <div class="details-title">Archivos cambiados ({details.files.length})</div>
          <ul>
            {#each details.files as f (f.path)}
              <li>
                <button
                  type="button"
                  class="details-file"
                  data-path={f.path}
                  title="Abrir el diff de {f.path} en este commit"
                  ondblclick={() => openFile(f)}
                  onkeydown={(e) => e.key === 'Enter' && openFile(f)}
                >
                  <span class="change-badge c-{f.change}" title={changeLabel[f.change] ?? f.change}>{f.change}</span>
                  <FileText size={12} />
                  <span class="file-name">{f.path.split('/').pop()}</span>
                  <span class="file-dir">{f.path.includes('/') ? f.path.slice(0, f.path.lastIndexOf('/')) : ''}</span>
                  {#if f.previous_path}<span class="file-dir">← {f.previous_path}</span>{/if}
                </button>
              </li>
            {/each}
          </ul>
        </div>
        <div class="details-commit">
          <pre class="details-message">{details.message}</pre>
          <div class="details-meta">
            <span class="hash">{details.commit_hash}</span>
            <span>{details.author_name} &lt;{details.author_email}&gt;</span>
            <span>{formatDate(details.date)}</span>
            {#if details.committer_name && details.committer_name !== details.author_name}
              <span>confirmado por {details.committer_name}</span>
            {/if}
            {#if details.parents.length > 1}<span>Fusión de {details.parents.map((p) => p.slice(0, 7)).join(' y ')}</span>{/if}
          </div>
        </div>
      {:else}
        <div class="repo-log-state">Elige un commit para ver sus detalles.</div>
      {/if}
    </div>
  </div>

  {#if menu}
    {@const c = menu.commit}
    {@const locals = c.refs.filter((r) => r.kind === 'local')}
    {@const tags = c.refs.filter((r) => r.kind === 'tag')}
    <div class="repo-log-menu" role="menu" style="left: {menu.x}px; top: {menu.y}px">
      <div class="menu-title">{c.short_hash} · {c.message}</div>
      {#each locals.filter((r) => r.name !== headBranch) as r (r.name)}
        <button type="button" role="menuitem" onclick={() => run(() => gitApi.gitCheckout(r.name, repo), true)}>
          <LogIn size={13} /> Checkout de «{r.name}»
        </button>
      {/each}
      {#if c.commit_hash !== head || headBranch}
        <button
          type="button"
          role="menuitem"
          data-action="checkout-commit"
          onclick={() =>
            askConfirm(
              `¿Hacer checkout de ${c.short_hash}? HEAD quedará separado (sin rama): los commits nuevos no pertenecerán a ninguna rama.`,
              () => gitApi.gitCheckout(c.commit_hash, repo)
            )}
        >
          <Crosshair size={13} /> Checkout de este commit (HEAD separado)
        </button>
      {/if}
      <button type="button" role="menuitem" data-action="new-branch" onclick={() => askName('branch', c)}>
        <GitBranchPlus size={13} /> Nueva rama aquí… (checkout -b)
      </button>
      <button type="button" role="menuitem" data-action="new-tag" onclick={() => askName('tag', c)}>
        <Tag size={13} /> Crear tag aquí…
      </button>
      {#if tags.length || locals.some((r) => r.name !== headBranch)}<div class="menu-sep"></div>{/if}
      {#each tags as r (r.name)}
        <button
          type="button"
          role="menuitem"
          class="danger"
          data-action="delete-tag"
          onclick={() => askConfirm(`¿Borrar el tag local «${r.name}»? (no se borra del remoto)`, () => gitApi.gitDeleteRef('tag', r.name, repo))}
        >
          <Trash2 size={13} /> Borrar tag «{r.name}»
        </button>
      {/each}
      {#each locals.filter((r) => r.name !== headBranch) as r (r.name)}
        <button
          type="button"
          role="menuitem"
          class="danger"
          data-action="delete-branch"
          onclick={() =>
            askConfirm(`¿Borrar la rama local «${r.name}»? (no se borra del remoto)`, async () => {
              try {
                return await gitApi.gitDeleteRef('branch', r.name, repo);
              } catch (err) {
                const msg = err instanceof Error ? err.message : String(err);
                if (!msg.includes('sin fusionar')) throw err;
                // Commits sin fusionar: segunda confirmación para forzar (git branch -D)
                confirmAction = {
                  text: `${msg}: si la borras, esos commits quedarán sin rama. ¿Borrarla igual (git branch -D)?`,
                  run: () => gitApi.gitDeleteRef('branch', r.name, repo, true),
                };
                return msg;
              }
            })}
        >
          <Trash2 size={13} /> Borrar rama «{r.name}»
        </button>
      {/each}
      <div class="menu-sep"></div>
      <button type="button" role="menuitem" onclick={() => copyHash(c.commit_hash)}>
        <Copy size={13} /> Copiar hash
      </button>
    </div>
  {/if}

  {#if prompt}
    <div class="repo-log-overlay">
      <form
        class="repo-log-prompt"
        onsubmit={(e) => {
          e.preventDefault();
          void submitPrompt();
        }}
      >
        <div class="prompt-title">
          {prompt.kind === 'tag' ? 'Crear tag' : 'Nueva rama'} en {prompt.commit.short_hash}
        </div>
        <input
          type="text"
          bind:value={prompt.name}
          placeholder={prompt.kind === 'tag' ? 'v1.2.0' : 'feature/nombre'}
          aria-label="Nombre"
        />
        {#if prompt.kind === 'tag'}
          <input type="text" bind:value={prompt.message} placeholder="Mensaje (opcional: tag anotado)" aria-label="Mensaje del tag" />
        {:else}
          <label class="prompt-check"><input type="checkbox" bind:checked={prompt.checkout} /> Cambiar a la rama nueva (checkout -b)</label>
        {/if}
        <div class="prompt-actions">
          <button type="button" onclick={() => (prompt = null)}>Cancelar</button>
          <button type="submit" class="primary" disabled={!prompt.name.trim()}>Crear</button>
        </div>
      </form>
    </div>
  {/if}

  {#if confirmAction}
    {@const ca = confirmAction}
    <div class="repo-log-overlay">
      <div class="repo-log-prompt" role="alertdialog">
        <div class="prompt-text">{ca.text}</div>
        <div class="prompt-actions">
          <button type="button" onclick={() => (confirmAction = null)}>Cancelar</button>
          <button
            type="button"
            class="primary"
            data-action="confirm"
            onclick={() => {
              // Leer la acción antes de cerrar: `ca` se recalcula al vaciar `confirmAction`
              const action = ca.run;
              confirmAction = null;
              void run(action, true);
            }}
          >
            Aceptar
          </button>
        </div>
      </div>
    </div>
  {/if}
</ResizableDialog>

<style>
  .repo-log {
    display: grid;
    grid-template-rows: auto minmax(120px, 1fr) minmax(140px, 38%);
    height: 100%;
    min-height: 0;
  }
  .repo-log-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border-secondary, #e1e4e8);
    flex-wrap: wrap;
  }
  .repo-log-search {
    flex: 1;
    min-width: 180px;
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    padding: 4px 8px;
    background: var(--bg-secondary);
  }
  .repo-log-search input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font: inherit;
    font-size: 12.5px;
  }
  .repo-log-branch,
  .repo-log-refresh {
    font-size: 12px;
    padding: 4px 6px;
    border-radius: 6px;
    border: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    color: var(--text-primary);
  }
  .repo-log-branch {
    max-width: 220px;
  }
  .repo-log-refresh {
    display: inline-flex;
    cursor: pointer;
  }
  .repo-log-list {
    overflow: auto;
    min-height: 0;
  }
  .repo-log-table {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
    font-size: 12.5px;
  }
  .repo-log-table th {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--bg-secondary, #f6f8fa);
    text-align: left;
    font-weight: 600;
    font-size: 11.5px;
    color: var(--text-secondary);
    padding: 5px 10px;
    border-bottom: 1px solid var(--border-secondary);
  }
  .repo-log-table td {
    padding: 0 10px;
    height: 24px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-secondary);
  }
  .repo-log-table td.col-message {
    position: relative;
    padding: 0 10px 0 4px;
    color: var(--text-primary);
  }
  .col-author {
    width: 150px;
  }
  .col-date {
    width: 160px;
  }
  .col-hash {
    width: 80px;
    font-family: monospace;
  }
  .repo-log-row {
    cursor: default;
  }
  .repo-log-row:hover {
    background: var(--bg-hover, rgba(127, 127, 127, 0.08));
  }
  .repo-log-row.selected {
    background: color-mix(in srgb, var(--accent, #0969da) 18%, transparent);
  }
  .repo-log-row.merge .message-text {
    color: var(--text-secondary);
  }
  .message-cell {
    display: flex;
    align-items: center;
    gap: 5px;
    min-height: 24px;
    min-width: 0;
  }
  /* El grafo ocupa toda la altura de la celda (la fila puede ser más alta que 24 px) */
  .graph {
    position: absolute;
    left: 4px;
    top: 0;
    bottom: 0;
  }
  .graph svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: calc(100% + 1px);
    overflow: visible;
  }
  .graph-dot {
    position: absolute;
    top: 50%;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--dot);
    transform: translate(-50%, -50%);
  }
  .message-text {
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .ref-label {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    flex-shrink: 0;
    padding: 0 5px;
    border-radius: 4px;
    font-size: 11px;
    line-height: 16px;
    border: 1px solid;
  }
  .ref-label.local {
    color: #1a7f37;
    border-color: color-mix(in srgb, #1a7f37 45%, transparent);
    background: color-mix(in srgb, #1a7f37 10%, transparent);
  }
  .ref-label.remote {
    color: #8250df;
    border-color: color-mix(in srgb, #8250df 45%, transparent);
    background: color-mix(in srgb, #8250df 10%, transparent);
  }
  .ref-label.tag {
    color: #9a6700;
    border-color: color-mix(in srgb, #9a6700 45%, transparent);
    background: color-mix(in srgb, #9a6700 10%, transparent);
  }
  .ref-label.head {
    font-weight: 700;
  }
  .repo-log-details {
    display: grid;
    grid-template-columns: minmax(200px, 1fr) minmax(200px, 1fr);
    border-top: 1px solid var(--border-primary);
    min-height: 0;
    overflow: hidden;
  }
  .details-files,
  .details-commit {
    overflow: auto;
    padding: 8px 12px;
    min-height: 0;
  }
  .details-files {
    border-right: 1px solid var(--border-secondary);
  }
  .details-title {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }
  .details-files ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .details-file {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    border: none;
    background: none;
    padding: 2px 4px;
    border-radius: 4px;
    font: inherit;
    font-size: 12.5px;
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
  }
  .details-file:hover,
  .details-file:focus-visible {
    background: var(--bg-hover, rgba(127, 127, 127, 0.1));
  }
  .file-dir {
    color: var(--text-secondary);
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .change-badge {
    font-family: monospace;
    font-size: 10.5px;
    font-weight: 700;
    width: 14px;
    text-align: center;
  }
  .c-A {
    color: #1a7f37;
  }
  .c-M {
    color: #0969da;
  }
  .c-D {
    color: #cf222e;
  }
  .c-R,
  .c-C {
    color: #8250df;
  }
  .details-message {
    margin: 0 0 8px;
    white-space: pre-wrap;
    font-family: inherit;
    font-size: 13px;
    color: var(--text-primary);
  }
  .details-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 11.5px;
    color: var(--text-secondary);
  }
  .details-meta .hash {
    font-family: monospace;
  }
  .repo-log-state {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px;
    color: var(--text-secondary);
    font-size: 12.5px;
    grid-column: 1 / -1;
  }
  .repo-log-state.error {
    color: var(--danger, #cf222e);
  }
  .repo-log-row.head .message-text {
    font-weight: 700;
  }
  .graph-dot.head {
    width: 12px;
    height: 12px;
    background: var(--bg-primary, #fff);
    border: 3px solid var(--dot);
    box-sizing: border-box;
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--dot) 30%, transparent);
  }
  .head-label {
    flex-shrink: 0;
    padding: 0 5px;
    border-radius: 4px;
    font-size: 10.5px;
    font-weight: 700;
    line-height: 16px;
    color: #fff;
    background: var(--accent, #0969da);
  }
  .repo-log-menu {
    position: fixed;
    z-index: 1000;
    min-width: 240px;
    max-width: 420px;
    padding: 4px;
    border-radius: 8px;
    border: 1px solid var(--border-primary);
    background: var(--bg-primary, #fff);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
    font-size: 12.5px;
  }
  .repo-log-menu .menu-title {
    padding: 4px 8px 6px;
    color: var(--text-secondary);
    font-size: 11.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .repo-log-menu button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 5px 8px;
    border: none;
    border-radius: 5px;
    background: none;
    color: var(--text-primary);
    font: inherit;
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
  }
  .repo-log-menu button:hover {
    background: var(--bg-hover, rgba(127, 127, 127, 0.12));
  }
  .repo-log-menu button.danger {
    color: var(--danger, #cf222e);
  }
  .menu-sep {
    height: 1px;
    margin: 4px 0;
    background: var(--border-secondary);
  }
  .repo-log-overlay {
    position: fixed;
    inset: 0;
    z-index: 1001;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.25);
  }
  .repo-log-prompt {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: min(420px, calc(100vw - 32px));
    padding: 14px;
    border-radius: 10px;
    border: 1px solid var(--border-primary);
    background: var(--bg-primary, #fff);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.25);
    font-size: 13px;
    color: var(--text-primary);
  }
  .prompt-title {
    font-weight: 600;
  }
  .repo-log-prompt input[type='text'] {
    padding: 6px 8px;
    border-radius: 6px;
    border: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    color: var(--text-primary);
    font: inherit;
  }
  .prompt-check {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
  }
  .prompt-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .prompt-actions button {
    padding: 5px 14px;
    border-radius: 6px;
    border: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    color: var(--text-primary);
    font: inherit;
    cursor: pointer;
  }
  .prompt-actions button.primary {
    background: var(--accent, #0969da);
    border-color: var(--accent, #0969da);
    color: #fff;
  }
  .prompt-actions button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .repo-log-notice {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--text-secondary);
    max-width: 50%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .repo-log-notice.error {
    color: var(--danger, #cf222e);
  }
  .repo-log-count {
    margin-right: auto;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .close-btn {
    font-size: 12.5px;
    padding: 5px 14px;
    border-radius: 6px;
    border: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    color: var(--text-primary);
    cursor: pointer;
  }
  :global(.git-repo-log-dialog .spin) {
    animation: repo-log-spin 1s linear infinite;
  }
  @keyframes repo-log-spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
