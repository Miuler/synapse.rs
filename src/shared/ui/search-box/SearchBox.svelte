<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Command } from 'bits-ui';
  import { Search } from 'lucide-svelte';

  interface Props {
    value?: string;
    placeholder?: string;
    loading?: boolean;
    class?: string;
    icon?: Snippet;
    badge?: Snippet;
    children?: Snippet;
  }

  let {
    value = $bindable(''),
    placeholder = 'Buscar...',
    loading = false,
    class: className = '',
    icon,
    badge,
    children,
  }: Props = $props();
</script>

<div class="search-box-wrapper {className}">
  {#if icon}
    {@render icon()}
  {:else}
    <Search size={16} class="search-box-icon" />
  {/if}

  <Command.Input
    class="search-box-input"
    bind:value
    {placeholder}
  />

  {#if badge}
    {@render badge()}
  {/if}

  {#if loading}
    <span class="search-box-spinner" aria-label="Buscando..."></span>
  {/if}

  {#if children}
    {@render children()}
  {/if}

  <span class="search-box-esc">ESC</span>
</div>

<style>
  .search-box-wrapper {
    display: flex;
    align-items: center;
    position: relative;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    background: var(--bg-secondary, #f6f8fa);
    flex-shrink: 0;
    gap: 12px;
    width: 100%;
    box-sizing: border-box;
  }

  :global(.search-box-icon) {
    position: static !important;
    left: auto !important;
    top: auto !important;
    width: 18px;
    height: 18px;
    color: var(--text-secondary, #656d76);
    margin-right: 0;
    flex-shrink: 0;
    pointer-events: none;
    z-index: 1;
  }

  :global(.search-box-input) {
    flex: 1 1 auto;
    min-width: 0;
    width: auto;
    background: transparent;
    border: none;
    padding: 0;
    margin: 0;
    color: var(--text-primary, #1f2328);
    font-size: 15px;
    outline: none;
    font-family: inherit;
    box-shadow: none;
  }

  :global(.search-box-input::placeholder) {
    color: var(--text-secondary, #656d76);
  }

  .search-box-esc {
    font-size: 11px;
    font-family: var(--mono, monospace);
    color: var(--text-secondary, #656d76);
    background: var(--bg-primary, #ffffff);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--border-primary, #d0d7de);
    flex-shrink: 0;
    user-select: none;
  }

  .search-box-spinner {
    width: 14px;
    height: 14px;
    border: 2px solid var(--border-primary, #d0d7de);
    border-top-color: var(--accent, #0969da);
    border-radius: 50%;
    animation: searchBoxSpin 0.6s linear infinite;
    flex-shrink: 0;
  }

  @keyframes searchBoxSpin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
