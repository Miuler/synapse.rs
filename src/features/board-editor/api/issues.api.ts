import { invokeTauri } from '@shared/api';
import type { Issue, IssueProvider, IssueSource, IssueWriter } from '../model/issue-sync';

/** Error que devuelve Rust: la clase decide el texto, el detalle lo pone el proveedor. */
export interface IssueError {
  kind: 'missing_token' | 'keyring' | 'token_reference' | 'one_password' | 'host_not_allowed' | 'invalid_url' | 'auth' | 'not_found' | 'conflict' | 'rule' | 'request' | 'unknown_provider' | 'connection_credential' | 'no_connection' | 'ambiguous_connection' | 'no_current_iteration';
  detail: string;
}

export const issueProviders = () => invokeTauri<IssueProvider[]>('issues_providers');

/** A quién se pide: sin conexión elegida, Rust toma la que cubra el ámbito. */
const target = ({ provider, connection, scope }: IssueSource) => ({ provider, connection: connection || null, scope });

/** Las tareas de `query` (vacía, ninguna) y las de `linked`. */
export function fetchIssues(source: IssueSource, query: string, linked: string[]) {
  return invokeTauri<{ items: Issue[]; requested: string[] }>('issues_fetch', { ...target(source), query, linked });
}

export function issueWriter(source: IssueSource): IssueWriter {
  const to = target(source);
  return {
    update: (id, revision, changes) => invokeTauri('issues_update', { ...to, id, revision, changes }),
    create: (type, title) =>
      invokeTauri('issues_create', { ...to, issue: { type, title, assignee: source.assignee || null } }),
    comment: (id, text) => invokeTauri('issues_add_comment', { ...to, id, text }),
  };
}
