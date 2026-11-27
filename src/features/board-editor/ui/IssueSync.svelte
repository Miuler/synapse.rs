<script lang="ts">
  import { onMount } from 'svelte';
  import { Dialog } from 'bits-ui';
  import { CloudDownload, CloudUpload, Settings2, X } from 'lucide-svelte';
  import { t } from '@shared/lib/i18n';
  import SourceDialog from './SourceDialog.svelte';
  import { azureToken } from '../api/azure-devops.api';
  import { connectionsApi, type Connection } from '../api/connections.api';
  import { fetchActivity, fetchIssues, filterQuery, issueProviders, issueWriter, type IssueError } from '../api/issues.api';
  import { changedIssues, receive } from '../model/activity';
  import type { TaskFile } from '../model/folder-board';
  import {
    applyPull,
    buildPushPlan,
    hasChanges,
    runPush,
    usesFilter,
    type IssueProvider,
    type IssueSource,
    type PushChange,
    type PushPlan,
  } from '../model/issue-sync';
  import type { BoardDocument } from '../model/board.svelte';

  interface Props {
    /** El tablero tal como está ahora: se vuelve a leer después de cada llamada. */
    read: () => BoardDocument;
    write: (next: BoardDocument) => void;
    /** El tablero se acaba de crear: si se puede sincronizar, se pregunta de dónde salen sus tareas. */
    isNew?: boolean;
    /** Solo en los tableros de carpeta: con ella se guarda también la descripción y la actividad. */
    readTask?: (id: string) => Promise<{ task: TaskFile }>;
  }

  let { read, write, isNew = false, readTask }: Props = $props();

  let busy = $state(false);
  let providers = $state<IssueProvider[]>([]);
  // El del tablero o, si no tiene origen, el primero que haya
  let provider = $derived(providers.find((p) => p.id === read().source?.provider) ?? providers[0]);
  let messages = $state<string[]>([]);
  /** El diálogo del origen: abierto con las conexiones de ese momento. */
  let editing = $state<{ connections: Connection[]; isNew: boolean } | null>(null);
  let review = $state<PushPlan | null>(null);
  let chosen = $state<Record<string, boolean>>({});
  let toCreate = $state<Record<string, boolean>>({});
  /** Lo que puede perder formato en el proveedor se marca aparte. */
  let withDescription = $state<Record<string, boolean>>({});
  let withEdits = $state<Record<string, boolean>>({});
  /** Las descripciones leídas al preparar el envío. */
  let descriptions: Record<string, string> = {};

  /** Lo marcado de un cambio. */
  function marked(change: PushChange): PushChange {
    const next = { ...change };
    if (next.description?.lossy && !withDescription[change.taskId]) delete next.description;
    if (!withEdits[change.taskId]) delete next.edits;
    return next;
  }
  let outgoing = $derived(review ? review.changes.filter((c) => chosen[c.taskId]).map(marked).filter(hasChanges) : []);
  let selection = $derived(
    review ? outgoing.length + review.unlinked.filter((u) => toCreate[u.id]).length : 0,
  );

  async function loadProviders() {
    if (!providers.length) providers = await issueProviders();
  }

  onMount(() => {
    loadProviders()
      .then(() => isNew && !read().source && offerSource())
      .catch((error) => (messages = [errorText(error)]));
  });

  /** Solo se pregunta si hay con qué sincronizar: una conexión o el token único. */
  async function offerSource() {
    const [connections, token] = await Promise.all([connectionsApi.list(), azureToken.get().catch(() => null)]);
    if (connections.length || token?.reference || token?.stored) editing = { connections, isNew: true };
  }

  function errorText(error: unknown): string {
    const e = error as Partial<IssueError>;
    if (!e?.kind) return String(error);
    return t(`issues.errors.${e.kind}`, { provider: provider?.name ?? '', detail: e.detail ?? '' });
  }

  async function openSettings() {
    try {
      await loadProviders();
      editing = { connections: await connectionsApi.list(), isNew: false };
    } catch (error) {
      messages = [errorText(error)];
    }
  }

  function saveSource(source: IssueSource) {
    write({ ...read(), source });
    editing = null;
    messages = [];
  }

  /** Los ids vinculados a tareas que siguen en el tablero. */
  function linkedIds(doc: BoardDocument): string[] {
    return doc.tasks.flatMap((task) => doc.source?.links?.[task.id]?.id ?? []);
  }

  async function sync(action: (source: IssueSource, provider: IssueProvider) => Promise<void>) {
    const source = read().source;
    if (!source?.scope) return openSettings();
    busy = true;
    messages = [];
    try {
      await loadProviders();
      const declared = providers.find((p) => p.id === source.provider);
      if (!declared) throw { kind: 'unknown_provider', detail: source.provider } satisfies IssueError;
      await action(source, declared);
    } catch (error) {
      messages = [errorText(error)];
      // Varias conexiones sirven: hay que elegir una
      if ((error as Partial<IssueError>).kind === 'ambiguous_connection') await openSettings();
    } finally {
      busy = false;
    }
  }

  const pull = () =>
    sync(async (source, declared) => {
      let query = source.query?.trim() || declared.default_query;
      // «La iteración en curso» cambia con los días: la consulta del filtro se rehace al traer
      if (usesFilter(source)) {
        const compiled = await filterQuery(source, source.filter);
        const doc = read();
        if (compiled !== query && doc.source && usesFilter(doc.source)) {
          write({ ...doc, source: { ...doc.source, query: compiled, filter_query: compiled } });
        }
        query = compiled;
      }
      const { items, requested, matched } = await fetchIssues(source, query, linkedIds(read()));
      if (read().source?.scope !== source.scope) return;
      const today = new Date().toLocaleDateString('sv-SE');
      const apply = () => applyPull(read(), declared, items, requested, today, matched);
      // En un tablero de carpeta, además, la actividad de lo que cambió
      let activity: Awaited<ReturnType<typeof fetchActivity>> | undefined;
      const files: Record<string, TaskFile> = {};
      if (readTask) {
        const ids = changedIssues(apply().doc, items);
        activity = ids.length ? await fetchActivity(source, ids) : { items: {}, failed: [] };
        const existing = Object.entries(read().source?.links ?? {}).filter(([, link]) => activity!.items[link.id]);
        const present = new Set(read().tasks.map((task) => task.id));
        await Promise.all(
          existing.filter(([id]) => present.has(id)).map(async ([id]) => (files[id] = (await readTask(id)).task)),
        );
        if (read().source?.scope !== source.scope) return;
      }
      const result = apply();
      let { doc } = result;
      let changed = result.changed;
      let conflicts = result.conflicts;
      if (activity) {
        const received = receive(doc, items, activity.items, files);
        doc = received.doc;
        doc = { ...doc, source: { ...doc.source!, synced_at: new Date().toISOString().slice(0, 19) } };
        changed = [...new Set([...changed, ...received.changed])];
        conflicts = [...new Set([...conflicts, ...received.conflicts])];
      }
      write(doc);
      const failed = activity?.failed ?? [];
      messages = [
        t('issues.pulled', { added: result.added, updated: changed.length }),
        ...(failed.length ? [t('issues.activityFailed', { count: failed.length, detail: errorText(failed[0].error) })] : []),
        ...(conflicts.length ? [t('issues.conflicts', { tasks: conflicts.join(', ') })] : []),
        ...(result.missing.length ? [t('issues.missing', { ids: result.missing.join(', ') })] : []),
        ...(result.outside.length ? [t('issues.outsideCount', { tasks: result.outside.join(', ') })] : []),
      ];
    });

  const preparePush = () =>
    sync(async (source, declared) => {
      const { items, editable } = await fetchIssues(source, '', linkedIds(read()));
      const local: Record<string, string> = {};
      if (readTask) {
        await Promise.all(read().tasks.map(async ({ id }) => (local[id] = (await readTask(id)).task.description)));
      }
      const doc = read();
      if (doc.source?.scope !== source.scope) return;
      descriptions = local;
      const plan = buildPushPlan(doc, declared, items, { descriptions, editable });
      if (plan.missing.length) {
        const links = { ...doc.source.links };
        for (const { taskId } of plan.missing) delete links[taskId];
        write({ ...doc, source: { ...doc.source, links } });
        messages = [t('issues.missing', { ids: plan.missing.map((m) => m.id).join(', ') })];
      }
      if (plan.unmapped.length) messages = [...messages, t('issues.unmapped', { tasks: plan.unmapped.join(', ') })];
      if (plan.unsupported.length) messages = [...messages, ...plan.unsupported.map(unsupportedText)];
      if (plan.changes.length === 0 && plan.unlinked.length === 0) {
        messages = [...messages, t('issues.nothingToSend')];
        return;
      }
      // Lo que choca con el proveedor y las tareas por crear hay que marcarlos
      chosen = Object.fromEntries(plan.changes.map((c) => [c.taskId, !c.conflict]));
      toCreate = {};
      withDescription = {};
      withEdits = {};
      review = plan;
    });

  const unsupportedText = ({ taskId, field }: PushPlan['unsupported'][number]) =>
    field === 'foreignComment'
      ? t('issues.foreignComment', { task: taskId })
      : t('issues.unsupported', { provider: provider?.name ?? '', field: t(`issues.fields.${field}`), task: taskId });

  async function send() {
    const plan = review;
    if (!plan) return;
    const changes = outgoing;
    review = null;
    await sync(async (source, declared) => {
      const result = await runPush(
        declared,
        issueWriter(source),
        read,
        (taskId, link, sent, description) => {
          const doc = read();
          if (!doc.source) return;
          const tasks = doc.tasks.map((task) => {
            if (task.id !== taskId) return task;
            let next = task;
            if (sent) {
              const comments = task.comments.map((c) => {
                if (c.timestamp !== sent.timestamp || c.text !== sent.text) return c;
                const { edited: _, ...same } = c;
                return { ...same, remote: sent.remote };
              });
              next = { ...next, comments };
            }
            // La descripción tal como quedó en el proveedor se guarda como lo traído
            if (description) next = { ...next, incoming: { entries: [], description: description.text, over: description.over } };
            return next;
          });
          write({ ...doc, tasks, source: { ...doc.source, links: { ...doc.source.links, [taskId]: link } } });
        },
        $state.snapshot(changes) as PushChange[],
        plan.unlinked.filter((u) => toCreate[u.id]).map((u) => u.id),
        descriptions,
      );
      messages = [
        ...messages,
        t('issues.sent', { updated: result.updated, created: result.created, comments: result.comments }),
        ...result.errors.map((e) => `${e.taskId}: ${errorText(e.error)}`),
      ];
    });
  }

  const taskTitle = (id: string) => read().tasks.find((task) => task.id === id)?.title ?? id;
