<!--
  Diálogo modal redimensionable: marco, cabecera (icono, título, insignia, descripción y cerrar),
  cuerpo y pie opcional. El tamaño se cambia arrastrando bordes o esquinas y se recuerda por `sizeKey`.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Dialog } from 'bits-ui';
  import { X } from 'lucide-svelte';
  import DialogResizeHandles from './DialogResizeHandles.svelte';
  import { persistedDialogSize, type DialogSize } from '../lib/dialog-size.svelte';

  interface Props {
    open: boolean;
    /** Se llama cuando el usuario pide cerrar (Esc, clic fuera o botón de cerrar). */
    onClose: () => void;
    /** Impide cerrar mientras hay una operación en curso. */
    busy?: boolean;
    title: string;
    description?: string;
    /** Clave de localStorage donde se guarda el tamaño. */
    sizeKey: string;
    defaultSize: DialogSize;
    minSize: DialogSize;
    /** Color del icono de la cabecera (variable CSS). */
    tone?: string;
    /** Clase extra del contenedor, para estilos propios del diálogo. */
    class?: string;
    /** Relleno del cuerpo; `0` para cuerpos con secciones a todo el ancho. */
    bodyPadding?: string;
    onkeydown?: (e: KeyboardEvent) => void;
    onOpenAutoFocus?: (e: Event) => void;
    icon: Snippet;
    badge?: Snippet;
    children: Snippet;
    footer?: Snippet;
  }

  let {
    open,
    onClose,
    busy = false,
    title,
    description,
    sizeKey,
    defaultSize,
    minSize,
    tone = 'var(--accent, #0969da)',
    class: extraClass = '',
    bodyPadding = '16px 20px',
    onkeydown,
    onOpenAutoFocus,
    icon,
    badge,
    children,
    footer,
  }: Props = $props();

  // svelte-ignore state_referenced_locally
  const dialogSize = persistedDialogSize(sizeKey, defaultSize, minSize);

  function requestClose() {
    if (!busy) onClose();
  }
</script>

<Dialog.Root
  {open}
  onOpenChange={(isOpen) => {
    if (!isOpen) requestClose();
  }}
>
  <Dialog.Portal>
    <Dialog.Overlay class="resizable-dialog-overlay" />
    <Dialog.Content
      class="resizable-dialog synapse-dialog {extraClass}"
      style="{dialogSize.style} --dialog-tone: {tone};"
      {onkeydown}
      {onOpenAutoFocus}
    >
      <DialogResizeHandles onStart={dialogSize.startResize} />

      <div class="resizable-dialog-header">
        <div class="resizable-dialog-icon">{@render icon()}</div>
        <div class="resizable-dialog-titles">
          <div class="resizable-dialog-title-row">
            <Dialog.Title class="resizable-dialog-title">{title}</Dialog.Title>
            {@render badge?.()}
          </div>
          {#if description}
            <Dialog.Description class="resizable-dialog-desc">{description}</Dialog.Description>
          {/if}
        </div>
        <button
          type="button"
          class="resizable-dialog-close"
          onclick={requestClose}
          disabled={busy}
          aria-label="Cerrar diálogo"
          title="Cerrar (Esc)"
        >
          <X size={16} />
        </button>
      </div>

      <div class="resizable-dialog-body" style="padding: {bodyPadding};">
        {@render children()}
      </div>

      {#if footer}
        <div class="resizable-dialog-footer">{@render footer()}</div>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.resizable-dialog-overlay) {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    backdrop-filter: blur(2px);
    z-index: 10001;
    animation: resizable-dialog-fade 0.15s ease-out;
  }

  :global(.resizable-dialog) {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    background: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 10px;
    box-shadow: var(--modal-shadow, 0 12px 32px rgba(0, 0, 0, 0.25));
    display: flex;
    flex-direction: column;
    z-index: 10002;
    outline: none;
    overflow: hidden;
    box-sizing: border-box;
    animation: resizable-dialog-in 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes resizable-dialog-fade {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes resizable-dialog-in {
    from {
      opacity: 0;
      transform: translate(-50%, -48%) scale(0.97);
    }
    to {
      opacity: 1;
      transform: translate(-50%, -50%) scale(1);
    }
  }

  .resizable-dialog-header {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 16px 20px 14px;
    border-bottom: 1px solid var(--border-secondary, #e1e4e8);
    background: var(--bg-secondary, #f6f8fa);
    flex-shrink: 0;
  }

  .resizable-dialog-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--dialog-tone) 12%, transparent);
    color: var(--dialog-tone);
    flex-shrink: 0;
  }

  .resizable-dialog-titles {
    flex: 1;
    min-width: 0;
  }

  .resizable-dialog-title-row {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  :global(.resizable-dialog-title) {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
    margin: 0;
  }

  :global(.resizable-dialog-desc) {
    font-size: 12px;
    color: var(--text-secondary, #656d76);
    margin: 3px 0 0;
  }

  .resizable-dialog-close {
    border: none;
    background: none;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    padding: 6px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .resizable-dialog-close:hover:not(:disabled) {
    background: var(--bg-tertiary, #eaeef2);
    color: var(--text-primary, #1f2328);
  }

  .resizable-dialog-body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
    overflow: auto;
    box-sizing: border-box;
  }

  .resizable-dialog-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 10px 20px;
    border-top: 1px solid var(--border-secondary, #e1e4e8);
    background: var(--bg-secondary, #f6f8fa);
    flex-shrink: 0;
  }
</style>
