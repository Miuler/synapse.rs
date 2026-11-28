<!--
  Conexiones a gestores de issues: varias, también del mismo proveedor, cada una con su servidor y su
  token. El formulario sale de los campos que declara el proveedor. Sin ninguna, queda el token único.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { Plus, Pencil, Trash2, PlugZap } from 'lucide-svelte';
  import { reasonText, t } from '@shared/lib/i18n';
  import { ConfirmDialog } from '@shared/ui/confirm-dialog';
  import AzureTokenField from './AzureTokenField.svelte';
  import { azureToken } from '../api/azure-devops.api';
  import type { IssueError } from '../api/issues.api';
  import { connectionsApi, type Connection, type ConnectionProvider } from '../api/connections.api';

  /** El único proveedor con credencial anterior a las conexiones. */
  const LEGACY_PROVIDER = 'azure_devops';

  interface Form {
    id?: string;
    provider: string;
    name: string;
    server: string;
    user: string;
    credential: string;
    useGlobal: boolean;
  }

  let providers = $state<ConnectionProvider[]>([]);
  let connections = $state<Connection[] | null>(null);
  /** Hay un token único (llavero o `op://`) que se puede copiar a una conexión. */
  let hasGlobal = $state(false);
  let form = $state<Form | null>(null);
  let results = $state<Record<string, { ok: boolean; text: string }>>({});
  let removing = $state<Connection | null>(null);
  let error = $state('');
  let busy = $state(false);

  let provider = $derived(providers.find((p) => p.id === form?.provider));
  let previous = $derived(connections?.find((c) => c.id === form?.id));
  let canCopyGlobal = $derived(!!form && !form.id && form.provider === LEGACY_PROVIDER && hasGlobal);
  let legacyOnly = $derived(!!connections && !connections.some((c) => c.provider === LEGACY_PROVIDER));
  // Una referencia se ve; cualquier otra cosa puede ser un token y se oculta
  let secret = $derived(!['op://', '"op://'].some((p) => form?.credential.startsWith(p) || p.startsWith(form?.credential ?? '')));

  const providerName = (id: string) => providers.find((p) => p.id === id)?.name ?? id;

  function errorText(cause: unknown, id = form?.provider ?? ''): string {
    const e = cause as Partial<IssueError>;
    if (!e?.kind) return String(cause);
    return t(`issues.errors.${e.kind}`, { provider: providerName(id), detail: reasonText(e.detail ?? '') });
  }

  async function load() {
    const token = await azureToken.get().catch(() => null);
    hasGlobal = !!token && (!!token.reference || token.stored);
    connections = await connectionsApi.list();
  }

  onMount(() => {
    Promise.all([connectionsApi.providers().then((p) => (providers = p)), load()]).catch(
      (cause) => (error = errorText(cause)),
    );
  });

  function create(useGlobal = false) {
    error = '';
    form = {
      provider: useGlobal ? LEGACY_PROVIDER : (providers[0]?.id ?? ''),
      name: useGlobal ? providerName(LEGACY_PROVIDER) : '',
      server: useGlobal ? 'https://dev.azure.com/' : '',
      user: '',
      credential: '',
      useGlobal,
    };
  }

  function edit(connection: Connection) {
    error = '';
    form = {
      id: connection.id,
      provider: connection.provider,
      name: connection.name,
      server: connection.server,
      user: connection.user ?? '',
      credential: connection.reference ?? '',
      useGlobal: false,
    };
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!form || busy) return;
    busy = true;
    error = '';
    try {
      const credential = form.credential.trim();
      // Sin escribir nada nuevo, la credencial se queda como está; vaciar una referencia la quita
      const unchanged = credential === (previous?.reference ?? '');
      const saved = await connectionsApi.save({
        id: form.id ?? null,
        provider: form.provider,
        name: form.name,
        server: form.server,
        user: form.user.trim() || null,
        credential: form.useGlobal || unchanged ? null : credential,
        use_global: form.useGlobal && canCopyGlobal,
      });
      delete results[saved.id];
      form = null;
      await load();
    } catch (cause) {
      error = errorText(cause);
    } finally {
      busy = false;
    }
  }

  async function test(connection: Connection) {
    results[connection.id] = { ok: true, text: t('connections.testing') };
    try {
      const { name } = await connectionsApi.check(connection);
      results[connection.id] = {
        ok: true,
        text: name ? t('connections.testedAs', { name }) : t('connections.tested'),
      };
    } catch (cause) {
      results[connection.id] = { ok: false, text: errorText(cause, connection.provider) };
    }
  }

  async function remove() {
    const target = removing;
    if (!target) return;
    try {
      await connectionsApi.remove(target.id);
      if (form?.id === target.id) form = null;
      await load();
    } catch (cause) {
      error = errorText(cause, target.provider);
    } finally {
      removing = null;
    }
  }
</script>

