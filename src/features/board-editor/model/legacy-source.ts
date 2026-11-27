// Formato anterior del origen de un tablero: la clave `azure_devops` del frontmatter. Se lee y se
// escribe tal como era hasta que los tableros se conviertan al formato nuevo (tarea 3.5 de
// add-issue-providers).
import type { IssueLink, IssueSource } from './issue-sync';

export const LEGACY_KEY = 'azure_devops';
const PROVIDER = 'azure_devops';

/** Nombre de cada campo en el archivo. */
const FIELDS = { query: 'query', filter: 'filter', filter_query: 'filter_query', issue_type: 'work_item_type', assignee: 'assigned_to', states: 'states' } as const;

type Stored = Record<string, unknown>;

export function readLegacySource(value: Stored): IssueSource {
  const links = value.links as Record<string, IssueLink> | undefined;
  const source: IssueSource = {
    provider: PROVIDER,
    connection: String(value.connection ?? ''),
    scope: String(value.project_url ?? ''),
    original: value,
  };
  for (const [field, stored] of Object.entries(FIELDS)) {
    if (value[stored] !== undefined) Object.assign(source, { [field]: value[stored] });
  }
  // En el archivo el id es un número; en la aplicación, texto
  if (links) {
    source.links = Object.fromEntries(Object.entries(links).map(([task, link]) => [task, { ...link, id: String(link.id) }]));
  }
  return source;
}

/** El frontmatter de `source`, conservando el orden y las claves del que se leyó. */
export function writeLegacySource(source: IssueSource): Record<string, Stored> {
  const stored: Stored = { ...source.original, project_url: source.scope };
  const set = (key: string, value: unknown) => {
    if (value === undefined) delete stored[key];
    else stored[key] = value;
  };
  set('connection', source.connection || undefined);
  for (const [field, key] of Object.entries(FIELDS)) set(key, source[field as keyof typeof FIELDS]);
  set(
    'links',
    source.links &&
      Object.fromEntries(
        Object.entries(source.links).map(([task, link]) => [
          task,
          { ...link, id: /^\d+$/.test(link.id) ? Number(link.id) : link.id },
        ]),
      ),
  );
  return { [LEGACY_KEY]: stored };
}
