<script lang="ts">
  import { Popover } from 'bits-ui';
  import { Pencil } from 'lucide-svelte';
  import { appSettings, EMPTY_TAB_PARTS, MAX_RECENTS_COUNT, NO_WALLPAPER, type EmptyTabPart } from '@entities/settings';
  import { invokeTauri } from '@shared/repositories/tauri';
  import { t } from '@shared/lib/i18n';

  /** `active`: la pestaña está a la vista; al dejar de estarlo, el panel se cierra. */
  let { active = true }: { active?: boolean } = $props();

  let open = $state(false);
  $effect(() => {
    if (!active) open = false;
  });

  const LABELS = {
    search: 'emptyTab.search',
    recents: 'emptyTab.recents',
    create: 'emptyTab.createButton',
    tips: 'emptyTab.tips',
  } as const satisfies Record<EmptyTabPart, string>;

  const anyShown = $derived(EMPTY_TAB_PARTS.some((part) => appSettings.emptyTab[part]));
  const setAll = (shown: boolean) =>
    appSettings.setEmptyTab(Object.fromEntries(EMPTY_TAB_PARTS.map((part) => [part, shown])));

  async function chooseWallpaper() {
    try {
      const path = await invokeTauri<string | null>('pick_image_file');
      if (path) appSettings.setWallpaper(path);
    } catch (err) {
      console.warn('No se pudo elegir el fondo:', err);
    }
  }
</script>

<Popover.Root bind:open>
  <Popover.Trigger class="customize-btn">
    <Pencil size={14} />
    <span>{t('emptyTab.customize')}</span>
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content class="customize-panel" side="top" align="end" sideOffset={8}>
      <strong>{t('emptyTab.title')}</strong>
      {#each EMPTY_TAB_PARTS as part (part)}
        <label class="customize-row">
          <span>{t(LABELS[part])}</span>
          <input
            type="checkbox"
            data-part={part}
            checked={appSettings.emptyTab[part]}
            onchange={(e) => appSettings.setEmptyTab({ [part]: e.currentTarget.checked })}
          />
        </label>
        {#if part === 'recents' && appSettings.emptyTab.recents}
          <label class="customize-row sub">
            <span>{t('emptyTab.count')}</span>
            <input
              type="number"
              class="customize-count"
              min="1"
              max={MAX_RECENTS_COUNT}
              value={appSettings.recentsCount}
              onchange={(e) => {
                appSettings.setRecentsCount(e.currentTarget.valueAsNumber);
                e.currentTarget.value = String(appSettings.recentsCount);
              }}
            />
          </label>
        {/if}
      {/each}
      <div class="customize-section">{t('emptyTab.background')}</div>
      <div class="customize-buttons">
        <button type="button" class:chosen={!!appSettings.wallpaper && appSettings.wallpaper !== NO_WALLPAPER} onclick={chooseWallpaper}>
          {t('emptyTab.chooseImage')}
        </button>
        <button type="button" data-wallpaper="default" class:chosen={!appSettings.wallpaper} onclick={() => appSettings.setWallpaper(undefined)}>
          {t('emptyTab.defaultBackground')}
        </button>
        <button type="button" data-wallpaper="none" class:chosen={appSettings.wallpaper === NO_WALLPAPER} onclick={() => appSettings.setWallpaper(NO_WALLPAPER)}>
          {t('emptyTab.noBackground')}
        </button>
      </div>
      <button type="button" class="customize-all" onclick={() => setAll(!anyShown)}>
        {anyShown ? t('emptyTab.onlyBackground') : t('emptyTab.showAll')}
      </button>
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>

<style>
  :global(.customize-btn) {
    position: absolute;
    right: 16px;
    bottom: 16px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 12px;
    color: var(--text-secondary, #656d76);
    background: color-mix(in srgb, var(--bg-primary, #ffffff) 88%, transparent);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 999px;
    cursor: pointer;
    z-index: 2;
  }

  :global(.customize-btn:hover) {
    color: var(--text-primary, #1f2328);
    border-color: var(--accent, #0969da);
  }

  :global(.customize-panel) {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 280px;
    padding: 14px;
    background: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 12px;
    box-shadow: var(--popover-shadow);
    font-size: 13px;
    z-index: 1000;
    outline: none;
    user-select: none;
  }

  .customize-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    cursor: pointer;
  }

  .customize-row.sub {
    padding-left: 14px;
    color: var(--text-secondary, #656d76);
  }

  .customize-count {
    width: 60px;
  }

  .customize-section {
    padding-top: 10px;
    border-top: 1px solid var(--border-primary, #d0d7de);
    color: var(--text-secondary, #656d76);
  }

  .customize-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .customize-buttons button,
  .customize-all,
  .customize-count {
    padding: 4px 10px;
    font: inherit;
    font-size: 12px;
    color: var(--text-primary, #1f2328);
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
  }

  .customize-buttons button,
  .customize-all {
    cursor: pointer;
  }

  .customize-buttons button.chosen {
    border-color: var(--accent, #0969da);
    color: var(--accent, #0969da);
  }

  .customize-buttons button:hover,
  .customize-all:hover {
    border-color: var(--accent, #0969da);
  }
</style>
