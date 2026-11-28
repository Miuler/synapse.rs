<script lang="ts">
  import { onMount } from 'svelte';
  import { reasonText, t } from '@shared/lib/i18n';
  import { azureToken, type AzureTokenSetting } from '../api/azure-devops.api';
  import type { IssueError } from '../api/issues.api';

  interface Props {
    /** Guarda al salir del campo; sin esto, guarda Enter o quien llame a `save`. */
    autosave?: boolean;
  }

  let { autosave = false }: Props = $props();

  let setting = $state<AzureTokenSetting>({ reference: null, stored: false });
  let draft = $state('');
  let error = $state('');
  // Una referencia se ve; cualquier otra cosa puede ser un token y se oculta
  let secret = $derived(!['op://', '"op://'].some((p) => draft.startsWith(p) || p.startsWith(draft)));

  function show(next: AzureTokenSetting) {
    setting = next;
    draft = next.reference ?? '';
  }

  function fail(cause: unknown) {
    const e = cause as Partial<IssueError>;
    error = e.kind ? t(`issues.errors.${e.kind}`, { provider: 'Azure DevOps', detail: reasonText(e.detail ?? '') }) : String(cause);
  }

  onMount(() => {
    azureToken.get().then(show).catch(fail);
  });

  async function store(value: string): Promise<boolean> {
    error = '';
    try {
      show(await azureToken.set(value));
      return true;
    } catch (cause) {
      fail(cause);
      return false;
    }
  }

  /** Guarda lo escrito, si cambió. Un campo vacío no borra el token del llavero: para eso está «Quitar». */
  export async function save(): Promise<boolean> {
    const value = draft.trim();
    return value === (setting.reference ?? '') ? true : store(value);
  }
</script>

<div class="azure-token">
  <label for="azure-token">{t('azure.token')}</label>
  <div class="field">
    <input
      id="azure-token"
      class="azure-token-input"
      type={secret ? 'password' : 'text'}
      spellcheck="false"
      autocomplete="off"
      placeholder={setting.stored ? t('azure.tokenStored') : t('azure.tokenPlaceholder')}
      value={draft}
      oninput={(e) => (draft = e.currentTarget.value)}
      onkeydown={(e) => {
        if (e.key !== 'Enter') return;
        e.preventDefault();
        void save();
      }}
      onblur={() => autosave && void save()}
    />
    {#if setting.stored}
      <button type="button" class="azure-token-remove" onclick={() => store('')}>{t('azure.tokenRemove')}</button>
    {/if}
  </div>
  <small>{t('azure.tokenHint')}</small>
  {#if error}<small class="error">{error}</small>{/if}
</div>

<style>
  .azure-token {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
  }

  .field {
    display: flex;
    gap: 8px;
  }

  input,
  button {
    padding: 6px 8px;
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    font: inherit;
    color: var(--text-primary);
  }

  input {
    flex: 1;
    min-width: 0;
    background: var(--bg-secondary);
  }

  button {
    background: var(--bg-primary);
    cursor: pointer;
  }

  small {
    color: var(--text-muted);
  }

  .error {
    color: var(--danger, #cf222e);
    overflow-wrap: anywhere;
  }
</style>
