<!--
  De dónde salen las tareas de un tablero: conexión, proyecto, qué tareas (las propias, por criterios
  o con una consulta escrita) y qué estados lleva cada columna. Es un panel junto al tablero, sin
  bloquear el resto: el agente del chat puede probar consultas y cambiar este borrador, y lo que
  cambia queda marcado hasta que el usuario guarda o cancela. Con tareas vinculadas, cambiar de
  conexión o de proyecto las desvincula y se confirma antes.
-->
<script lang="ts">
  import { onDestroy, onMount, untrack } from 'svelte';
  import { t } from '@shared/lib/i18n';
  import { ConfirmDialog } from '@shared/ui/confirm-dialog';
  import AzureTokenField from './AzureTokenField.svelte';
  import FilterPicker from './FilterPicker.svelte';
  import { fetchIssues, filterQuery, filterValues } from '../api/issues.api';
  import type { Connection } from '../api/connections.api';
  import { usesFilter, type Criterion, type IssueFilter, type IssueProvider, type IssueSource } from '../model/issue-sync';
  import { openConfigDraft, type DraftChange, type QueryMode, type SourceDraft } from '../model/config-draft';

  interface Props {
    /** El origen actual; sin él, se está eligiendo por primera vez. */
    source: IssueSource | undefined;
    columns: { id: string; label: string }[];
    provider: IssueProvider;
    connections: Connection[];
    /** Recién creado el tablero: cancelar lo deja solo local. */
    isNew?: boolean;
    errorText: (error: unknown) => string;
    onSave: (source: IssueSource) => void;
    onClose: () => void;
  }

  let { source, columns, provider, connections, isNew = false, errorText, onSave, onClose }: Props = $props();

  // El panel edita una copia de lo que había al abrirlo
  const initial = untrack(() => source);
  let connection = $state(initial?.connection ?? '');
  let scope = $state(initial?.scope ?? '');
  let issueType = $state(initial?.issue_type ?? '');
  let assignee = $state(initial?.assignee ?? '');
  let mode = $state<QueryMode>(initial && usesFilter(initial) ? 'criteria' : initial?.query?.trim() ? 'custom' : 'mine');
  let query = $state(initial?.query ?? '');
  let filter = $state<IssueFilter>(structuredClone($state.snapshot(initial?.filter) ?? {}));
  /** Los estados de cada columna, como se escriben: separados por comas. */
  let stateText = $state<Record<string, string>>(
    Object.fromEntries(untrack(() => columns).map(({ id }) => [id, initial?.states?.[id]?.join(', ') ?? ''])),
  );
  /** Lo que cambió el agente y el usuario aún no ha tocado. */
  let marks = $state<Record<string, boolean>>({});
  let tokenField = $state<ReturnType<typeof AzureTokenField>>();
  let panel = $state<HTMLDivElement>();
  let error = $state('');
  let saving = $state(false);
  /** El origen listo para guardar, a la espera de confirmar que se desvinculan las tareas. */
  let unlinking = $state<IssueSource | null>(null);

  let own = $derived(connections.filter((c) => c.provider === provider.id));
  /** Lo que se sabe del origen mientras se escribe: con esto se piden los valores. */
  let draft = $derived<IssueSource>({ provider: provider.id, connection, scope: scope.trim() });

  const display = (value: string) =>
    value === '@me' ? t('issues.source.me') : value === '@current' ? t('issues.source.current') : value;

  const chosenFilter = () =>
    Object.fromEntries(Object.entries($state.snapshot(filter)).filter(([, values]) => values?.length)) as IssueFilter;

  const chosenStates = () =>
    Object.fromEntries(
      Object.entries(stateText)
        .map(([id, text]) => [id, text.split(',').map((name) => name.trim()).filter(Boolean)] as const)
        .filter(([, names]) => names.length),
    );

  /** El borrador tal como se ve ahora. */
  function snapshot(): SourceDraft {
    const links = source?.scope === scope.trim() ? Object.values(source?.links ?? {}) : [];
    return {
      provider: provider.id,
      connection,
      scope: scope.trim(),
      mode,
      query: mode === 'custom' ? query : '',
      filter: mode === 'criteria' ? chosenFilter() : {},
      states: chosenStates(),
      columns: columns.map(({ id, label }) => ({ id, title: label })),
      linked: links.map((link) => link.id),
      today: new Date().toLocaleDateString('sv-SE'),
    };
  }

  /** Un cambio del agente: se ve y queda marcado; nada se guarda. */
  function applyChange(change: DraftChange): SourceDraft {
    if (change.query !== undefined) {
      query = change.query;
      mode = change.query.trim() ? 'custom' : 'mine';
      marks.mode = marks.query = true;
    }
    if (change.filter !== undefined) {
      filter = structuredClone(change.filter);
      mode = 'criteria';
      marks.mode = marks.filter = true;
    }
    for (const [column, names] of Object.entries(change.states ?? {})) {
      stateText[column] = names.join(', ');
      marks[`states:${column}`] = true;
    }
    return snapshot();
  }

  const assistant = openConfigDraft(applyChange);
  $effect(() => assistant.publish(snapshot()));
  onDestroy(() => assistant.close());

  onMount(() => panel?.querySelector<HTMLElement>('select, input')?.focus());

  /** Escape cierra el panel solo desde él o sin nada enfocado: el resto de la ventana sigue en uso. */
  function onWindowKey(event: KeyboardEvent) {
    if (event.key !== 'Escape' || event.defaultPrevented || unlinking || !panel?.offsetParent) return;
    if (event.target === document.body || panel.contains(event.target as Node)) onClose();
  }

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
      // Lo que no se edita aquí (vínculos, fechas) es lo que el tablero tenga ahora
      const next: IssueSource = { ...$state.snapshot(source), provider: provider.id, connection, scope: scope.trim() };
      const optional = { issue_type: issueType.trim(), assignee: assignee.trim() };
      for (const key of ['issue_type', 'assignee'] as const) {
        if (optional[key]) next[key] = optional[key];
        else delete next[key];
      }
      delete next.query;
      delete next.filter;
      delete next.filter_query;
      const states = { ...next.states };
      for (const { id } of columns) delete states[id];
      Object.assign(states, chosenStates());
      if (Object.keys(states).length) next.states = states;
      else delete next.states;
      if (mode === 'custom' && query.trim()) next.query = query.trim();
      if (mode === 'criteria') {
        const chosen = chosenFilter();
        const compiled = await filterQuery(next, chosen);
        Object.assign(next, { filter: chosen, query: compiled, filter_query: compiled });
      }
      // Elegir la conexión que antes se deducía no es cambiarla
      const moved = !!source && (source.scope !== next.scope || (!!source.connection && source.connection !== next.connection));
      if (moved && Object.keys(source?.links ?? {}).length) {
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

<svelte:window onkeydown={onWindowKey} />

{#snippet changed(mark: string)}
  {#if marks[mark]}<span class="agent-mark">{t('issues.assistant.changed')}</span>{/if}
{/snippet}

<div bind:this={panel} class="issues-dialog source-panel synapse-dialog" role="dialog" aria-modal="false" aria-labelledby="source-panel-title">
  <h2 id="source-panel-title" class="issues-title">
    {isNew ? t('issues.source.newTitle') : t('issues.settingsTitle', { provider: provider.name })}
  </h2>
  <p class="note assistant-note">{t('issues.assistant.notice')}</p>
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

    <fieldset class:agent-changed={marks.mode}>
      <legend>{t('issues.source.which')} {@render changed('mode')}</legend>
      {#each ['mine', 'criteria', 'custom'] as const as option (option)}
        <label class="radio">
          <input type="radio" name="mode" value={option} bind:group={mode} onchange={() => (marks.mode = false)} />
          {t(`issues.source.${option === 'criteria' ? 'criteriaMode' : option}`)}
        </label>
      {/each}
    </fieldset>

    {#if mode === 'criteria'}
      <div class="criteria" class:agent-changed={marks.filter}>
        <p class="note">{t('issues.source.criteriaHint')} {@render changed('filter')}</p>
        {#each provider.filter_criteria ?? [] as criterion (criterion)}
          <!-- Otro proyecto o conexión, otros valores -->
          {#key `${draft.scope} ${connection}`}
            <FilterPicker
              id={criterion}
              label={t(`issues.source.criteria.${criterion}`)}
              selected={filter[criterion] ?? []}
              load={() => load(criterion)}
              {display}
              onChange={(values) => {
                filter = { ...filter, [criterion]: values };
                marks.filter = false;
              }}
            />
          {/key}
        {/each}
      </div>
    {:else if mode === 'custom'}
      <label class:agent-changed={marks.query}>
        <span>{t('issues.query', { language: provider.query_language })} {@render changed('query')}</span>
        <textarea name="query" rows="6" placeholder={provider.default_query} bind:value={query} oninput={() => (marks.query = false)}></textarea>
      </label>
    {/if}

    <fieldset class="states">
      <legend>{t('issues.source.states')}</legend>
      <p class="note">{t('issues.source.statesHint')}</p>
      {#each columns as column (column.id)}
        <label class:agent-changed={marks[`states:${column.id}`]}>
          <span>{column.label} {@render changed(`states:${column.id}`)}</span>
          <input
            name="states"
            data-column={column.id}
            placeholder={provider.default_states[column.id]?.join(', ')}
            bind:value={stateText[column.id]}
            oninput={() => (marks[`states:${column.id}`] = false)}
          />
        </label>
      {/each}
    </fieldset>

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
</div>

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
  /* Pegado al borde izquierdo del tablero: el chat del agente, a la derecha, queda libre */
  :global(.board-editor:has(.source-panel)) {
    position: relative;
  }

  :global(.issues-dialog.source-panel) {
    position: absolute;
    top: 0;
    right: auto;
    bottom: 0;
    left: 0;
    transform: none;
    z-index: 20;
    width: min(520px, 100%);
    max-width: none;
    max-height: none;
    border-width: 0 1px 0 0;
    border-radius: 0;
  }

  .issues-title {
    margin: 0;
  }

  .agent-changed {
    padding-left: 8px;
    border-left: 3px solid var(--accent);
  }

  .criteria {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .agent-mark {
    padding: 1px 6px;
    font-size: 10px;
    border-radius: 999px;
    background: var(--accent);
    color: white;
  }

  .source-form {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  label,
  fieldset {
    flex-shrink: 0;
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
