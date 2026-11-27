import { invokeTauri } from '@shared/api';
import type { AzureSyncConfig, AzureWriter, WorkItem } from '../model/azure-sync';

/** Error que devuelve Rust: la clase decide el texto, el detalle lo pone Azure DevOps. */
export interface AzureError {
  kind: 'missing_token' | 'host_not_allowed' | 'invalid_url' | 'auth' | 'not_found' | 'conflict' | 'rule' | 'request';
  detail: string;
}

/** Los work items de `query` (vacía, ninguno) y los de `linked`. */
export function fetchWorkItems(projectUrl: string, query: string, linked: number[]) {
  return invokeTauri<{ items: WorkItem[]; requested: number[] }>('azure_fetch_work_items', { projectUrl, query, linked });
}

export function azureWriter(config: AzureSyncConfig): AzureWriter {
  const projectUrl = config.project_url;
  return {
    update: (id, rev, changes) => invokeTauri('azure_update_work_item', { projectUrl, id, rev, ...changes }),
    create: (workItemType, title) =>
      invokeTauri('azure_create_work_item', { projectUrl, workItemType, title, assignedTo: config.assigned_to || null }),
    comment: (id, text) => invokeTauri('azure_add_work_item_comment', { projectUrl, id, text }),
  };
}
