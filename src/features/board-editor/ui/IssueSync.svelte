<script lang="ts">
  import { onMount } from 'svelte';
  import { Dialog } from 'bits-ui';
  import { CloudDownload, CloudUpload, Settings2, X } from 'lucide-svelte';
  import { t } from '@shared/lib/i18n';
  import AzureTokenField from './AzureTokenField.svelte';
  import { connectionsApi } from '../api/connections.api';
  import { fetchIssues, issueProviders, issueWriter, type IssueError } from '../api/issues.api';
  import {
    applyPull,
    buildPushPlan,
    runPush,
    type IssueProvider,
    type IssueSource,
    type PushPlan,
  } from '../model/issue-sync';
  import type { BoardDocument } from '../model/board.svelte';

  interface Props {
    /** El tablero tal como está ahora: se vuelve a leer después de cada llamada. */
    read: () => BoardDocument;
    write: (next: BoardDocument) => void;
  }

  let { read, write }: Props = $props();

  let busy = $state(false);
  let tokenField = $state<ReturnType<typeof AzureTokenField>>();
  let providers = $state<IssueProvider[]>([]);
  /** Con conexiones del proveedor, el token es el de cada una. */
  let hasConnections = $state(false);
  // El del tablero o, si no tiene origen, el primero que haya
  let provider = $derived(providers.find((p) => p.id === read().source?.provider) ?? providers[0]);
  let messages = $state<string[]>([]);
  let settings = $state<Required<Pick<IssueSource, 'scope' | 'issue_type' | 'assignee' | 'query'>> | null>(null);
  let review = $state<PushPlan | null>(null);
  let chosen = $state<Record<string, boolean>>({});
  let toCreate = $state<Record<string, boolean>>({});
  let selection = $derived(
    review
      ? review.changes.filter((c) => chosen[c.taskId]).length + review.unlinked.filter((u) => toCreate[u.id]).length
      : 0,
  );

  async function loadProviders() {
    if (!providers.length) providers = await issueProviders();
  }

  onMount(() => {
    loadProviders().catch((error) => (messages = [errorText(error)]));
  });

  function errorText(error: unknown): string {
    const e = error as Partial<IssueError>;
    if (!e?.kind) return String(error);
    return t(`issues.errors.${e.kind}`, { provider: provider?.name ?? '', detail: e.detail ?? '' });
  }

  function openSettings() {
    const source = read().source;
    connectionsApi
      .list()
      .then((all) => (hasConnections = all.some((c) => c.provider === (source?.provider ?? provider?.id))))
      .catch((error) => (messages = [errorText(error)]));
    settings = {
      scope: source?.scope ?? '',
      issue_type: source?.issue_type ?? '',
      assignee: source?.assignee ?? '',
      query: source?.query ?? '',
    };
  }

  async function saveSettings(event: SubmitEvent) {
    event.preventDefault();
    // El token no va en el tablero; si no se puede guardar, el diálogo sigue abierto con el motivo
    if (tokenField && !(await tokenField.save())) return;
    if (!settings || !provider) return;
    const doc = read();
    const scope = settings.scope.trim();
    const source: IssueSource = { ...(doc.source ?? { provider: provider.id, connection: '' }), scope };
    for (const key of ['issue_type', 'assignee', 'query'] as const) {
      const value = settings[key].trim();
      if (value) source[key] = value;
      else delete source[key];
    }
    // Los ids vinculados son del ámbito anterior: con otro no se conservan
    if (doc.source && doc.source.scope !== scope) delete source.links;
    write({ ...doc, source });
    settings = null;
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
    } finally {
      busy = false;
    }
  }

  const pull = () =>
    sync(async (source, declared) => {
      const query = source.query?.trim() || declared.default_query;
      const { items, requested } = await fetchIssues(source, query, linkedIds(read()));
      if (read().source?.scope !== source.scope) return;
      const result = applyPull(read(), declared, items, requested, new Date().toLocaleDateString('sv-SE'));
      write(result.doc);
      messages = [
        t('issues.pulled', { added: result.added, updated: result.updated }),
        ...(result.conflicts.length ? [t('issues.conflicts', { tasks: result.conflicts.join(', ') })] : []),
        ...(result.missing.length ? [t('issues.missing', { ids: result.missing.join(', ') })] : []),
      ];
    });

  const preparePush = () =>
    sync(async (source, declared) => {
      const { items } = await fetchIssues(source, '', linkedIds(read()));
      const doc = read();
      if (doc.source?.scope !== source.scope) return;
      const plan = buildPushPlan(doc, declared, items);
      if (plan.missing.length) {
        const links = { ...doc.source.links };
        for (const { taskId } of plan.missing) delete links[taskId];
        write({ ...doc, source: { ...doc.source, links } });
        messages = [t('issues.missing', { ids: plan.missing.map((m) => m.id).join(', ') })];
      }
      if (plan.unmapped.length) messages = [...messages, t('issues.unmapped', { tasks: plan.unmapped.join(', ') })];
      if (plan.changes.length === 0 && plan.unlinked.length === 0) {
        messages = [...messages, t('issues.nothingToSend')];
        return;
      }
      // Lo que choca con el proveedor y las tareas por crear hay que marcarlos
      chosen = Object.fromEntries(plan.changes.map((c) => [c.taskId, !c.conflict]));
      toCreate = {};
      review = plan;
    });

  async function send() {
    const plan = review;
    if (!plan) return;
    review = null;
    await sync(async (source, declared) => {
      const result = await runPush(
        declared,
        issueWriter(source),
        read,
        (taskId, link) => {
          const doc = read();
          if (doc.source) write({ ...doc, source: { ...doc.source, links: { ...doc.source.links, [taskId]: link } } });
        },
        plan.changes.filter((c) => chosen[c.taskId]),
        plan.unlinked.filter((u) => toCreate[u.id]).map((u) => u.id),
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

<Dialog.Root open={settings !== null} onOpenChange={(open) => { if (!open) settings = null; }}>
  <Dialog.Portal>
    <Dialog.Overlay class="issues-overlay" />
    <Dialog.Content class="issues-dialog synapse-dialog">
      <Dialog.Title class="issues-title">{t('issues.settingsTitle', { provider: provider?.name ?? '' })}</Dialog.Title>
      {#if settings}
        <form onsubmit={saveSettings}>
          <label>
            {t('issues.projectUrl')}
            <input name="scope" type="url" required placeholder={provider?.scope_example} bind:value={settings.scope} />
          </label>
          {#if hasConnections}
            <p class="issues-note">{t('connections.boardHint')}</p>
          {:else}
            <AzureTokenField bind:this={tokenField} />
          {/if}
          <label>
            {t('issues.issueType')}
            <input name="issue_type" placeholder={provider?.default_issue_type} bind:value={settings.issue_type} />
          </label>
          <label>
            {t('issues.assignee')}
            <input name="assignee" bind:value={settings.assignee} />
          </label>
          <label>
            {t('issues.query', { language: provider?.query_language ?? '' })}
            <textarea name="query" rows="6" placeholder={provider?.default_query} bind:value={settings.query}></textarea>
          </label>
          {#if read().source?.links && read().source?.scope !== settings.scope.trim()}
            <p class="issues-note warning">{t('issues.linksCleared')}</p>
          {/if}
          <div class="issues-actions">
            <button type="button" onclick={() => (settings = null)}>{t('common.cancel')}</button>
            <button type="submit" class="primary">{t('common.save')}</button>
          </div>
        </form>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<Dialog.Root open={review !== null} onOpenChange={(open) => { if (!open) review = null; }}>
  <Dialog.Portal>
    <Dialog.Overlay class="issues-overlay" />
    <Dialog.Content class="issues-dialog synapse-dialog">
      <Dialog.Title class="issues-title">{t('issues.reviewTitle', { provider: provider?.name ?? '' })}</Dialog.Title>
      {#if review}
        <p class="issues-note">{read().source?.scope}</p>
        <div class="issues-list">
          {#each review.changes as change (change.taskId)}
            <label class="issues-change" data-task={change.taskId}>
              <input type="checkbox" bind:checked={chosen[change.taskId]} />
              <span>
                <strong>#{change.item.id} {taskTitle(change.taskId)}</strong>
                {#if change.title !== undefined}<small>{t('issues.titleChange', { from: change.item.title, to: change.title })}</small>{/if}
                {#if change.state !== undefined}<small>{t('issues.stateChange', { from: change.item.state.name, to: change.state })}</small>{/if}
                {#each change.comments as comment (comment.timestamp)}<small>{t('issues.commentChange', { text: comment.text })}</small>{/each}
                {#if change.conflict}<small class="warning">{t('issues.conflictChange', { provider: provider?.name ?? '' })}</small>{/if}
              </span>
            </label>
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

  form,
  .issues-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  form label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  form input,
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

  .issues-list {
    overflow: auto;
  }

  .issues-list label {
    display: flex;
    align-items: flex-start;
    gap: 8px;
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
