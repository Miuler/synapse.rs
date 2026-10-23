<script lang="ts">
  import { appSettings } from '@entities/settings';
  import { Sun, Moon } from 'lucide-svelte';

  interface Props {
    class?: string;
  }

  let { class: className = '' }: Props = $props();
</script>

<button
  type="button"
  class="theme-btn {className}"
  onclick={() => appSettings.cycleTheme()}
  title="Cambiar tema: {appSettings.theme === 'system' ? `Sistema (${appSettings.resolvedTheme === 'dark' ? 'Oscuro' : 'Claro'})` : appSettings.theme === 'dark' ? 'Oscuro' : 'Claro'} (Clic para alternar)"
  aria-label="Cambiar tema"
>
  {#if appSettings.resolvedTheme === 'dark'}
    <Moon size={13} class="theme-icon" />
  {:else}
    <Sun size={13} class="theme-icon" />
  {/if}
  <span class="theme-text">{appSettings.theme === 'system' ? 'Auto' : appSettings.theme === 'dark' ? 'Oscuro' : 'Claro'}</span>
</button>

<style>
  .theme-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 6px;
    height: 20px;
    background: transparent;
    border: none;
    color: inherit;
    cursor: pointer;
    border-radius: 4px;
    transition: all 0.15s ease;
    font-size: 11px;
    font-weight: 500;
    font-family: inherit;
    user-select: none;
  }

  .theme-btn:hover {
    background: var(--hover-bg, rgba(255, 255, 255, 0.08));
    color: var(--accent, #0969da);
  }

  :global(html[data-theme='light']) .theme-btn:hover {
    background: rgba(0, 0, 0, 0.05);
  }

  :global(.theme-icon) {
    color: var(--accent, #0969da);
    flex-shrink: 0;
  }

  .theme-text {
    font-size: 11px;
    line-height: 1;
  }
</style>
