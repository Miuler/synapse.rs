<script lang="ts">
  /**
   * Pulpo robot de Synapse: marca lo que viene de otra bóveda (otra instancia de Synapse).
   * Toma el color del texto (`currentColor`) para seguir el tema.
   */
  interface Props {
    size?: number;
    title?: string;
    class?: string;
  }

  let { size = 14, title, class: className = '' }: Props = $props();
</script>

<svg
  class="octopus-icon {className}"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.7"
  stroke-linecap="round"
  stroke-linejoin="round"
  role={title ? 'img' : undefined}
  aria-hidden={title ? undefined : 'true'}
  aria-label={title}
>
  {#if title}<title>{title}</title>{/if}
  <!-- antena -->
  <path d="M12 2.2v2" />
  <circle cx="12" cy="1.8" r="0.9" fill="currentColor" stroke="none" />
  <!-- cabeza -->
  <path d="M5.5 12.2V10a6.5 6.5 0 0 1 13 0v2.2" />
  <path d="M5.5 12.2h13" />
  <!-- ojos de robot -->
  <rect x="8.2" y="7.6" width="2.4" height="2.4" rx="0.5" fill="currentColor" stroke="none" />
  <rect x="13.4" y="7.6" width="2.4" height="2.4" rx="0.5" fill="currentColor" stroke="none" />
  <!-- tentáculos -->
  <path d="M6.5 12.2c-.3 3-1.8 4.6-3.6 5.3" />
  <path d="M9.5 12.2c0 3.6-.8 6.2-2.6 8.2" />
  <path d="M14.5 12.2c0 3.6.8 6.2 2.6 8.2" />
  <path d="M17.5 12.2c.3 3 1.8 4.6 3.6 5.3" />
</svg>

<style>
  .octopus-icon {
    display: inline-block;
    vertical-align: -2px;
    flex: none;
  }
</style>
