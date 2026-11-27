<script module lang="ts">
  // Cada pestaña nueva empieza en la sugerencia siguiente
  let opened = 0;
</script>

<script lang="ts">
  import { shortcutsConfig } from '@entities/settings';
  import { t } from '@shared/lib/i18n';
  // El mismo pulpo de las notificaciones del sistema (una sola copia de la imagen)
  import pulpo from '../../../../src-tauri/icons/notification-pulpo.png';

  // Los atajos se leen de la configuración: si se cambian, la sugerencia sigue siendo cierta
  const tips = $derived.by(() => {
    const ws = shortcutsConfig.workspace;
    return (
      [
        ['quickOpen', [ws.quick_open]],
        ['switchTab', [ws.prev_tab_outside_text, ws.next_tab_outside_text]],
        ['searchText', [ws.search_fulltext]],
        ['history', [ws.nav_back, ws.nav_forward]],
        ['palette', [ws.command_palette]],
        ['split', [ws.split_view]],
        ['newTab', [ws.new_tab]],
        ['editMode', [ws.toggle_edit_mode]],
        ['sidebar', [ws.toggle_sidebar]],
        ['outline', [ws.toggle_outline]],
        ['agent', [ws.toggle_agent]],
        ['locate', [ws.locate_active_file]],
        ['closeTab', [ws.close_tab]],
      ] as const
    )
      .map(([text, specs]) => ({ text, keys: specs.map(firstCombo).filter(Boolean) }))
      .filter((tip) => tip.keys.length > 0);
  });

  /** La primera combinación de un atajo («Alt+1, Ctrl+B» → «Alt+1»), con las flechas dibujadas. */
  function firstCombo(spec: string): string {
    return (spec ?? '').split(', ')[0].trim().replace('ArrowLeft', '←').replace('ArrowRight', '→');
  }

  let index = $state(opened++);
  const tip = $derived(tips[index % tips.length]);
</script>

{#if tip}
  <button type="button" class="shortcut-tip" data-tip={tip.text} title={t('emptyTab.nextTip')} onclick={() => index++}>
    <img class="tip-mascot" src={pulpo} alt="" width="64" height="64" />
    <span class="tip-bubble">
      {#each tip.keys as keys (keys)}<kbd>{keys}</kbd>{/each}
      <span>{t(`emptyTab.tip.${tip.text}`)}</span>
    </span>
  </button>
{/if}

<style>
  .shortcut-tip {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 4vh;
    padding: 0;
    border: none;
    background: transparent;
    font: inherit;
    cursor: pointer;
    z-index: 1;
  }

  .tip-mascot {
    flex-shrink: 0;
    animation: tipFloat 3s ease-in-out infinite;
  }

  .shortcut-tip:hover .tip-mascot {
    animation: tipWiggle 0.5s ease-in-out;
  }

  .tip-bubble {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    padding: 10px 14px;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-primary, #ffffff) 90%, transparent);
    color: var(--text-primary, #1f2328);
    font-size: 13px;
    text-align: left;
  }

  kbd {
    padding: 2px 6px;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 4px;
    background: var(--bg-secondary, #f6f8fa);
    font-family: var(--mono, monospace);
    font-size: 11px;
  }

  @keyframes tipFloat {
    50% { transform: translateY(-4px); }
  }

  @keyframes tipWiggle {
    25% { transform: rotate(-12deg); }
    75% { transform: rotate(12deg); }
  }

  @media (prefers-reduced-motion: reduce) {
    .tip-mascot,
    .shortcut-tip:hover .tip-mascot {
      animation: none;
    }
  }
</style>
