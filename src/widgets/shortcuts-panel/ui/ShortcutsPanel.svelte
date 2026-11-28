<script lang="ts">
  import { t } from '@shared/lib/i18n';
  import { Dialog } from 'bits-ui';
  import { shortcutsConfig, appSettings } from '@entities/settings';
  import { Kbd } from '@shared/ui/kbd';
  import { SearchBox } from '@shared/ui/search-box';
  import { DialogResizeHandles, persistedDialogSize } from '@shared/ui/dialog-resize';

  interface Props {
    isOpen?: boolean;
  }

  let { isOpen = $bindable(false) }: Props = $props();

  interface Entry {
    label: string;
    shortcut: string;
  }
  interface Group {
    title: string;
    entries: Entry[];
  }

  let query = $state('');

  const dialogSize = persistedDialogSize(
    'synapse.shortcutsPanel.size',
    { w: 680, h: 560 },
    { w: 420, h: 300 },
    { centeredX: true, centeredY: false, maxHeightMargin: 120 },
  );

  // Los atajos configurables salen de shortcuts.jsonc; el resto son atajos fijos de los editores
  let groups = $derived.by<Group[]>(() => {
    const ws = shortcutsConfig.workspace;
    const all: Group[] = [
      {
        title: t('shortcutsPanel.groups.filesAndTabs'),
        entries: [
          { label: t('shortcutLabels.actions.quick_open'), shortcut: ws.quick_open },
          { label: t('shortcutLabels.actions.new_file'), shortcut: ws.new_file },
          { label: t('shortcutLabels.actions.new_tab'), shortcut: ws.new_tab },
          { label: t('shortcutLabels.actions.save_file'), shortcut: ws.save_file },
          { label: t('shortcutLabels.actions.close_tab'), shortcut: ws.close_tab },
          { label: t('shortcutLabels.actions.next_tab'), shortcut: ws.next_tab },
          { label: t('shortcutLabels.actions.prev_tab'), shortcut: ws.prev_tab },
          { label: t('shortcutLabels.actions.next_tab_outside_text'), shortcut: ws.next_tab_outside_text },
          { label: t('shortcutLabels.actions.prev_tab_outside_text'), shortcut: ws.prev_tab_outside_text },
          { label: t('shortcutLabels.actions.split_view'), shortcut: ws.split_view },
          { label: t('shortcutLabels.actions.toggle_edit_mode'), shortcut: ws.toggle_edit_mode },
        ],
      },
      {
        title: t('shortcutsPanel.groups.navigation'),
        entries: [
          { label: t('shortcutLabels.actions.command_palette'), shortcut: ws.command_palette },
          { label: t('shortcutLabels.actions.search_fulltext'), shortcut: ws.search_fulltext },
          { label: t('shortcutLabels.actions.nav_back'), shortcut: ws.nav_back },
          { label: t('shortcutLabels.actions.nav_forward'), shortcut: ws.nav_forward },
          { label: t('shortcutLabels.actions.toggle_sidebar'), shortcut: ws.toggle_sidebar },
          { label: t('shortcutLabels.actions.toggle_agent'), shortcut: ws.toggle_agent },
          { label: t('shortcutLabels.actions.locate_active_file'), shortcut: ws.locate_active_file },
          { label: t('shortcutLabels.actions.open_vault_folder'), shortcut: ws.open_vault_folder },
          { label: t('shortcutsPanel.fixed.keyboardShortcuts'), shortcut: 'F1' },
        ],
      },
      {
        title: t('shortcutsPanel.groups.editor'),
        entries: [
          { label: t('shortcutsPanel.fixed.undo'), shortcut: 'Ctrl+Z' },
          { label: t('shortcutsPanel.fixed.redo'), shortcut: 'Ctrl+Shift+Z, Ctrl+Y' },
          { label: t('shortcutsPanel.fixed.commentLines'), shortcut: 'Ctrl+/' },
          { label: t('shortcutsPanel.fixed.indent'), shortcut: 'Tab, Shift+Tab' },
          { label: t('shortcutLabels.actions.toggle_task'), shortcut: ws.toggle_task },
          { label: t('shortcutLabels.actions.zoom_in'), shortcut: ws.zoom_in },
          { label: t('shortcutLabels.actions.zoom_out'), shortcut: ws.zoom_out },
          { label: t('shortcutLabels.actions.zoom_reset'), shortcut: ws.zoom_reset },
          { label: t('shortcutsPanel.fixed.findInFile'), shortcut: 'Ctrl+F' },
          { label: t('chatSearch.placeholder'), shortcut: 'Ctrl+F' },
          { label: t('shortcutsPanel.fixed.findReplaceInFile'), shortcut: 'Ctrl+R' },
        ],
      },
      {
        title: t('shortcutsPanel.groups.explorer'),
        entries: [
          { label: t('shortcutsPanel.fixed.copy'), shortcut: 'Ctrl+C' },
          { label: t('shortcutsPanel.fixed.cut'), shortcut: 'Ctrl+X' },
          { label: t('shortcutsPanel.fixed.paste'), shortcut: 'Ctrl+V' },
          { label: t('shortcutsPanel.fixed.reload'), shortcut: 'F5, Ctrl+R' },
          { label: t('shortcutsPanel.fixed.delete'), shortcut: 'Delete' },
        ],
      },
      {
        title: t('shortcutsPanel.groups.viewAndDev'),
        entries: [
          { label: t('shortcutLabels.actions.reload_active_file'), shortcut: ws.reload_active_file },
          { label: t('shortcutLabels.actions.reload_vault'), shortcut: ws.reload_vault },
          { label: t('shortcutLabels.actions.toggle_theme'), shortcut: ws.toggle_theme },
          { label: t('shortcutLabels.actions.pick_theme'), shortcut: ws.pick_theme },
          { label: t('shortcutLabels.actions.toggle_outline'), shortcut: ws.toggle_outline },
          { label: t('shortcutLabels.actions.toggle_devtools'), shortcut: ws.toggle_devtools },
          { label: t('shortcutLabels.actions.git_push'), shortcut: ws.git_push },
        ],
      },
    ];
    // Los atajos de vim solo aparecen si el modo vim está activo
    if (appSettings.vimMode) {
      all.push({
        title: t('shortcutsPanel.groups.vim'),
        entries: [
          { label: t('shortcutsPanel.fixed.undo'), shortcut: 'u' },
          { label: t('shortcutsPanel.fixed.vimUndoLastLine'), shortcut: 'U' },
          { label: t('shortcutsPanel.fixed.redo'), shortcut: 'Ctrl+R' },
        ],
      });
    }
    return all;
  });

  let filtered = $derived.by<Group[]>(() => {
    const q = query.trim().toLowerCase();
    if (!q) return groups;
    return groups
      .map((g) => ({
        ...g,
        entries: g.entries.filter((e) => `${g.title} ${e.label} ${e.shortcut}`.toLowerCase().includes(q)),
      }))
      .filter((g) => g.entries.length > 0);
  });

  $effect(() => {
    if (!isOpen) query = '';
  });
