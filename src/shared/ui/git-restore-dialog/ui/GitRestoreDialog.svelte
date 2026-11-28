<script lang="ts">
  import { RotateCcw } from 'lucide-svelte';
  import { t } from '@shared/lib/i18n';
  import { ConfirmDialog } from '@shared/ui/confirm-dialog';

  interface Props {
    open: boolean;
    items: string[];
    loading?: boolean;
    onConfirm: () => void | Promise<void>;
    onCancel: () => void;
  }

  let {
    open = $bindable(false),
    items = [],
    loading = false,
    onConfirm,
    onCancel,
  }: Props = $props();

  let title = $derived(
    items.length > 1
      ? t('gitRestore.titleMultiple', { count: items.length })
      : t('gitRestore.titleSingle', { name: items[0] || '' })
  );

  let description = $derived(
    items.length > 1
      ? t('gitRestore.descMultiple', { count: items.length })
      : t('gitRestore.descSingle', { name: items[0] || '' })
  );
</script>

<ConfirmDialog
  bind:open
  {title}
  {description}
  confirmText={t('gitRestore.confirmText')}
  loadingText={t('gitRestore.loadingText')}
  cancelText={t('common.cancel')}
  variant="danger"
  icon={RotateCcw}
  {loading}
  {onConfirm}
  {onCancel}
>
  {#if items.length > 1}
    <div class="confirm-dialog-item-list">
      {#each items.slice(0, 5) as path}
        <div class="confirm-dialog-item">
          • {path}
        </div>
      {/each}
      {#if items.length > 5}
        <div class="confirm-dialog-item more">
          {t('gitRestore.more', { count: items.length - 5 })}
        </div>
      {/if}
    </div>
  {/if}
</ConfirmDialog>

<style>
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
    color: var(--text-secondary, #656d76);
    line-height: 1.4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .confirm-dialog-item.more {
    font-style: italic;
    color: var(--text-muted, #8c959f);
  }
</style>
