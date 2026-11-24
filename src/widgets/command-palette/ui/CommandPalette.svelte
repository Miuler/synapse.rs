<script lang="ts">
  import { commandAliases, commandCategory, commandName, commandRegistry, type AppCommand } from '@entities/command';
  import { Command, computeCommandScore, Dialog } from 'bits-ui';
  import { SearchBox } from '@shared/ui/search-box';
  import { i18n, t } from '@shared/lib/i18n';
  import { Kbd } from '@shared/ui/kbd';
  import { DialogResizeHandles, persistedDialogSize } from '@shared/ui/dialog-resize';

  interface Props {
    isOpen?: boolean;
    onClose?: () => void;
  }

  let { isOpen = $bindable(false), onClose }: Props = $props();

  let searchValue = $state('');

  const dialogSize = persistedDialogSize(
    'synapse.commandPalette.size',
    { w: 620, h: 440 },
    { w: 400, h: 260 },
    { centeredX: true, centeredY: false, maxHeightMargin: 140 },
  );

  // La apertura global Ctrl+P se maneja de forma centralizada en WorkspacePage para evitar listeners duplicados

  function closePalette() {
    isOpen = false;
    searchValue = '';
    if (onClose) onClose();
  }

  function executeCommand(cmd: AppCommand) {
    cmd.action();
    closePalette();
  }

  // bits-ui filtra pero no reordena los elementos sin grupo: se ordenan aquí, del más parecido al menos
  let shownCommands = $derived.by(() => {
    const query = searchValue.trim();
    if (!query) return commandRegistry.all;
    return commandRegistry.all
      .map((cmd) => ({
        cmd,
        score: computeCommandScore(
          `${commandName(cmd.id, i18n.locale)} ${commandCategory(cmd.category, i18n.locale)} ${cmd.shortcut || ''}`,
          query,
          commandAliases(cmd.id, cmd.category, i18n.locale),
        ),
      }))
      .filter((r) => r.score > 0)
      .sort((a, b) => b.score - a.score)
      .map((r) => r.cmd);
  });

  // Al escribir, queda elegido el primer resultado
  let selected = $state('');
  $effect(() => {
    selected = searchValue.trim() && shownCommands.length ? shownCommands[0].id : selected;
  });
</script>

<Dialog.Root
  open={isOpen}
  onOpenChange={(open) => {
    isOpen = open;
    if (!open) {
      closePalette();
    }
  }}
>
  <Dialog.Portal>
    <Dialog.Overlay class="palette-backdrop" />
    <Dialog.Content
      class={`palette-container synapse-dialog synapse-command-palette ${dialogSize.resizing ? 'is-resizing' : ''}`}
      style={dialogSize.style}
    >
      <DialogResizeHandles onStart={dialogSize.startResize} showCornerGrip />
      <Dialog.Title class="sr-only">{t('palette.title')}</Dialog.Title>
      <Command.Root class="command-root" loop shouldFilter={false} bind:value={selected}>
        <SearchBox
          bind:value={searchValue}
          placeholder={t('palette.placeholder')}
        />

        <Command.List class="results-container">
          <Command.Empty class="empty-state">{t('palette.empty')}</Command.Empty>

          {#each shownCommands as cmd (cmd.id)}
            <Command.Item
              class="command-item"
              value={cmd.id}
              onSelect={() => executeCommand(cmd)}
            >
              <span class="category-tag">{commandCategory(cmd.category, i18n.locale)}</span>
              <span class="command-name">{commandName(cmd.id, i18n.locale)}</span>
              {#if cmd.shortcut}
                <Kbd shortcut={cmd.shortcut} />
              {/if}
            </Command.Item>
          {/each}
        </Command.List>

        <footer class="palette-footer">
          <span><kbd>↑</kbd> <kbd>↓</kbd> {t('palette.navigate')}</span>
          <span><kbd>↵</kbd> {t('palette.run')}</span>
          <span><kbd>esc</kbd> {t('palette.close')}</span>
        </footer>
      </Command.Root>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.sr-only) {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border-width: 0;
  }

  :global(.palette-backdrop) {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background-color: var(--dialog-backdrop, rgba(10, 12, 16, 0.7));
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 14vh;
    z-index: 1000;
    animation: fadeIn 0.15s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  :global(.palette-container) {
    position: fixed;
    top: 14vh;
    left: 50%;
    transform: translateX(-50%);
    background-color: var(--bg-primary, #ffffff);
    border-radius: 12px;
    border: 1px solid var(--border-primary, #d0d7de);
    box-shadow: var(--modal-shadow, 0 24px 48px rgba(0, 0, 0, 0.1));
    overflow: hidden;
    display: flex;
    flex-direction: column;
    z-index: 1001;
    outline: none;
    animation: slideDown 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  :global(.palette-container.is-resizing) {
    animation: none !important;
    user-select: none !important;
  }

  @keyframes slideDown {
    from { transform: translateX(-50%) translateY(-12px) scale(0.98); opacity: 0; }
    to { transform: translateX(-50%) translateY(0) scale(1); opacity: 1; }
  }

  :global(.palette-container .command-root) {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    width: 100%;
    outline: none;
  }

  :global(.palette-container .results-container) {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 6px 0;
    outline: none;
  }

  :global(.palette-container .empty-state) {
    padding: 24px;
    text-align: center;
    color: var(--text-secondary, #656d76);
    font-size: 14px;
  }

  :global(.palette-container .command-item) {
    display: flex;
    align-items: center;
    padding: 10px 16px;
    cursor: pointer;
    font-size: 14px;
    color: var(--text-secondary, #656d76);
    user-select: none;
    outline: none;
    contain: layout style paint;
  }

  :global(.palette-container .command-item:hover),
  :global(.palette-container .command-item[data-selected]),
  :global(.palette-container .command-item[data-highlighted]) {
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.1));
    color: var(--text-primary, #1f2328);
  }

  :global(.palette-container .command-item[data-selected] .category-tag),
  :global(.palette-container .command-item[data-highlighted] .category-tag) {
    color: var(--accent, #0969da);
  }

  :global(.palette-container .category-tag) {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary, #656d76);
    margin-right: 12px;
    min-width: 80px;
  }

  :global(.palette-container .command-name) {
    flex-grow: 1;
  }

  :global(.palette-container .shortcut-badge) {
    font-family: var(--mono, monospace);
    font-size: 11px;
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-primary, #1f2328);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--border-primary, #d0d7de);
  }

  :global(.palette-container .palette-footer) {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    padding: 8px 16px;
    background-color: var(--footer-bg, rgba(0, 0, 0, 0.03));
    border-top: 1px solid var(--border-primary, #d0d7de);
    font-size: 11px;
    color: var(--text-secondary, #656d76);
  }

  :global(.palette-container .palette-footer kbd) {
    font-family: var(--mono, monospace);
    background: var(--bg-secondary, #f6f8fa);
    padding: 1px 4px;
    border-radius: 3px;
    color: var(--text-primary, #1f2328);
    border: 1px solid var(--border-primary, #d0d7de);
  }
</style>
