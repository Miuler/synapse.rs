<script lang="ts">
  import { Button } from 'bits-ui';
  import { FileText, Plus } from 'lucide-svelte';

  interface Props {
    hasVaultItems?: boolean;
    onCreateNew?: () => void;
  }

  let {
    hasVaultItems = true,
    onCreateNew
  }: Props = $props();
</script>

<div class="empty-workspace">
  <FileText
    class="empty-icon"
    size={56}
    strokeWidth={1.5}
  />
  <h2>
    {!hasVaultItems
      ? "No hay archivos en la bóveda"
      : "Ningún archivo abierto"}
  </h2>
  <p>
    {!hasVaultItems
      ? "Crea un nuevo archivo para comenzar a escribir."
      : "Selecciona un archivo del panel lateral para abrirlo."}
  </p>
  {#if onCreateNew}
    <Button.Root type="button" class="create-btn" onclick={onCreateNew}>
      <Plus size={16} />
      <span>Crear nuevo archivo</span>
    </Button.Root>
  {/if}
</div>

<style>
  .empty-workspace {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    width: 100%;
    gap: 12px;
    color: var(--text-secondary, #656d76);
    text-align: center;
    user-select: none;
  }

  :global(.empty-icon) {
    width: 56px;
    height: 56px;
    color: var(--border-primary, #d0d7de);
    margin-bottom: 8px;
  }

  .empty-workspace h2 {
    font-size: 20px;
    font-weight: 500;
    color: var(--text-primary, #1f2328);
    margin: 0;
  }

  .empty-workspace p {
    font-size: 14px;
    color: var(--text-secondary, #656d76);
    margin: 0;
  }

  :global(.create-btn) {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
    padding: 8px 16px;
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.1));
    color: var(--accent, #0969da);
    border: 1px solid var(--accent-border, rgba(9, 105, 218, 0.3));
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  }

  :global(.create-btn:hover) {
    background-color: var(--accent, #0969da);
    color: #ffffff;
    border-color: var(--accent, #0969da);
    box-shadow: 0 4px 12px rgba(9, 105, 218, 0.2);
  }
</style>
