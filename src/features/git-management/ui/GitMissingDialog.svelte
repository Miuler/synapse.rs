<script lang="ts">
  // La bóveda es un repositorio pero no se encuentra Git: permite elegir el ejecutable (se guarda en
  // config.jsonc → git.path). Ver el estado, el historial y las versiones funciona sin Git; hace
  // falta para commit, cambiar de rama, push, clone y LFS.
  import { ResizableDialog } from '@shared/ui/dialog-resize';
  import { t } from '@shared/lib/i18n';
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
  title={t('gitMissing.title')}
  description={t('gitMissing.description')}
  sizeKey="synapse.gitMissingDialog.size"
  defaultSize={{ w: 520, h: 330 }}
  minSize={{ w: 380, h: 260 }}
  tone="var(--warning)"
  class="git-missing-dialog"
>
  {#snippet icon()}<GitBranch size={20} />{/snippet}
  <p>
    {t('gitMissing.body1')}
  </p>
  <p>
    {t('gitMissing.body2Part1')}
    <code>C:\Program Files\Git\cmd\git.exe</code>{t('gitMissing.body2Part2')}
    <a href="https://git-scm.com/downloads" target="_blank" rel="noopener noreferrer">git-scm.com</a>{t('gitMissing.body2Part3')}
  </p>
  {#if gitBinary.error}
    <p class="git-missing-error" role="alert">{gitBinary.error}</p>
  {/if}
  {#snippet footer()}
    <button type="button" class="git-missing-btn" onclick={() => close(true)} disabled={gitBinary.busy}>{t('gitMissing.dontAskAgain')}</button>
    <button type="button" class="git-missing-btn" onclick={() => close(false)} disabled={gitBinary.busy}>{t('gitMissing.notNow')}</button>
    <button type="button" class="git-missing-btn primary" onclick={choose} disabled={gitBinary.busy}>
      <FolderOpen size={14} /> {t('gitMissing.chooseExecutable')}
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