</script>

<div class="issues-sync">
  <span class="issues-name">{provider?.name}</span>
  <button type="button" class="issues-pull" disabled={busy} title={t('issues.pullHint', { provider: provider?.name ?? '' })} onclick={pull}>
    <CloudDownload size={14} /> {t('issues.pull')}
  </button>
  <button type="button" class="issues-push" disabled={busy} title={t('issues.pushHint', { provider: provider?.name ?? '' })} onclick={preparePush}>
    <CloudUpload size={14} /> {t('issues.push')}
  </button>
  <button type="button" class="issues-settings" disabled={busy} title={t('issues.settings', { provider: provider?.name ?? '' })} aria-label={t('issues.settings', { provider: provider?.name ?? '' })} onclick={openSettings}>
    <Settings2 size={14} />
  </button>
</div>

{#if busy || messages.length}
  <div class="issues-status" role="status">
    <div>
      {#if busy}<p>{t('issues.syncing')}</p>{/if}
      {#each messages as message, i (i)}<p>{message}</p>{/each}
    </div>
    {#if !busy}
      <button type="button" title={t('common.close')} aria-label={t('common.close')} onclick={() => (messages = [])}><X size={13} /></button>
    {/if}
  </div>
{/if}

{#if editing && provider}
  <SourceDialog
    source={read().source}
    {provider}
    connections={editing.connections}
    isNew={editing.isNew}
    {errorText}
    onSave={saveSource}
    onClose={() => (editing = null)}
  />
{/if}

<Dialog.Root open={review !== null} onOpenChange={(open) => { if (!open) review = null; }}>
  <Dialog.Portal>
    <Dialog.Overlay class="issues-overlay" />
    <Dialog.Content class="issues-dialog synapse-dialog">
      <Dialog.Title class="issues-title">{t('issues.reviewTitle', { provider: provider?.name ?? '' })}</Dialog.Title>
      {#if review}
        <p class="issues-note">{read().source?.scope}</p>
        <div class="issues-list">
          {#each review.changes as change (change.taskId)}
            {@const name = provider?.name ?? ''}
            <div class="issues-change" data-task={change.taskId}>
              <label>
                <input type="checkbox" bind:checked={chosen[change.taskId]} />
                <span>
                  <strong>#{change.item.id} {taskTitle(change.taskId)}</strong>
                  {#if change.title !== undefined}<small>{t('issues.titleChange', { from: change.item.title, to: change.title })}</small>{/if}
                  {#if change.state !== undefined}<small>{t('issues.stateChange', { from: change.item.state.name, to: change.state })}</small>{/if}
                  {#if change.type !== undefined}<small>{t('issues.typeChange', { from: change.item.type, to: change.type })}</small>{/if}
                  {#if change.assignee !== undefined}<small>{t('issues.assigneeChange', { from: change.item.assignee?.name || '—', to: change.assignee || '—' })}</small>{/if}
                  {#if change.description && !change.description.lossy}<small>{t('issues.descriptionChange', { text: change.description.text.slice(0, 200) })}</small>{/if}
                  {#each change.comments as comment (comment.timestamp)}<small>{t('issues.commentChange', { text: comment.text })}</small>{/each}
                  {#if change.conflict}<small class="warning">{t('issues.conflictChange', { provider: name })}</small>{/if}
                </span>
              </label>
              {#if change.description?.lossy}
                <label class="issues-option issues-lossy">
                  <input type="checkbox" bind:checked={withDescription[change.taskId]} />
                  <span>
                    <small class="warning">{t('issues.descriptionLossy', { provider: name })}</small>
                    <small>{t('issues.descriptionChange', { text: change.description.text.slice(0, 200) })}</small>
                  </span>
                </label>
              {/if}
              {#if change.edits}
                <label class="issues-option issues-edits">
                  <input type="checkbox" bind:checked={withEdits[change.taskId]} />
                  <span>
                    <small class="warning">{t('issues.commentEdits', { provider: name })}</small>
                    {#each change.edits as edit (edit.remote)}<small>{t('issues.commentChange', { text: edit.text })}</small>{/each}
                    {#if change.stale}<small class="warning">{t('issues.staleChange', { provider: name })}</small>{/if}
                  </span>
                </label>
              {/if}
            </div>
          {/each}
          {#if review.unlinked.length}
            <h4>{t('issues.createHeading', { type: read().source?.issue_type || (provider?.default_issue_type ?? '') })}</h4>
            {#each review.unlinked as task (task.id)}
              <label class="issues-create" data-task={task.id}>
                <input type="checkbox" bind:checked={toCreate[task.id]} />
                <span>
                  <strong>{task.title}</strong>
                  {#if task.comments.length}<small>{t('issues.createComments', { count: task.comments.length })}</small>{/if}
                </span>
              </label>
            {/each}
          {/if}
        </div>
        <div class="issues-actions">
          <button type="button" onclick={() => (review = null)}>{t('common.cancel')}</button>
          <button type="button" class="primary issues-confirm" disabled={selection === 0} onclick={send}>
            {t('issues.confirm', { count: selection })}
          </button>
        </div>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  .issues-sync {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }

  .issues-name {
    color: var(--text-muted);
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

  .issues-status {
    flex-basis: 100%;
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 12px;
    color: var(--text-secondary);
    user-select: text;
  }

  .issues-status p {
    margin: 2px 0;
  }

  .issues-status button {
    align-self: flex-start;
    border: none;
    padding: 2px;
  }

  :global(.issues-overlay) {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    z-index: 10001;
  }

  :global(.issues-dialog) {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 10002;
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 90%;
    max-width: 560px;
    max-height: 85vh;
    overflow: auto;
    padding: 20px;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border-primary);
    border-radius: 10px;
    box-shadow: var(--modal-shadow);
    font-size: 12px;
  }

  :global(.issues-title) {
    font-size: 15px;
    font-weight: 600;
  }

  .issues-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .issues-list {
    overflow: auto;
  }

  .issues-list label {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }

  /* Lo que se marca aparte cuelga de su tarea */
  .issues-list .issues-option {
    padding-left: 22px;
  }

  .issues-list span {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-wrap: anywhere;
  }

  .issues-note,
  small {
    color: var(--text-muted);
    margin: 0;
    overflow-wrap: anywhere;
  }

  .warning {
    color: var(--warning, #d97706);
  }

  h4 {
    margin: 4px 0 0;
  }

  .issues-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }
</style>