<div class="connections">
  <p class="note">{t('connections.intro')}</p>
  {#if error}<p class="note error" role="alert">{error}</p>{/if}

  {#each connections ?? [] as connection (connection.id)}
    <div class="connection" data-connection={connection.id}>
      <div class="summary">
        <strong>{connection.name}</strong>
        <small>{providerName(connection.provider)} · {connection.server}</small>
        <small class="credential">
          {connection.reference ?? t(connection.stored ? 'connections.stored' : 'connections.noToken')}
        </small>
        {#if results[connection.id]}
          <small class="result" class:error={!results[connection.id].ok} role="status">{results[connection.id].text}</small>
        {/if}
      </div>
      <div class="actions">
        <button type="button" class="test" onclick={() => test(connection)}>
          <PlugZap size={13} /> {t('connections.test')}
        </button>
        <button type="button" class="edit" title={t('connections.edit')} aria-label={t('connections.edit')} onclick={() => edit(connection)}>
          <Pencil size={13} />
        </button>
        <button type="button" class="remove" title={t('connections.remove')} aria-label={t('connections.remove')} onclick={() => (removing = connection)}>
          <Trash2 size={13} />
        </button>
      </div>
    </div>
  {/each}

  {#if form}
    <form onsubmit={save}>
      <label>
        {t('connections.provider')}
        <select name="provider" bind:value={form.provider} disabled={!!form.id}>
          {#each providers as option (option.id)}
            <option value={option.id}>{option.name}</option>
          {/each}
        </select>
      </label>
      <label>
        {t('connections.name')}
        <input name="name" required bind:value={form.name} />
      </label>
      {#each provider?.connection_fields ?? [] as field (field.id)}
        <label>
          {t(`connections.fields.${field.id}`)}
          <input name={field.id} required={field.required} placeholder={field.example} spellcheck="false" bind:value={form[field.id]} />
        </label>
      {/each}
      {#if previous && previous.server !== form.server.trim().replace(/\/+$/, '')}
        <p class="note warning">{t('connections.serverChange')}</p>
      {/if}
      {#if canCopyGlobal}
        <label class="check">
          <input type="checkbox" name="use_global" bind:checked={form.useGlobal} />
          {t('connections.useGlobal')}
        </label>
      {/if}
      {#if !form.useGlobal}
        <label>
          {t('connections.token')}
          <input
            name="credential"
            type={secret ? 'password' : 'text'}
            autocomplete="off"
            spellcheck="false"
            placeholder={previous?.stored ? t('azure.tokenStored') : t('azure.tokenPlaceholder')}
            bind:value={form.credential}
          />
        </label>
      {/if}
      {#if !form.id && legacyOnly && form.provider === LEGACY_PROVIDER}
        <p class="note warning">{t('connections.firstWarning')}</p>
      {/if}
      <div class="actions end">
        <button type="button" onclick={() => (form = null)}>{t('common.cancel')}</button>
        <button type="submit" class="primary" disabled={busy}>{t('common.save')}</button>
      </div>
    </form>
  {:else}
    <div class="actions">
      <button type="button" class="add" disabled={!providers.length} onclick={() => create()}>
        <Plus size={13} /> {t('connections.add')}
      </button>
      {#if legacyOnly && hasGlobal}
        <button type="button" class="from-global" onclick={() => create(true)}>{t('connections.fromGlobal')}</button>
      {/if}
    </div>
  {/if}

  {#if connections && connections.length === 0 && !form}
    <h4>{t('connections.single')}</h4>
    <AzureTokenField autosave />
  {/if}
</div>

<ConfirmDialog
  open={removing !== null}
  title={t('connections.removeTitle', { name: removing?.name ?? '' })}
  description={t('connections.removeHint')}
  confirmText={t('connections.remove')}
  cancelText={t('common.cancel')}
  variant="danger"
  onConfirm={remove}
  onCancel={() => (removing = null)}
/>

<style>
  .connections {
    display: flex;
    flex-direction: column;
    gap: 10px;
    font-size: 12px;
  }

  .connection {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
    padding: 10px;
    border: 1px solid var(--border-primary);
    border-radius: 8px;
  }

  .summary {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    overflow-wrap: anywhere;
    user-select: text;
  }

  small,
  .note {
    color: var(--text-muted);
    margin: 0;
  }

  .result {
    color: var(--text-primary);
  }

  .error {
    color: var(--danger, #d1242f);
  }

  .warning {
    color: var(--warning, #d97706);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border: 1px solid var(--accent);
    border-radius: 8px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  label.check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }

  input:not([type='checkbox']),
  select {
    padding: 6px 8px;
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    background: var(--bg-secondary);
    color: var(--text-primary);
    font: inherit;
  }

  .actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  .actions.end {
    justify-content: flex-end;
  }

  button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    font-size: 12px;
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    background: var(--bg-primary);
    color: var(--text-primary);
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  h4 {
    margin: 8px 0 0;
  }
</style>
