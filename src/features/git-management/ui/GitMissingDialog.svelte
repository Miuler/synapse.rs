<script lang="ts">
  // La bóveda es un repositorio pero no se encuentra Git: permite elegir el ejecutable (se guarda en
  // config.jsonc → git.path). Ver el estado, el historial y las versiones funciona sin Git; hace
  // falta para commit, cambiar de rama, push, clone y LFS.
  import { ResizableDialog } from '@shared/ui/dialog-resize';
  import { gitBinary } from '@entities/git';
  import { FolderOpen, GitBranch } from 'lucide-svelte';

  interface Props {
    open: boolean;
  }
  let { open = $bindable(false) }: Props = $props();

  function close(remember: boolean) {
    if (remember) gitBinary.dismiss();
    open = false;
  }

  async function choose() {
    if (await gitBinary.choose()) open = false;
  }
</script>

<ResizableDialog
  {open}
  onClose={() => close(false)}
  busy={gitBinary.busy}
  title="No se encontró Git"
  description="La bóveda es un repositorio Git, pero Synapse no encuentra el programa git."
  sizeKey="synapse.gitMissingDialog.size"
  defaultSize={{ w: 520, h: 330 }}
  minSize={{ w: 380, h: 260 }}
  tone="var(--warning)"
  class="git-missing-dialog"
>
  {#snippet icon()}<GitBranch size={20} />{/snippet}
  <p>
    Puedes ver el estado, el historial y las versiones sin él, pero hace falta para hacer commit,
    cambiar de rama, hacer push, clonar y usar Git LFS.
  </p>
  <p>
    Si lo tienes instalado fuera del PATH, elige el ejecutable (por ejemplo
    <code>C:\Program Files\Git\cmd\git.exe</code>). Si no, instálalo desde
    <a href="https://git-scm.com/downloads" target="_blank" rel="noopener noreferrer">git-scm.com</a>.
    La ruta se guarda en la configuración y también se puede cambiar en Ajustes → Git.
  </p>
  {#if gitBinary.error}
    <p class="git-missing-error" role="alert">{gitBinary.error}</p>
  {/if}
  {#snippet footer()}
    <button type="button" class="git-missing-btn" onclick={() => close(true)} disabled={gitBinary.busy}>No volver a preguntar</button>
    <button type="button" class="git-missing-btn" onclick={() => close(false)} disabled={gitBinary.busy}>Ahora no</button>
    <button type="button" class="git-missing-btn primary" onclick={choose} disabled={gitBinary.busy}>
      <FolderOpen size={14} /> Elegir ejecutable…
    </button>
  {/snippet}
</ResizableDialog>

<style>
  p {
    margin: 0 0 10px;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-primary);
  }
  code {
    font-size: 12px;
  }
  .git-missing-error {
    color: var(--danger);
  }
  .git-missing-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 14px;
    border-radius: 6px;
    border: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    color: var(--text-primary);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .git-missing-btn.primary {
    background: var(--accent);
    color: var(--on-accent);
    border-color: transparent;
  }
  .git-missing-btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
</style>
