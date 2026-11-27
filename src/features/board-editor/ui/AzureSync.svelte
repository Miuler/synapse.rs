<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { CloudDownload, CloudUpload, Settings2, X } from 'lucide-svelte';
  import { t } from '@shared/lib/i18n';
  import AzureTokenField from './AzureTokenField.svelte';
  import { azureWriter, fetchWorkItems, type AzureError } from '../api/azure-devops.api';
  import {
    applyPull,
    buildPushPlan,
    runPush,
    DEFAULT_QUERY,
    DEFAULT_WORK_ITEM_TYPE,
    type AzureSyncConfig,
    type PushPlan,
  } from '../model/azure-sync';
  import type { BoardDocument } from '../model/board.svelte';

  interface Props {
    /** El tablero tal como está ahora: se vuelve a leer después de cada llamada. */
    read: () => BoardDocument;
    write: (next: BoardDocument) => void;
  }

  let { read, write }: Props = $props();

  let busy = $state(false);
  let tokenField = $state<ReturnType<typeof AzureTokenField>>();
  let messages = $state<string[]>([]);
  let settings = $state<Required<Pick<AzureSyncConfig, 'project_url' | 'work_item_type' | 'assigned_to' | 'query'>> | null>(null);
  let review = $state<PushPlan | null>(null);
  let chosen = $state<Record<string, boolean>>({});
  let toCreate = $state<Record<string, boolean>>({});
  let selection = $derived(
    review
      ? review.changes.filter((c) => chosen[c.taskId]).length + review.unlinked.filter((u) => toCreate[u.id]).length
      : 0,
  );

  function errorText(error: unknown): string {
    const e = error as Partial<AzureError>;
    if (!e?.kind) return String(error);
    return t(`azure.errors.${e.kind}`, { detail: e.detail ?? '' });
  }

  function openSettings() {
    const azure = read().azure;
    settings = {
      project_url: azure?.project_url ?? '',
      work_item_type: azure?.work_item_type ?? '',
      assigned_to: azure?.assigned_to ?? '',
      query: azure?.query ?? '',
    };
  }

  async function saveSettings(event: SubmitEvent) {
    event.preventDefault();
    // El token no va en el tablero; si no se puede guardar, el diálogo sigue abierto con el motivo
    if (tokenField && !(await tokenField.save())) return;
    if (!settings) return;
    const doc = read();
    const projectUrl = settings.project_url.trim();
    const azure: AzureSyncConfig = { ...doc.azure, project_url: projectUrl };
    for (const key of ['work_item_type', 'assigned_to', 'query'] as const) {
      const value = settings[key].trim();
      if (value) azure[key] = value;
      else delete azure[key];
    }
    // Los ids vinculados son de la organización anterior: con otro proyecto no se conservan
    if (doc.azure && doc.azure.project_url !== projectUrl) delete azure.links;
    write({ ...doc, azure });
    settings = null;
    messages = [];
  }

  /** Los ids vinculados a tareas que siguen en el tablero. */
  function linkedIds(doc: BoardDocument): number[] {
    return doc.tasks.flatMap((task) => doc.azure?.links?.[task.id]?.id ?? []);
  }

  async function sync(action: (azure: AzureSyncConfig) => Promise<void>) {
    const azure = read().azure;
    if (!azure?.project_url) return openSettings();
    busy = true;
    messages = [];
    try {
      await action(azure);
    } catch (error) {
      messages = [errorText(error)];
    } finally {
      busy = false;
    }
  }

  const pull = () =>
    sync(async (azure) => {
      const { items, requested } = await fetchWorkItems(azure.project_url, azure.query?.trim() || DEFAULT_QUERY, linkedIds(read()));
      if (read().azure?.project_url !== azure.project_url) return;
      const result = applyPull(read(), items, requested, new Date().toLocaleDateString('sv-SE'));
      write(result.doc);
      messages = [
        t('azure.pulled', { added: result.added, updated: result.updated }),
        ...(result.conflicts.length ? [t('azure.conflicts', { tasks: result.conflicts.join(', ') })] : []),
        ...(result.missing.length ? [t('azure.missing', { ids: result.missing.join(', ') })] : []),
      ];
    });

  const preparePush = () =>
    sync(async (azure) => {
      const { items } = await fetchWorkItems(azure.project_url, '', linkedIds(read()));
      const doc = read();
      if (doc.azure?.project_url !== azure.project_url) return;
      const plan = buildPushPlan(doc, items);
      if (plan.missing.length) {
        const links = { ...doc.azure.links };
        for (const { taskId } of plan.missing) delete links[taskId];
        write({ ...doc, azure: { ...doc.azure, links } });
        messages = [t('azure.missing', { ids: plan.missing.map((m) => m.id).join(', ') })];
      }
      if (plan.unmapped.length) messages = [...messages, t('azure.unmapped', { tasks: plan.unmapped.join(', ') })];
      if (plan.changes.length === 0 && plan.unlinked.length === 0) {
        messages = [...messages, t('azure.nothingToSend')];
        return;
      }
      // Lo que choca con Azure DevOps y las tareas por crear hay que marcarlos
      chosen = Object.fromEntries(plan.changes.map((c) => [c.taskId, !c.conflict]));
      toCreate = {};
      review = plan;
    });

  async function send() {
    const plan = review;
    if (!plan) return;
    review = null;
    await sync(async (azure) => {
      const result = await runPush(
        azureWriter(azure),
        read,
        (taskId, link) => {
          const doc = read();
          if (doc.azure) write({ ...doc, azure: { ...doc.azure, links: { ...doc.azure.links, [taskId]: link } } });
        },
        plan.changes.filter((c) => chosen[c.taskId]),
        plan.unlinked.filter((u) => toCreate[u.id]).map((u) => u.id),
      );
      messages = [
        ...messages,
        t('azure.sent', { updated: result.updated, created: result.created, comments: result.comments }),
        ...result.errors.map((e) => `${e.taskId}: ${errorText(e.error)}`),
      ];
    });
  }

  const taskTitle = (id: string) => read().tasks.find((task) => task.id === id)?.title ?? id;
