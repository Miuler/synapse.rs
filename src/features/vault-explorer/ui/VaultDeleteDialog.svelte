<script lang="ts">
  import { Trash2 } from 'lucide-svelte';
  import { ConfirmDialog } from '@shared/ui/confirm-dialog';
  import { t } from '@shared/lib/i18n';

  export interface ItemToDelete {
    name: string;
    relativePath: string;
    isFolder: boolean;
  }

  interface Props {
    open: boolean;
    items: ItemToDelete[];
    /** Carpetas de tareas de los tableros elegidos: se pregunta si se eliminan también. */
    boardFolders?: string[];
    deleteBoardFolders?: boolean;
    loading?: boolean;
    onConfirm: () => void | Promise<void>;
    onCancel: () => void;
  }

  let {
    open = $bindable(false),
    items = [],
    boardFolders = [],
    deleteBoardFolders = $bindable(true),
    loading = false,
    onConfirm,
    onCancel,
  }: Props = $props();

  let title = $derived(
    items.length > 1
      ? t('explorer.deleteDialog.titleMultiple', { count: items.length })
      : items[0]?.isFolder
        ? t('explorer.deleteDialog.titleFolder')
        : t('explorer.deleteDialog.titleFile')
  );

  let description = $derived(
    items.length > 1
      ? t('explorer.deleteDialog.descMultiple', { count: items.length })
      : items.length === 1
        ? t(items[0]?.isFolder ? 'explorer.deleteDialog.descFolder' : 'explorer.deleteDialog.descFile', {
            name: items[0].name,
          })
        : ''
  );
</script>

<ConfirmDialog
  bind:open
  {title}
  {description}
  confirmText={t('common.delete')}
  loadingText={t('explorer.deleteDialog.deleting')}
  cancelText={t('common.cancel')}
  variant="danger"
  icon={Trash2}
  {loading}
  {onConfirm}
  {onCancel}
>
  {#each boardFolders as folder (folder)}
    <label class="board-folder">
      <input type="checkbox" bind:checked={deleteBoardFolders} />
      {t('board.alsoDeleteFolder', { folder })}
    </label>
  {/each}
  {#if items.length > 1}
    <div class="confirm-dialog-item-list">
      {#each items.slice(0, 5) as item}
        <div class="confirm-dialog-item">
          • {item.name}{item.isFolder ? ` (${t('explorer.deleteDialog.folderTag')})` : ''}
        </div>
      {/each}
      {#if items.length > 5}
        <div class="confirm-dialog-item more">
          {t('explorer.deleteDialog.more', { count: items.length - 5 })}
        </div>
      {/if}
    </div>
  {/if}
</ConfirmDialog>

<style>
  .board-folder {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
    font-size: 12px;
  }

  .confirm-dialog-item-list {
    margin-top: 8px;
    padding: 8px 12px;
    background: var(--bg-secondary, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--border-secondary, rgba(255, 255, 255, 0.08));
    border-radius: 6px;
    font-size: 12px;
    max-height: 120px;
    overflow-y: auto;
  }

  .confirm-dialog-item {
    padding: 2px 0;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .confirm-dialog-item.more {
    font-style: italic;
    color: var(--text-muted);
  }
</style>