</script>

<Dialog.Root bind:open={isOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="shortcuts-backdrop" />
    <Dialog.Content
      class={`shortcuts-panel synapse-dialog ${dialogSize.resizing ? 'is-resizing' : ''}`}
      style={dialogSize.style}
    >
      <DialogResizeHandles onStart={dialogSize.startResize} />
      <Dialog.Title class="shortcuts-title">{t('shortcutsPanel.title')}</Dialog.Title>
      <SearchBox bind:value={query} placeholder={t('shortcutsPanel.searchPlaceholder')} />

      <div class="shortcuts-body">
        {#each filtered as group (group.title)}
          <section class="shortcuts-group">
            <h3>{group.title}</h3>
            <ul>
              {#each group.entries as entry (entry.label + entry.shortcut)}
                <li>
                  <span class="shortcut-label">{entry.label}</span>
                  <Kbd shortcut={entry.shortcut} all />
                </li>
              {/each}
            </ul>
          </section>
        {:else}
          <p class="shortcuts-empty">{t('shortcutsPanel.empty')}</p>
        {/each}
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.shortcuts-backdrop) {
    position: fixed;
    inset: 0;
    background-color: var(--dialog-backdrop, rgba(10, 12, 16, 0.7));
    z-index: 1000;
  }

  :global(.shortcuts-panel) {
    position: fixed;
    top: 10vh;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    background: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 10px;
    box-shadow: var(--modal-shadow);
    overflow: hidden;
    box-sizing: border-box;
    z-index: 1001;
  }

  :global(.shortcuts-panel.is-resizing) {
    user-select: none !important;
  }

  :global(.shortcuts-title) {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }

  .shortcuts-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 16px 24px;
    padding-right: 4px;
  }

  .shortcuts-group h3 {
    margin: 0 0 6px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-secondary, #656d76);
  }

  .shortcuts-group ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .shortcuts-group li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 5px 0;
    font-size: 13px;
    border-bottom: 1px solid var(--border-primary, #d0d7de33);
  }

  .shortcuts-empty {
    grid-column: 1 / -1;
    text-align: center;
    color: var(--text-secondary, #656d76);
  }
</style>
