<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { GitCommitHorizontal, GitBranch, AlertCircle, Loader2 } from 'lucide-svelte';

  export interface GitCommitDialogProps {
    open?: boolean;
    paths: string[];
    branchName?: string | null;
    loading?: boolean;
    errorMessage?: string | null;
    onConfirm: (message: string) => void | Promise<void>;
    onCancel?: () => void;
  }

  let {
    open = $bindable(false),
    paths = [],
    branchName = null,
    loading = false,
    errorMessage = null,
    onConfirm,
    onCancel,
  }: GitCommitDialogProps = $props();

  let message = $state('');
  let textareaEl = $state<HTMLTextAreaElement | null>(null);

  // Auto-enfoque y reseteo al abrir el diálogo
  $effect(() => {
    if (open) {
      message = '';
      requestAnimationFrame(() => {
        textareaEl?.focus();
      });
    }
  });

  const canSubmit = $derived(!loading && message.trim().length > 0 && paths.length > 0);

  function handleCancel() {
    if (loading) return;
    open = false;
    onCancel?.();
  }

  async function handleSubmit() {
    if (!canSubmit) return;
    await onConfirm(message.trim());
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      handleCancel();
    } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      if (canSubmit) {
        handleSubmit();
      }
    }
  }
</script>

<Dialog.Root
  {open}
  onOpenChange={(isOpen) => {
    if (!isOpen && !loading) {
      handleCancel();
    }
  }}