</script>

<div class="azure-sync">
  <span class="azure-name">Azure DevOps</span>
  <button type="button" class="azure-pull" disabled={busy} title={t('azure.pullHint')} onclick={pull}>
    <CloudDownload size={14} /> {t('azure.pull')}
  </button>
  <button type="button" class="azure-push" disabled={busy} title={t('azure.pushHint')} onclick={preparePush}>
    <CloudUpload size={14} /> {t('azure.push')}
  </button>
  <button type="button" class="azure-settings" disabled={busy} title={t('azure.settings')} aria-label={t('azure.settings')} onclick={openSettings}>
    <Settings2 size={14} />
  </button>
</div>

{#if busy || messages.length}
  <div class="azure-status" role="status">
    <div>
      {#if busy}<p>{t('azure.syncing')}</p>{/if}
      {#each messages as message, i (i)}<p>{message}</p>{/each}
    </div>
    {#if !busy}
      <button type="button" title={t('common.close')} aria-label={t('common.close')} onclick={() => (messages = [])}><X size={13} /></button>
    {/if}
  </div>
{/if}

<Dialog.Root open={settings !== null} onOpenChange={(open) => { if (!open) settings = null; }}>
  <Dialog.Portal>
    <Dialog.Overlay class="azure-overlay" />
    <Dialog.Content class="azure-dialog synapse-dialog">
      <Dialog.Title class="azure-title">{t('azure.settingsTitle')}</Dialog.Title>
      {#if settings}
        <form onsubmit={saveSettings}>
          <label>
            {t('azure.projectUrl')}
            <input name="project_url" type="url" required placeholder="https://dev.azure.com/organization/project" bind:value={settings.project_url} />
          </label>
          <AzureTokenField bind:this={tokenField} />
          <label>
            {t('azure.workItemType')}
            <input name="work_item_type" placeholder={DEFAULT_WORK_ITEM_TYPE} bind:value={settings.work_item_type} />
          </label>
          <label>
            {t('azure.assignedTo')}
            <input name="assigned_to" bind:value={settings.assigned_to} />
          </label>
          <label>
            {t('azure.query')}
            <textarea name="query" rows="6" placeholder={DEFAULT_QUERY} bind:value={settings.query}></textarea>
          </label>
          {#if read().azure?.links && read().azure?.project_url !== settings.project_url.trim()}
            <p class="azure-note warning">{t('azure.linksCleared')}</p>
          {/if}
          <div class="azure-actions">
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
    <Dialog.Overlay class="azure-overlay" />
    <Dialog.Content class="azure-dialog synapse-dialog">
      <Dialog.Title class="azure-title">{t('azure.reviewTitle')}</Dialog.Title>
      {#if review}
        <p class="azure-note">{read().azure?.project_url}</p>
        <div class="azure-list">
          {#each review.changes as change (change.taskId)}
            <label class="azure-change" data-task={change.taskId}>
              <input type="checkbox" bind:checked={chosen[change.taskId]} />
              <span>
                <strong>#{change.item.id} {taskTitle(change.taskId)}</strong>
                {#if change.title !== undefined}<small>{t('azure.titleChange', { from: change.item.title, to: change.title })}</small>{/if}
                {#if change.state !== undefined}<small>{t('azure.stateChange', { from: change.item.state, to: change.state })}</small>{/if}
                {#each change.comments as comment (comment.timestamp)}<small>{t('azure.commentChange', { text: comment.text })}</small>{/each}
                {#if change.conflict}<small class="warning">{t('azure.conflictChange')}</small>{/if}
              </span>
            </label>
          {/each}
          {#if review.unlinked.length}
            <h4>{t('azure.createHeading', { type: read().azure?.work_item_type || DEFAULT_WORK_ITEM_TYPE })}</h4>
            {#each review.unlinked as task (task.id)}
              <label class="azure-create" data-task={task.id}>
                <input type="checkbox" bind:checked={toCreate[task.id]} />
                <span>
                  <strong>{task.title}</strong>
                  {#if task.comments.length}<small>{t('azure.createComments', { count: task.comments.length })}</small>{/if}
                </span>
              </label>
            {/each}
          {/if}
        </div>
        <div class="azure-actions">
          <button type="button" onclick={() => (review = null)}>{t('common.cancel')}</button>
          <button type="button" class="primary azure-confirm" disabled={selection === 0} onclick={send}>
            {t('azure.confirm', { count: selection })}
          </button>
        </div>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  .azure-sync {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }

  .azure-name {
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

  .azure-status {
    flex-basis: 100%;
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 12px;
    color: var(--text-secondary);
    user-select: text;
  }

  .azure-status p {
    margin: 2px 0;
  }

  .azure-status button {
    align-self: flex-start;
    border: none;
    padding: 2px;
  }

  :global(.azure-overlay) {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    z-index: 10001;
  }

  :global(.azure-dialog) {
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

  :global(.azure-title) {
    font-size: 15px;
    font-weight: 600;
  }

  form,
  .azure-list {
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

  .azure-list {
    overflow: auto;
  }

  .azure-list label {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }

  .azure-list span {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-wrap: anywhere;
  }

  .azure-note,
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

  .azure-actions {
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
