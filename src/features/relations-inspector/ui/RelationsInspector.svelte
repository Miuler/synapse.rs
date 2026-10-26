<script lang="ts">
  import { onMount } from 'svelte';
  import { vaultApi, type NoteBacklinksReport } from '@entities/vault-item';
  import { Link, ArrowDownLeft, ArrowUpRight, Search, FileText, Sparkles, RefreshCw } from 'lucide-svelte';

  interface Props {
    activeNotePath: string | null;
    onSelectNote: (notePath: string) => void;
  }

  let { activeNotePath, onSelectNote }: Props = $props();

  let report = $state<NoteBacklinksReport | null>(null);
  let isLoading = $state(false);
  let activeTab = $state<'backlinks' | 'outlinks' | 'unlinked'>('backlinks');

  async function loadRelations() {
    if (!activeNotePath) {
      report = null;
      return;
    }
    isLoading = true;
    try {
      report = await vaultApi.getNoteBacklinksAndRelations(activeNotePath);
    } catch (err) {
      console.error('Error cargando relaciones de la nota:', err);
      report = null;
    } finally {
      isLoading = false;
    }
  }

  $effect(() => {
    if (activeNotePath) {
      loadRelations();
    } else {
      report = null;
    }
  });

  onMount(() => {
    if (activeNotePath) {
      loadRelations();
    }
  });
</script>