>
  <Dialog.Portal>
    <Dialog.Overlay class="git-commit-dialog-overlay" />
    <Dialog.Content
      class="git-commit-dialog"
      onOpenAutoFocus={(e) => {
        e.preventDefault();
        requestAnimationFrame(() => textareaEl?.focus());
      }}
      onkeydown={handleKeyDown}
    >
      <!-- Cabecera -->
      <div class="git-commit-dialog-header">
        <div class="git-commit-dialog-icon-wrap">
          <GitCommitHorizontal size={20} class="git-commit-dialog-icon" />
        </div>
        <div class="git-commit-dialog-titles">
          <div class="git-commit-dialog-title-row">
            <Dialog.Title class="git-commit-dialog-title">
              Hacer commit en Git
            </Dialog.Title>
            {#if branchName}
              <span class="git-commit-branch-badge" title="Rama actual: {branchName}">
                <GitBranch size={11} />
                <span>{branchName}</span>
              </span>
            {/if}
          </div>
          <Dialog.Description class="git-commit-dialog-desc">
            Confirma los cambios de los archivos seleccionados en el historial de Git.
          </Dialog.Description>
        </div>
      </div>

      <!-- Lista de archivos seleccionados -->
      <div class="git-commit-files-box">
        <div class="git-commit-files-header">
          <span>Archivos seleccionados ({paths.length}):</span>
        </div>
        <div class="git-commit-files-list">
          {#each paths.slice(0, 5) as p}
            <div class="git-commit-file-item" title={p}>
              <span class="git-commit-bullet">•</span>
              <span class="git-commit-filename">{p}</span>
            </div>
          {/each}
          {#if paths.length > 5}
            <div class="git-commit-file-more">
              ... y {paths.length - 5} más
            </div>
          {/if}
        </div>
      </div>

      <!-- Campo para el mensaje de commit -->
      <div class="git-commit-input-section">
        <label for="git-commit-message-input" class="git-commit-input-label">
          Mensaje del commit <span class="required">*</span>
        </label>
        <textarea
          id="git-commit-message-input"
          bind:this={textareaEl}
          bind:value={message}
          class="git-commit-textarea"
          rows="3"
          placeholder="Escribe un mensaje descriptivo para el commit..."
          disabled={loading}
        ></textarea>
        <div class="git-commit-hint">
          <span>Presiona <strong>Ctrl+Enter</strong> para confirmar directamente</span>
        </div>
      </div>

      <!-- Mensaje de error (si ocurre alguno) -->
      {#if errorMessage}
        <div class="git-commit-error-box" role="alert">
          <AlertCircle size={15} class="git-commit-error-icon" />
          <span class="git-commit-error-text">{errorMessage}</span>
        </div>
      {/if}

      <!-- Botones de acción -->
      <div class="git-commit-dialog-actions">
        <button
          type="button"
          class="git-commit-btn cancel"
          onclick={handleCancel}
          disabled={loading}
        >
          Cancelar
        </button>
        <button
          type="button"
          class="git-commit-btn submit"
          onclick={handleSubmit}
          disabled={!canSubmit}
        >
          {#if loading}
            <Loader2 size={14} class="spin-icon" />
            <span>Haciendo commit...</span>
          {:else}
            <GitCommitHorizontal size={14} />
            <span>Hacer commit</span>
          {/if}
        </button>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.git-commit-dialog-overlay) {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10001;
    animation: overlay-fade-in 0.15s ease-out;
  }

  @keyframes overlay-fade-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  :global(.git-commit-dialog) {
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 10px;
    box-shadow: var(--modal-shadow, 0 12px 32px rgba(0, 0, 0, 0.25));
    width: 90%;
    max-width: 460px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    animation: modal-scale-in 0.15s cubic-bezier(0.16, 1, 0.3, 1);
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 10002;
    outline: none;
    box-sizing: border-box;
  }

  @keyframes modal-scale-in {
    from {
      opacity: 0;
      transform: translate(-50%, -50%) scale(0.95);
    }
    to {
      opacity: 1;
      transform: translate(-50%, -50%) scale(1);
    }
  }

  .git-commit-dialog-header {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }

  .git-commit-dialog-icon-wrap {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background: rgba(46, 160, 67, 0.15);
    color: #2ea043;
  }

  .git-commit-dialog-titles {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .git-commit-dialog-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  :global(.git-commit-dialog-title) {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
    margin: 0;
    line-height: 1.25;
  }

  .git-commit-branch-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 7px;
    font-size: 11px;
    font-weight: 500;
    border-radius: 12px;
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    color: var(--text-secondary, #656d76);
  }

  :global(.git-commit-dialog-desc) {
    font-size: 13px;
    color: var(--text-secondary, #656d76);
    line-height: 1.4;
    margin: 0;
  }

  /* Cuadro de archivos */
  .git-commit-files-box {
    background-color: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    padding: 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .git-commit-files-header {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary, #656d76);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .git-commit-files-list {
    max-height: 105px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
  }

  .git-commit-file-item {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-primary, #1f2328);
    overflow: hidden;
  }

  .git-commit-bullet {
    color: #2ea043;
    font-size: 14px;
    line-height: 1;
  }

  .git-commit-filename {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: monospace;
    font-size: 11.5px;
  }

  .git-commit-file-more {
    font-size: 11.5px;
    color: var(--text-secondary, #656d76);
    font-style: italic;
    padding-left: 10px;
  }

  /* Sección de entrada */
  .git-commit-input-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .git-commit-input-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary, #1f2328);
  }

  .git-commit-input-label .required {
    color: #cf222e;
  }

  .git-commit-textarea {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    padding: 8px 10px;
    font-family: inherit;
    font-size: 13px;
    color: var(--text-primary, #1f2328);
    line-height: 1.45;
    resize: vertical;
    min-height: 72px;
    max-height: 180px;
    outline: none;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .git-commit-textarea:focus {
    border-color: var(--accent, #0969da);
    box-shadow: 0 0 0 3px rgba(9, 105, 218, 0.18);
  }

  .git-commit-textarea:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .git-commit-hint {
    display: flex;
    justify-content: flex-end;
    font-size: 11px;
    color: var(--text-secondary, #656d76);
  }

  .git-commit-hint strong {
    font-weight: 600;
  }

  /* Caja de error */
  .git-commit-error-box {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 12px;
    border-radius: 6px;
    background: rgba(207, 34, 46, 0.1);
    border: 1px solid rgba(207, 34, 46, 0.35);
    color: #cf222e;
    font-size: 12px;
    line-height: 1.4;
  }

  :global(.git-commit-error-icon) {
    flex-shrink: 0;
    margin-top: 1px;
  }

  .git-commit-error-text {
    word-break: break-word;
  }

  /* Botones de acción */
  .git-commit-dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 2px;
  }

  .git-commit-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 14px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    border: 1px solid transparent;
    font-family: inherit;
    outline: none;
  }

  .git-commit-btn.cancel {
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-primary, #1f2328);
    border-color: var(--border-primary, #d0d7de);
  }

  .git-commit-btn.cancel:hover:not(:disabled) {
    background: var(--border-primary, #d0d7de);
  }

  .git-commit-btn.submit {
    background: #2ea043;
    color: #ffffff;
    border-color: rgba(27, 31, 36, 0.15);
  }

  .git-commit-btn.submit:hover:not(:disabled) {
    background: #2c974b;
  }

  .git-commit-btn.submit:focus-visible {
    outline: 2px solid #2ea043;
    outline-offset: 2px;
  }

  .git-commit-btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  :global(.spin-icon) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>
