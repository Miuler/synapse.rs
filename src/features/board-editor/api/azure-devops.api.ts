import { invokeTauri } from '@shared/api';
import type { AzureSyncConfig, AzureWriter, WorkItem } from '../model/azure-sync';

/** Error que devuelve Rust: la clase decide el texto, el detalle lo pone Azure DevOps. */
export interface AzureError {
  kind: 'missing_token' | 'keyring' | 'token_reference' | 'one_password' | 'host_not_allowed' | 'invalid_url' | 'auth' | 'not_found' | 'conflict' | 'rule' | 'request' | 'unknown_provider';
  detail: string;
}

/** La conexión implícita mientras no haya conexiones configurables. */
const connection = 'azure_devops';

/** La parte del modelo común de Rust que el tablero usa todavía. */
interface Issue {
  id: string;
  type: string;
  title: string;
  state: { name: string };
  revision: string | null;
}

const workItem = (issue: Issue): WorkItem => ({
  id: Number(issue.id),
  rev: Number(issue.revision),
  title: issue.title,
  state: issue.state.name,
  work_item_type: issue.type,
});

/** Los work items de `query` (vacía, ninguno) y los de `linked`. */
export async function fetchWorkItems(projectUrl: string, query: string, linked: number[]) {
  const result = await invokeTauri<{ items: Issue[]; requested: string[] }>('issues_fetch', {
    connection,
    scope: projectUrl,
    query,
    linked: linked.map(String),
  });
  return { items: result.items.map(workItem), requested: result.requested.map(Number) };
}

/** De dónde sale el token, sin su valor: una referencia `op://` o uno guardado en el llavero. */
export interface AzureTokenSetting {
  reference: string | null;
  stored: boolean;
}

/** El token es el mismo para todos los tableros. */
export const azureToken = {
  get: () => invokeTauri<AzureTokenSetting>('get_azure_token_setting'),
  /** Una referencia `op://` o un token; vacío, lo quita. Devuelve cómo quedó. */
  set: (value: string) => invokeTauri<AzureTokenSetting>('set_azure_token', { value }),
};

export function azureWriter(config: AzureSyncConfig): AzureWriter {
  const scope = config.project_url;
  return {
    update: (id, rev, changes) =>
      invokeTauri<Issue>('issues_update', { connection, scope, id: String(id), revision: String(rev), changes }).then(workItem),
    create: (type, title) =>
      invokeTauri<Issue>('issues_create', {
        connection,
        scope,
        issue: { type, title, assignee: config.assigned_to || null },
      }).then(workItem),
    comment: (id, text) => invokeTauri('issues_add_comment', { connection, scope, id: String(id), text }),
  };
}
