import { invokeTauri } from '@shared/api';
import type { RemoteEntry } from '../model/activity';
import type { Criterion, Issue, IssueFilter, IssueProvider, IssueSource, IssueWriter } from '../model/issue-sync';

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
  return invokeTauri<{ items: Issue[]; requested: string[]; matched: string[]; editable: string[] }>('issues_fetch', {
    ...target(source),
    query,
    linked,
  });
}

/** Los comentarios y el histórico de las tareas pedidas. */
export const fetchActivity = (source: IssueSource, ids: string[]) =>
  invokeTauri<{ items: Record<string, RemoteEntry[]>; failed: { id: string; error: IssueError }[] }>('issues_activity', {
    ...target(source),
    ids,
  });

/** Los valores elegibles de un criterio. */
export const filterValues = (source: IssueSource, criterion: Criterion) =>
  invokeTauri<string[]>('issues_filter_values', { ...target(source), criterion });

/** La consulta del proveedor para un filtro, con la iteración en curso a día de hoy. */
export const filterQuery = (source: IssueSource, filter: IssueFilter) =>
  invokeTauri<string>('issues_filter_query', {
    ...target(source),
    filter,
    today: new Date().toLocaleDateString('sv-SE'),
  });

export function issueWriter(source: IssueSource): IssueWriter {
  const to = target(source);
  return {
    update: (id, revision, changes) => invokeTauri('issues_update', { ...to, id, revision, changes }),
    create: (type, title, extra) =>
      invokeTauri('issues_create', {
        ...to,
        issue: { type, title, assignee: extra?.assignee || source.assignee || null, description: extra?.description ?? null },
      }),
    comment: (id, text) => invokeTauri('issues_add_comment', { ...to, id, text }),
    editComment: (id, comment, text) => invokeTauri('issues_update_comment', { ...to, id, comment, text }),
  };
}
