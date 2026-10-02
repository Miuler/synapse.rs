<script lang="ts">
  import { type Snippet, type Component } from 'svelte';
  import { AlertDialog } from 'bits-ui';

  export interface ConfirmDialogProps {
    open?: boolean;
    title: string;
    description?: string;
    confirmText?: string;
    loadingText?: string;
    cancelText?: string;
    variant?: 'danger' | 'default';
    icon?: any;
    loading?: boolean;
    onConfirm: () => void | Promise<void>;
    onCancel?: () => void;
    children?: Snippet;
  }

  let {
    open = $bindable(false),
    title,
    description,
    confirmText = 'Confirmar',
    loadingText,
    cancelText = 'Cancelar',
    variant = 'default',
    icon: IconComponent,
    loading = false,
    onConfirm,
    onCancel,
    children,
  }: ConfirmDialogProps = $props();

  let cancelButtonEl = $state<HTMLButtonElement | null>(null);
  let confirmButtonEl = $state<HTMLButtonElement | null>(null);
  let dialogContentEl = $state<HTMLElement | null>(null);

  function focusCancelButton() {
    requestAnimationFrame(() => {
      if (cancelButtonEl) {
        cancelButtonEl.focus();
      } else {
        const btn = dialogContentEl?.querySelector<HTMLButtonElement>('.confirm-dialog-btn.cancel');
        btn?.focus();
      }
    });
  }

  $effect(() => {
    if (open) {
      focusCancelButton();
    }
  });

  function handleKeyDown(e: KeyboardEvent) {
    if (loading) return;

    const cancelBtn = cancelButtonEl || dialogContentEl?.querySelector<HTMLButtonElement>('.confirm-dialog-btn.cancel');
    const confirmBtn = confirmButtonEl || dialogContentEl?.querySelector<HTMLButtonElement>('.confirm-dialog-btn.confirm');

    if (e.key === 'ArrowRight' || (e.key === 'Tab' && !e.shiftKey)) {
      e.preventDefault();
      if (document.activeElement === cancelBtn) {
        confirmBtn?.focus();
      } else {
        cancelBtn?.focus();
      }
    } else if (e.key === 'ArrowLeft' || (e.key === 'Tab' && e.shiftKey)) {
      e.preventDefault();
      if (document.activeElement === confirmBtn) {
        cancelBtn?.focus();
      } else {
        confirmBtn?.focus();
      }
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      if (document.activeElement === cancelBtn) {
        confirmBtn?.focus();
      } else {
        cancelBtn?.focus();
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (document.activeElement === confirmBtn) {
        handleConfirm();
      } else {
        handleCancel();
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      handleCancel();
    }
  }

  function handleCancel() {
    if (loading) return;
    open = false;
    onCancel?.();
  }

  async function handleConfirm() {
    if (loading) return;
    await onConfirm();
  }
</script>

<AlertDialog.Root
  {open}
  onOpenChange={(isOpen) => {
    if (!isOpen && !loading) {
      handleCancel();
    }
  }}
>
  <AlertDialog.Portal>
    <AlertDialog.Overlay class="confirm-dialog-overlay" />
    <AlertDialog.Content
      bind:ref={dialogContentEl}
      class="confirm-dialog"
      onOpenAutoFocus={(e) => {
        e.preventDefault();
        focusCancelButton();
      }}
      onkeydown={handleKeyDown}
    >
      <div class="confirm-dialog-header">
        {#if IconComponent}
          <div class="confirm-dialog-icon-wrap {variant}">
            <IconComponent size={24} class="confirm-dialog-icon" />
          </div>
        {/if}
        <div class="confirm-dialog-text">
          <AlertDialog.Title class="confirm-dialog-title">
            {title}
          </AlertDialog.Title>
          {#if description}
            <AlertDialog.Description class="confirm-dialog-desc">
              {description}
            </AlertDialog.Description>
          {/if}
          {#if children}
            <div class="confirm-dialog-body">
              {@render children()}
            </div>
          {/if}
        </div>
      </div>
      <div class="confirm-dialog-actions">
        <AlertDialog.Cancel
          bind:ref={cancelButtonEl}
          class="confirm-dialog-btn cancel"
          disabled={loading}
          onclick={handleCancel}
        >
          {cancelText}
        </AlertDialog.Cancel>
        <AlertDialog.Action
          bind:ref={confirmButtonEl}
          class="confirm-dialog-btn confirm {variant}"
          onclick={(e) => {
            e.preventDefault();
            handleConfirm();
          }}
          disabled={loading}
        >
          {loading && loadingText ? loadingText : confirmText}
        </AlertDialog.Action>
      </div>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>

<style>
  /* Confirm Dialog Modal Overlay */
  :global(.confirm-dialog-overlay) {
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

  :global(.confirm-dialog) {
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 10px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.2);
    width: 90%;
    max-width: 420px;
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

  :global(.confirm-dialog .confirm-dialog-header) {
    display: flex;
    gap: 14px;
    align-items: flex-start;
  }

  :global(.confirm-dialog .confirm-dialog-icon-wrap) {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  :global(.confirm-dialog .confirm-dialog-icon-wrap.danger) {
    background: rgba(207, 34, 46, 0.12);
    color: #cf222e;
  }

  :global(.confirm-dialog .confirm-dialog-icon-wrap.default) {
    background: rgba(9, 105, 218, 0.12);
    color: var(--accent, #0969da);
  }

  :global(.confirm-dialog .confirm-dialog-icon) {
    width: 18px;
    height: 18px;
  }

  :global(.confirm-dialog .confirm-dialog-text) {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  :global(.confirm-dialog .confirm-dialog-title) {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
    margin: 0;
    line-height: 1.25;
  }

  :global(.confirm-dialog .confirm-dialog-desc) {
    font-size: 13px;
    color: var(--text-secondary, #656d76);
    line-height: 1.45;
    margin: 0;
    word-break: break-word;
  }

  :global(.confirm-dialog .confirm-dialog-desc strong) {
    color: var(--text-primary, #1f2328);
  }

  :global(.confirm-dialog .confirm-dialog-body) {
    margin-top: 4px;
  }

  :global(.confirm-dialog .confirm-dialog-item-list) {
    margin-top: 10px;
    padding: 8px 12px;
    background-color: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    max-height: 120px;
    overflow-y: auto;
    font-size: 12px;
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  :global(.confirm-dialog .confirm-dialog-item) {
    color: var(--text-primary, #1f2328);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.confirm-dialog .confirm-dialog-item.more) {
    color: var(--text-secondary, #656d76);
    font-style: italic;
  }

  :global(.confirm-dialog .confirm-dialog-actions) {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 4px;
  }

  :global(.confirm-dialog .confirm-dialog-btn) {
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

  :global(.confirm-dialog .confirm-dialog-btn.cancel) {
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-primary, #1f2328);
    border-color: var(--border-primary, #d0d7de);
  }

  :global(.confirm-dialog .confirm-dialog-btn.cancel:hover:not(:disabled)) {
    background: var(--border-primary, #d0d7de);
  }

  :global(.confirm-dialog .confirm-dialog-btn.cancel:focus),
  :global(.confirm-dialog .confirm-dialog-btn.cancel:focus-visible) {
    outline: 2px solid var(--accent, #0969da);
    outline-offset: 2px;
    border-color: var(--accent, #0969da);
  }

  :global(.confirm-dialog .confirm-dialog-btn.confirm.danger) {
    background: #cf222e;
    color: #ffffff;
    border-color: #a40e26;
  }

  :global(.confirm-dialog .confirm-dialog-btn.confirm.danger:hover:not(:disabled)) {
    background: #a40e26;
  }

  :global(.confirm-dialog .confirm-dialog-btn.confirm.danger:focus),
  :global(.confirm-dialog .confirm-dialog-btn.confirm.danger:focus-visible) {
    outline: 2px solid #cf222e;
    outline-offset: 2px;
    border-color: #a40e26;
  }

  :global(.confirm-dialog .confirm-dialog-btn.confirm.default) {
    background: var(--accent, #0969da);
    color: #ffffff;
    border-color: var(--accent, #0969da);
  }

  :global(.confirm-dialog .confirm-dialog-btn.confirm.default:hover:not(:disabled)) {
    filter: brightness(0.9);
  }

  :global(.confirm-dialog .confirm-dialog-btn.confirm.default:focus),
  :global(.confirm-dialog .confirm-dialog-btn.confirm.default:focus-visible) {
    outline: 2px solid var(--accent, #0969da);
    outline-offset: 2px;
  }

  :global(.confirm-dialog .confirm-dialog-btn:focus:not(:focus-visible):active) {
    outline: none;
  }

  :global(.confirm-dialog .confirm-dialog-btn:disabled) {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
