<script lang="ts">
  import { Folder, FolderOpen, FolderGit2 } from 'lucide-svelte';

  interface Props {
    isOpen?: boolean;
    name?: string;
    size?: number;
    strokeWidth?: number;
    class?: string;
  }

  let {
    isOpen = false,
    name = '',
    size = 15,
    strokeWidth = 2,
    class: className = ''
  }: Props = $props();

  const isGitFolder = $derived(name.toLowerCase() === '.git');
</script>

{#if isGitFolder}
  <FolderGit2 {size} {strokeWidth} class="synapse-folder-icon synapse-folder-git {className}" />
{:else if isOpen}
  <FolderOpen {size} {strokeWidth} class="synapse-folder-icon synapse-folder-open {className}" />
{:else}
  <Folder {size} {strokeWidth} class="synapse-folder-icon synapse-folder-closed {className}" />
{/if}

<style>
  :global(.synapse-folder-icon) {
    flex-shrink: 0;
    display: inline-block;
    vertical-align: middle;
    color: var(--icon-folder-color, var(--accent, #0969da));
    transition: color 0.15s ease;
  }

  :global(.synapse-folder-icon.synapse-folder-git) {
    color: var(--icon-folder-git-color, #fd8c73);
  }
</style>