<div class="relations-inspector">
  <div class="inspector-header">
    <div class="header-left">
      <Link size={14} class="header-icon" />
      <span class="header-title">Enlaces & Menciones</span>
    </div>
    <button
      class="refresh-btn"
      onclick={loadRelations}
      title="Recargar relaciones"
      disabled={isLoading || !activeNotePath}
    >
      <span class={isLoading ? 'spinning' : ''} style="display: inline-flex;">
        <RefreshCw size={12} />
      </span>
    </button>
  </div>

  {#if !activeNotePath}
    <div class="empty-state">
      Selecciona una nota para inspeccionar sus conexiones.
    </div>
  {:else if isLoading && !report}
    <div class="empty-state">
      Analizando enlaces entrantes y salientes...
    </div>
  {:else if report}
    <!-- Selector de Pestañas -->
    <div class="inspector-tabs">
      <button
        class="tab-btn"
        class:active={activeTab === 'backlinks'}
        onclick={() => (activeTab = 'backlinks')}
      >
        <ArrowDownLeft size={12} />
        <span>Entrantes ({report.backlinks.length})</span>
      </button>

      <button
        class="tab-btn"
        class:active={activeTab === 'outlinks'}
        onclick={() => (activeTab = 'outlinks')}
      >
        <ArrowUpRight size={12} />
        <span>Salientes ({report.outlinks.length})</span>
      </button>

      <button
        class="tab-btn"
        class:active={activeTab === 'unlinked'}
        onclick={() => (activeTab = 'unlinked')}
      >
        <Sparkles size={12} />
        <span>Menciones ({report.unlinked_mentions.length})</span>
      </button>
    </div>

    <!-- Contenido de la Pestaña -->
    <div class="inspector-content">
      {#if activeTab === 'backlinks'}
        {#if report.backlinks.length === 0}
          <div class="empty-msg">No hay notas que enlacen a este documento.</div>
        {:else}
          <div class="items-list">
            {#each report.backlinks as b}
              <div
                class="relation-card"
                role="button"
                tabindex="0"
                onclick={() => onSelectNote(b.source_path)}
                onkeydown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    onSelectNote(b.source_path);
                  }
                }}
              >
                <div class="card-top">
                  <FileText size={12} class="card-icon" />
                  <span class="card-title">{b.source_title || b.source_path}</span>
                </div>
                {#if b.snippet}
                  <div class="card-snippet">{b.snippet}</div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      {:else if activeTab === 'outlinks'}
        {#if report.outlinks.length === 0}
          <div class="empty-msg">Esta nota no tiene enlaces salientes `[[...]]`.</div>
        {:else}
          <div class="items-list">
            {#each report.outlinks as o}
              <div
                class="relation-card"
                class:is-unresolved={!o.resolved_path}
                role="button"
                tabindex="0"
                onclick={() => {
                  if (o.resolved_path) onSelectNote(o.resolved_path);
                  else onSelectNote(o.target.endsWith('.md') ? o.target : `${o.target}.md`);
                }}
                onkeydown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    if (o.resolved_path) onSelectNote(o.resolved_path);
                    else onSelectNote(o.target.endsWith('.md') ? o.target : `${o.target}.md`);
                  }
                }}
              >
                <div class="card-top">
                  <ArrowUpRight size={12} class="card-icon" />
                  <span class="card-title">{o.alias || o.target}</span>
                  {#if !o.resolved_path}
                    <span class="ghost-tag">pendiente</span>
                  {/if}
                </div>
                {#if o.resolved_path}
                  <div class="card-path">{o.resolved_path}</div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      {:else if activeTab === 'unlinked'}
        {#if report.unlinked_mentions.length === 0}
          <div class="empty-msg">No se encontraron menciones textuales sin enlazar.</div>
        {:else}
          <div class="items-list">
            {#each report.unlinked_mentions as m}
              <div
                class="relation-card"
                role="button"
                tabindex="0"
                onclick={() => onSelectNote(m.source_path)}
                onkeydown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    onSelectNote(m.source_path);
                  }
                }}
              >
                <div class="card-top">
                  <FileText size={12} class="card-icon" />
                  <span class="card-title">{m.source_title || m.source_path}</span>
                </div>
                {#if m.snippet}
                  <div class="card-snippet">{m.snippet}</div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .relations-inspector {
    display: flex;
    flex-direction: column;
    height: 100%;
    background-color: var(--bg-secondary, #f6f8fa);
    border-top: 1px solid var(--border-primary, #d0d7de);
    overflow: hidden;
  }

  .inspector-header {
    height: 32px;
    padding: 0 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    background-color: var(--bg-primary, #ffffff);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  :global(.header-icon) {
    color: var(--accent, #0969da);
  }

  .header-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
  }

  .refresh-btn {
    border: none;
    background: transparent;
    color: var(--text-secondary, #656d76);
    padding: 2px;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .refresh-btn:hover {
    background-color: var(--bg-tertiary, #eaeef2);
    color: var(--text-primary, #1f2328);
  }

  .spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .inspector-tabs {
    display: flex;
    background-color: var(--bg-secondary, #f6f8fa);
    padding: 4px;
    gap: 2px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
  }

  .tab-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 3px 6px;
    font-size: 10px;
    font-weight: 500;
    color: var(--text-secondary, #656d76);
    background: transparent;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .tab-btn:hover {
    background-color: var(--bg-tertiary, #eaeef2);
  }

  .tab-btn.active {
    background-color: var(--bg-primary, #ffffff);
    color: var(--accent, #0969da);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
  }

  .inspector-content {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
  }

  .empty-state, .empty-msg {
    padding: 16px 12px;
    text-align: center;
    font-size: 11px;
    color: var(--text-muted, #8c959f);
  }

  .items-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .relation-card {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 5px 8px;
    border-radius: 5px;
    background-color: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    cursor: pointer;
    text-align: left;
    transition: all 0.1s ease;
  }

  .relation-card:hover {
    border-color: var(--accent, #0969da);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
  }

  .relation-card.is-unresolved {
    border-style: dashed;
  }

  .card-top {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  :global(.card-icon) {
    color: var(--accent, #0969da);
    flex-shrink: 0;
  }

  .card-title {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-primary, #1f2328);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ghost-tag {
    margin-left: auto;
    font-size: 9px;
    padding: 0 4px;
    border-radius: 3px;
    background-color: var(--bg-tertiary, #eaeef2);
    color: var(--text-muted, #8c959f);
  }

  .card-snippet {
    font-size: 10px;
    color: var(--text-secondary, #656d76);
    line-height: 1.3;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .card-path {
    font-size: 9px;
    color: var(--text-muted, #8c959f);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
