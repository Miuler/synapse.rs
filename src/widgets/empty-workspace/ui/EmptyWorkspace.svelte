<script lang="ts">
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
  <svg
    class="empty-icon"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
  >
    <path
      d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
    />
    <polyline points="14 2 14 8 20 8" />
    <line x1="9" y1="15" x2="15" y2="15" />
  </svg>
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
    <button type="button" class="create-btn" onclick={onCreateNew}>
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
      >
        <line x1="12" y1="5" x2="12" y2="19" />
        <line x1="5" y1="12" x2="19" y2="12" />
      </svg>
      <span>Crear nuevo archivo</span>
    </button>
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

  .empty-icon {
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

  .create-btn {
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
    transition: all 0.15s ease;
  }

  .create-btn:hover {
    background-color: rgba(9, 105, 218, 0.15);
    border-color: var(--accent, #0969da);
  }
</style>
