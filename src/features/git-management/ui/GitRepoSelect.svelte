<script lang="ts">
  // Selector del repositorio sobre el que actúa un diálogo de git (solo con varios repositorios).
  import { FolderGit2 } from 'lucide-svelte';
  import { gitDialogState, repoLabel } from '@entities/git';

  let { disabled = false }: { disabled?: boolean } = $props();
</script>

{#if gitDialogState.repos.length > 1}
  <label class="git-repo-select" title="Repositorio">
    <FolderGit2 size={13} />
    <select bind:value={gitDialogState.repo} {disabled} aria-label="Repositorio">
      {#each gitDialogState.repos as r (r.path)}
        <option value={r.path}>{r.path || `${repoLabel('')} (raíz)`}{r.branch ? ` · ${r.branch}` : ''}</option>
      {/each}
    </select>
  </label>
{/if}

<style>
  .git-repo-select {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .git-repo-select select {
    max-width: 260px;
    font-size: 12px;
    padding: 2px 6px;
    border-radius: 6px;
    border: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    color: var(--text-primary);
  }
</style>
