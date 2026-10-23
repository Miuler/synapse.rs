<script lang="ts">
  import { AlertDialog } from "bits-ui";
  import { Lock, AlertTriangle } from "lucide-svelte";

  export interface UnsavedChangesDialogProps {
    open?: boolean;
    title?: string;
    files: string[];
    isWindowClose?: boolean;
    loading?: boolean;
    onAbort: () => void;
    onDiscard: () => void;
    onForceSave: () => void | Promise<void>;
  }

  let {
    open = $bindable(false),
    title,
    files = [],
    isWindowClose = false,
    loading = false,
    onAbort,
    onDiscard,
    onForceSave,
  }: UnsavedChangesDialogProps = $props();

  let abortButtonEl = $state<HTMLButtonElement | null>(null);
  let discardButtonEl = $state<HTMLButtonElement | null>(null);
  let forceSaveButtonEl = $state<HTMLButtonElement | null>(null);
  let dialogContentEl = $state<HTMLElement | null>(null);

  function focusDefaultButton() {
    requestAnimationFrame(() => {
      if (abortButtonEl) {
        abortButtonEl.focus();
      }
    });
  }

  $effect(() => {
    if (open) {
      focusDefaultButton();
    }
  });

  function handleKeyDown(e: KeyboardEvent) {
    if (loading) return;

    const buttons = [abortButtonEl, discardButtonEl, forceSaveButtonEl].filter(
      Boolean,
    ) as HTMLButtonElement[];
    const currentIndex = buttons.indexOf(
      document.activeElement as HTMLButtonElement,
    );

    if (e.key === "ArrowRight" || (e.key === "Tab" && !e.shiftKey)) {
      e.preventDefault();
      const nextIndex = (currentIndex + 1) % buttons.length;
      buttons[nextIndex]?.focus();
    } else if (e.key === "ArrowLeft" || (e.key === "Tab" && e.shiftKey)) {
      e.preventDefault();
      const prevIndex = (currentIndex - 1 + buttons.length) % buttons.length;
      buttons[prevIndex]?.focus();
    } else if (e.key === "Escape") {
      e.preventDefault();
      handleAbort();
    }
  }

  function handleAbort() {
    if (loading) return;
    open = false;
    onAbort();
  }

  function handleDiscard() {
    if (loading) return;
    open = false;
    onDiscard();
  }

  async function handleForceSave() {
    if (loading) return;
    await onForceSave();
  }

  let computedTitle = $derived(
    title ||
      (isWindowClose
        ? "Cambios sin guardar al cerrar la aplicación"
        : "Cambios sin persistir en archivo bloqueado"),
  );

  let singleFileName = $derived(
    files.length === 1 ? files[0].split("/").pop() || files[0] : "",
  );
</script>

<AlertDialog.Root
  {open}
  onOpenChange={(isOpen) => {
    if (!isOpen && !loading) {
      handleAbort();
    }
  }}
>
  <AlertDialog.Portal>
    <AlertDialog.Overlay class="confirm-dialog-overlay" />
    <AlertDialog.Content
      bind:ref={dialogContentEl}
      class="confirm-dialog unsaved-changes-dialog"
      onOpenAutoFocus={(e) => {
        e.preventDefault();
        focusDefaultButton();
      }}
      onkeydown={handleKeyDown}
    >
      <div class="confirm-dialog-header">
        <div class="confirm-dialog-icon-wrap warning">
          <Lock size={20} class="confirm-dialog-icon" />
        </div>
        <div class="confirm-dialog-text">
          <AlertDialog.Title class="confirm-dialog-title">
            {computedTitle}
          </AlertDialog.Title>
          <AlertDialog.Description class="confirm-dialog-desc">
            {#if isWindowClose}
              Hay <strong>{files.length}</strong>
              {files.length === 1
                ? "archivo bloqueado con cambios sin persistir"
                : "archivos bloqueados con cambios sin persistir"}. Si cierras
              la aplicación sin guardar, se perderán las modificaciones.
            {:else}
              El archivo <strong>{singleFileName}</strong> está
              <strong>bloqueado</strong> y tiene cambios que no han sido guardados
              en disco.
            {/if}
          </AlertDialog.Description>

          {#if files.length > 1}
            <div class="confirm-dialog-item-list">
              {#each files as f}
                <div class="confirm-dialog-item" title={f}>
                  • {f.split("/").pop() || f}
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>

      <div class="confirm-dialog-actions unsaved-actions">
        <button
          type="button"
          bind:this={abortButtonEl}
          class="confirm-dialog-btn cancel"
          disabled={loading}
          onclick={handleAbort}
        >
          Seguir Editando
        </button>
        <button
          type="button"
          bind:this={discardButtonEl}
          class="confirm-dialog-btn discard danger"
          disabled={loading}
          onclick={handleDiscard}
        >
          Descartar cambios
        </button>
        <button
          type="button"
          bind:this={forceSaveButtonEl}
          class="confirm-dialog-btn save primary"
          disabled={loading}
          onclick={handleForceSave}
        >
          {loading ? "Guardando..." : "Forzar el guardado"}
        </button>
      </div>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>

<style>
  :global(.unsaved-changes-dialog) {
    max-width: 520px !important;
  }

  :global(.confirm-dialog .confirm-dialog-icon-wrap.warning) {
    background: rgba(210, 153, 34, 0.15);
    color: #d29922;
  }

  .unsaved-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 12px;
  }

  .confirm-dialog-btn.primary {
    background: var(--accent, #0969da);
    color: #ffffff;
    border-color: var(--accent, #0969da);
  }

  .confirm-dialog-btn.primary:hover:not(:disabled) {
    opacity: 0.92;
  }

  .confirm-dialog-btn.primary:focus,
  .confirm-dialog-btn.primary:focus-visible {
    outline: 2px solid var(--accent, #0969da);
    outline-offset: 2px;
  }
</style>
