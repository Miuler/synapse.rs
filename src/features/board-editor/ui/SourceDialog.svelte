<!--
  De dónde salen las tareas de un tablero: conexión, proyecto y qué tareas (las propias, por criterios
  o con una consulta escrita). Con tareas vinculadas, cambiar de conexión o de proyecto las desvincula
  y se confirma antes.
-->
<script lang="ts">
  import { untrack } from 'svelte';
  import { Dialog } from 'bits-ui';
  import { t } from '@shared/lib/i18n';
  import { ConfirmDialog } from '@shared/ui/confirm-dialog';
  import AzureTokenField from './AzureTokenField.svelte';
  import FilterPicker from './FilterPicker.svelte';
  import { fetchIssues, filterQuery, filterValues } from '../api/issues.api';
  import type { Connection } from '../api/connections.api';
  import { usesFilter, type Criterion, type IssueFilter, type IssueProvider, type IssueSource } from '../model/issue-sync';

  interface Props {
    /** El origen actual; sin él, se está eligiendo por primera vez. */
    source: IssueSource | undefined;
    provider: IssueProvider;
    connections: Connection[];
    /** Recién creado el tablero: cancelar lo deja solo local. */
    isNew?: boolean;
    errorText: (error: unknown) => string;
    onSave: (source: IssueSource) => void;
    onClose: () => void;
  }

  let { source, provider, connections, isNew = false, errorText, onSave, onClose }: Props = $props();

  type Mode = 'mine' | 'criteria' | 'custom';

  // El diálogo edita una copia de lo que había al abrirlo
  const initial = untrack(() => source);
  let connection = $state(initial?.connection ?? '');
  let scope = $state(initial?.scope ?? '');
  let issueType = $state(initial?.issue_type ?? '');
  let assignee = $state(initial?.assignee ?? '');
  let mode = $state<Mode>(initial && usesFilter(initial) ? 'criteria' : initial?.query?.trim() ? 'custom' : 'mine');
  let query = $state(initial?.query ?? '');
  let filter = $state<IssueFilter>(structuredClone($state.snapshot(initial?.filter) ?? {}));
  let tokenField = $state<ReturnType<typeof AzureTokenField>>();
  let error = $state('');
  let saving = $state(false);
  /** El origen listo para guardar, a la espera de confirmar que se desvinculan las tareas. */
  let unlinking = $state<IssueSource | null>(null);

  let own = $derived(connections.filter((c) => c.provider === provider.id));
  /** Lo que se sabe del origen mientras se escribe: con esto se piden los valores. */
  let draft = $derived<IssueSource>({ provider: provider.id, connection, scope: scope.trim() });

  const display = (value: string) =>
    value === '@me' ? t('issues.source.me') : value === '@current' ? t('issues.source.current') : value;

  async function load(criterion: Criterion): Promise<string[]> {
    if (!draft.scope) throw new Error(t('issues.source.scopeFirst'));
    try {
      const values = await filterValues(draft, criterion);
      if (criterion !== 'assignee') return values;
      // El proveedor no lista personas: se ofrecen las ya asignadas a las tareas vinculadas
      const linked = Object.values(initial?.links ?? {}).map((link) => link.id);
      const seen = linked.length && draft.scope === initial?.scope ? (await fetchIssues(draft, '', linked)).items : [];
      return [...new Set([...values, ...seen.flatMap((issue) => issue.assignee?.id || [])])];
    } catch (cause) {
      throw new Error(errorText(cause));
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (saving) return;
    saving = true;
    error = '';
    try {
      // El token no va en el tablero; si no se puede guardar, el diálogo sigue abierto con el motivo
      if (tokenField && !(await tokenField.save())) return;
      const next: IssueSource = { ...initial, provider: provider.id, connection, scope: scope.trim() };
      const optional = { issue_type: issueType.trim(), assignee: assignee.trim() };
      for (const key of ['issue_type', 'assignee'] as const) {
        if (optional[key]) next[key] = optional[key];
        else delete next[key];
      }
      delete next.query;
      delete next.filter;
      delete next.filter_query;
      if (mode === 'custom' && query.trim()) next.query = query.trim();
      if (mode === 'criteria') {
        const chosen = Object.fromEntries(
          Object.entries($state.snapshot(filter)).filter(([, values]) => values?.length),
        ) as IssueFilter;
        const compiled = await filterQuery(next, chosen);
        Object.assign(next, { filter: chosen, query: compiled, filter_query: compiled });
      }
      // Elegir la conexión que antes se deducía no es cambiarla
      const moved = !!initial && (initial.scope !== next.scope || (!!initial.connection && initial.connection !== next.connection));
      if (moved && Object.keys(initial?.links ?? {}).length) {
        unlinking = next;
        return;
      }
      onSave(next);
    } catch (cause) {
      error = errorText(cause);
    } finally {
      saving = false;
    }
  }
</script>

<Dialog.Root open onOpenChange={(open) => { if (!open && !unlinking) onClose(); }}>
  <Dialog.Portal>
    <Dialog.Overlay class="issues-overlay" />
    <Dialog.Content class="issues-dialog synapse-dialog">
      <Dialog.Title class="issues-title">
        {isNew ? t('issues.source.newTitle') : t('issues.settingsTitle', { provider: provider.name })}
      </Dialog.Title>
      <form class="source-form" onsubmit={save}>
        {#if own.length}
          <label>
            {t('issues.source.connection')}
            <select name="connection" bind:value={connection}>
              <option value="">{t('issues.source.automatic')}</option>
              {#each own as option (option.id)}
                <option value={option.id}>{option.name} · {option.server}</option>
              {/each}
            </select>
          </label>
        {/if}
        <label>
          {t('issues.projectUrl')}
          <input name="scope" type="url" required placeholder={provider.scope_example} bind:value={scope} />
        </label>
        {#if own.length}
          <p class="note">{t('connections.boardHint')}</p>
        {:else}
          <AzureTokenField bind:this={tokenField} />
        {/if}

        <fieldset>
          <legend>{t('issues.source.which')}</legend>
          {#each ['mine', 'criteria', 'custom'] as const as option (option)}
            <label class="radio">
              <input type="radio" name="mode" value={option} bind:group={mode} />
              {t(`issues.source.${option === 'criteria' ? 'criteriaMode' : option}`)}
            </label>
          {/each}
        </fieldset>

        {#if mode === 'criteria'}
          <p class="note">{t('issues.source.criteriaHint')}</p>
          {#each provider.filter_criteria ?? [] as criterion (criterion)}
            <!-- Otro proyecto o conexión, otros valores -->
            {#key `${draft.scope} ${connection}`}
              <FilterPicker
                id={criterion}
                label={t(`issues.source.criteria.${criterion}`)}
                selected={filter[criterion] ?? []}
                load={() => load(criterion)}
                {display}
                onChange={(values) => (filter = { ...filter, [criterion]: values })}
              />
            {/key}
          {/each}
        {:else if mode === 'custom'}
          <label>
            {t('issues.query', { language: provider.query_language })}
            <textarea name="query" rows="6" placeholder={provider.default_query} bind:value={query}></textarea>
          </label>
        {/if}

        <label>
          {t('issues.issueType')}
          <input name="issue_type" placeholder={provider.default_issue_type} bind:value={issueType} />
        </label>
        <label>
          {t('issues.assignee')}
          <input name="assignee" bind:value={assignee} />
        </label>

        {#if error}<p class="note error" role="alert">{error}</p>{/if}
        <div class="actions">
          <button type="button" class="cancel" onclick={onClose}>
            {isNew ? t('issues.source.localOnly') : t('common.cancel')}
          </button>
          <button type="submit" class="primary" disabled={saving}>{t('common.save')}</button>
        </div>
      </form>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<ConfirmDialog
  open={unlinking !== null}
  title={t('issues.source.unlinkTitle')}
  description={t('issues.source.unlinkHint', { provider: provider.name })}
  confirmText={t('issues.source.unlink')}
  cancelText={t('common.cancel')}
  variant="danger"
  onConfirm={() => {
    const next = unlinking;
    unlinking = null;
    if (!next) return;
    delete next.links;
    onSave(next);
  }}
  onCancel={() => (unlinking = null)}
/>

<style>
  .source-form {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  label.radio {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }

  fieldset {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 8px 10px;
    border: 1px solid var(--border-primary);
    border-radius: 6px;
  }

  input:not([type='radio']),
  select,
  textarea {
    padding: 6px 8px;
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    background: var(--bg-secondary);
    color: var(--text-primary);
    font: inherit;
  }

  textarea {
    font-family: var(--font-mono, monospace);
  }

  /* El buscador del criterio dibuja su propio marco */
  .source-form :global(.filter-picker input) {
    padding: 2px 4px;
    border: none;
    background: transparent;
  }

  .note {
    margin: 0;
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }

  .error {
    color: var(--danger, #d1242f);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  button {
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
  }

  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }
</style>
