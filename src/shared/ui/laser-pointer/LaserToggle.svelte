<script lang="ts">
  // Botón del puntero láser, para la barra de un visor (dentro de un `Toolbar.Root`)
  import { t } from '@shared/lib/i18n';
  import { Toggle } from 'bits-ui';

  let { active = $bindable(false) }: { active?: boolean } = $props();
</script>

<Toggle.Root class="laser-toggle-btn {active ? 'active' : ''}" bind:pressed={active} title={t('laser.toggle')}>
  <span class="laser-dot"></span>
  {t('laser.label')}
</Toggle.Root>

<style>
  :global(.laser-toggle-btn) {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px !important;
    border: 1px solid transparent !important;
    border-radius: 6px !important;
    background: transparent;
    color: var(--text-primary, #24292f);
    font-size: 13px !important;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  :global(.laser-toggle-btn:hover) {
    background-color: var(--bg-hover, rgba(0, 0, 0, 0.08));
  }

  /* `.laser-toggle-btn .laser-dot`: gana a `.zoom-controls span` (min-width del «100%»), que lo estiraba */
  :global(.laser-toggle-btn .laser-dot) {
    flex: none;
    width: 8px;
    min-width: 0;
    height: 8px;
    border-radius: 50%;
    background-color: var(--text-muted);
    transition: background-color 0.2s ease, box-shadow 0.2s ease;
  }

  :global(.laser-toggle-btn.active) {
    background-color: color-mix(in srgb, var(--laser) 12%, transparent) !important;
    color: var(--laser) !important;
    border-color: color-mix(in srgb, var(--laser) 35%, transparent) !important;
  }

  :global(.laser-toggle-btn.active .laser-dot) {
    background-color: var(--laser);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--laser) 25%, transparent);
  }
</style>
