<script lang="ts">
  import { RotateCcw } from 'lucide-svelte';
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
      ? `¿Restaurar ${items.length} elementos con Git?`
      : `¿Restaurar "${items[0] || ''}" con Git?`
  );

  let description = $derived(
    items.length > 1
      ? `¿Estás seguro de que deseas descartar las modificaciones locales no confirmadas en Git de estos ${items.length} elementos? Esta acción sobreescribirá los cambios no preparados en tu espacio de trabajo.`
      : `¿Estás seguro de que deseas descartar las modificaciones locales no confirmadas en Git de "${items[0] || ''}"? Esta acción sobreescribirá los cambios no preparados en tu espacio de trabajo.`
  );
</script>

<ConfirmDialog
  bind:open
  {title}
  {description}
  confirmText="Restaurar cambios"
  loadingText="Restaurando..."
  cancelText="Cancelar"
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
