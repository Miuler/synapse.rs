<script lang="ts">
  import { Trash2 } from 'lucide-svelte';
  import { ConfirmDialog } from '@shared/ui/confirm-dialog';

  export interface ItemToDelete {
    name: string;
    relativePath: string;
    isFolder: boolean;
  }

  interface Props {
    open: boolean;
    items: ItemToDelete[];
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
      ? `¿Eliminar ${items.length} elementos?`
      : `¿Eliminar ${items[0]?.isFolder ? 'carpeta' : 'archivo'}?`
  );

  let description = $derived(
    items.length > 1
      ? `¿Estás seguro de que deseas eliminar permanentemente estos ${items.length} elementos y todo su contenido? Esta acción no se puede deshacer.`
      : items.length === 1
        ? `¿Estás seguro de que deseas eliminar permanentemente "${items[0]?.name}"${items[0]?.isFolder ? ' y todo su contenido' : ''}? Esta acción no se puede deshacer.`
        : ''
  );
</script>

<ConfirmDialog
  bind:open
  {title}
  {description}
  confirmText="Eliminar"
  loadingText="Eliminando..."
  cancelText="Cancelar"
  variant="danger"
  icon={Trash2}
  {loading}
  {onConfirm}
  {onCancel}
>
  {#if items.length > 1}
    <div class="confirm-dialog-item-list">
      {#each items.slice(0, 5) as item}
        <div class="confirm-dialog-item">
          • {item.name} {item.isFolder ? '(carpeta)' : ''}
        </div>
      {/each}
      {#if items.length > 5}
        <div class="confirm-dialog-item more">
          ... y {items.length - 5} más
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
