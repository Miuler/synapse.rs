<script lang="ts">
  import type { NormalizedGitStatus } from '../lib/git-status';

  interface Props {
    status?: NormalizedGitStatus | null;
    class?: string;
  }

  let {
    status = null,
    class: className = ''
  }: Props = $props();
</script>

{#if status}
  <div class="git-badges-group {className}" title="Estado Git de este archivo">
    {#if status.index === 'M'}
      <span class="git-badge staged" title="Modificado y en stage (preparado)">M</span>
    {:else if status.index === 'A'}
      <span class="git-badge staged" title="Nuevo archivo en stage (preparado)">A</span>
    {/if}

    {#if status.worktree === 'M'}
      <span class="git-badge modified" title="Modificaciones locales sin preparar">M</span>
    {:else if status.worktree === '?'}
      <span class="git-badge untracked" title="No controlado por Git (incógnita)">?</span>
    {/if}

    {#if status.is_stashed}
      <span class="git-badge stashed" title="En Stash de Git">S</span>
    {/if}
  </div>
{/if}

<style>
  .git-badges-group {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    flex-shrink: 0;
  }

  .git-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 9px;
    font-weight: 700;
    min-width: 15px;
    height: 15px;
    padding: 0 3px;
    border-radius: 3px;
    line-height: 1;
    cursor: help;
  }

  /*.git-badge.modified {*/
  /*  background: var(--git-modified-bg, rgba(217, 119, 6, 0.15));*/
  /*  color: var(--git-modified, #d97706);*/
  /*  border: 1px solid var(--git-modified-border, rgba(217, 119, 6, 0.3));*/
  /*}*/
  /*.git-badge.modified {*/
  /*  background: var(--git-modified-bg, rgba(220, 38, 38, 0.15));*/
  /*  color: var(--git-modified, #dc2626);*/
  /*  border: 1px solid var(--git-modified-border, rgba(220, 38, 38, 0.3));*/
  /*}*/

  .git-badge.modified {
    --git-modified-base: #ff4f4f;
    background: var(--git-modified-bg, rgb(from var(--git-modified-base) r g b / 0.15));
    color: var(--git-modified, var(--git-modified-base));
    border: 1px solid var(--git-modified-border, rgb(from var(--git-modified-base) r g b / 0.3));
  }

  .git-badge.staged {
    background: var(--git-staged-bg, rgba(22, 163, 74, 0.15));
    color: var(--git-staged, #16a34a);
    border: 1px solid var(--git-staged-border, rgba(22, 163, 74, 0.3));
  }

  .git-badge.untracked {
    background: var(--git-untracked-bg, rgba(202, 138, 4, 0.15));
    color: var(--git-untracked, #ca8a04);
    border: 1px solid var(--git-untracked-border, rgba(202, 138, 4, 0.3));
  }

  .git-badge.stashed {
    background: var(--git-stashed-bg, rgba(124, 58, 237, 0.15));
    color: var(--git-stashed, #7c3aed);
    border: 1px solid var(--git-stashed-border, rgba(124, 58, 237, 0.3));
  }
</style>
