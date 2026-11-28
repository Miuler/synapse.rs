// Pinta en una nota ya renderizada los bloques `board` y los enlaces a tareas, y los vuelve a pintar
// cuando cambia su tablero. Todo el texto entra como texto (`textContent`): lo que venga de un
// proveedor no puede ejecutar nada ni cargar recursos.
import { reasonText, t } from '@shared/lib/i18n';
import { referencedBoards } from '../api/board.api';
import { sharedReads } from '../lib/shared-reads';
import { boardRefresher, requestBoardTask } from '../model/actions';
import {
  CATEGORIES,
  parseBoardBlock,
  resolveQuery,
  resolveTask,
  taskReference,
  type ReferenceError,
  type ReferenceField,
  type ReferencedBoard,
  type Resolved,
  type Result,
  type Row,
} from '../model/query';
import './board-reference.css';

interface Loaded {
  /** Ruta del `.board.md` en la bóveda. */
  path: string;
  data: ReferencedBoard;
}

/** Lo que se necesita de la bóveda; lo pone quien pinta la nota. */
export interface BoardVault {
  resolveVaultLinkDetailed(
    link: string,
    baseFile?: string | null,
  ): Promise<{ status: 'resolved'; path: string } | { status: 'ambiguous'; candidates: string[] } | { status: 'missing' }>;
  onFilesChanged(callback: (payload: { paths: string[]; deleted: string[] }) => void): Promise<() => void>;
}

type Loader = (name: string) => Promise<Result<Loaded>>;

const failure = (kind: ReferenceError['kind'], detail: string, allowed?: string[]) =>
  ({ ok: false, error: { kind, detail, allowed } }) as const;

const readBoard = sharedReads(referencedBoards.read);
/** Dos bloques del mismo tablero no lo traen dos veces a la vez. */
const refreshOnce = sharedReads((path: string) => boardRefresher()!(path));

/** Lo que cuenta un bloque sobre su última actualización. */
interface Refresh {
  busy: boolean;
  message?: string;
  failed?: boolean;
}
const refreshes = new WeakMap<HTMLElement, Refresh>();

async function load(vault: BoardVault, name: string, notePath: string | null): Promise<Result<Loaded>> {
  const base = name.replace(/\.md$/i, '').replace(/\.board$/i, '');
  const found = await vault.resolveVaultLinkDetailed(`${base}.board.md`, notePath);
  if (found.status === 'missing') return failure('missing_board', name);
  if (found.status === 'ambiguous') return failure('ambiguous_board', name, found.candidates);
  try {
    return { ok: true, value: { path: found.path, data: await readBoard(found.path) } };
  } catch (error) {
    const { kind, detail } = error as { kind?: string; detail?: string };
    return failure(kind === 'not_folder_board' ? 'table_board' : 'unreadable', reasonText(detail || name));
  }
}

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className = '', text?: string) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function errorText({ kind, detail, allowed }: ReferenceError): string {
  const text = t(`boardRef.errors.${kind}`, { detail });
  return allowed?.length ? `${text} ${t('boardRef.allowed', { values: allowed.join(', ') })}` : text;
}

/** Con espacios de verdad entre las partes: así se lee y se copia como una frase. */
const spaced = (nodes: (Node | string)[]) => nodes.flatMap((node, i) => (i ? [' ', node] : [node]));

/** Un enlace que abre como cualquier otro de la nota. */
function link(path: string, text: string, className = '') {
  const anchor = element('a', `internal-link ${className}`.trim(), text);
  anchor.setAttribute('href', encodeURI(path));
  anchor.dataset.internalPath = path;
  return anchor;
}

/** Un enlace que abre el tablero en una tarea. */
function opensTask(anchor: HTMLElement, board: string, id: string) {
  anchor.setAttribute('href', encodeURI(board));
  anchor.dataset.internalPath = board;
  anchor.dataset.boardTaskId = id;
  return anchor;
}

function value(row: Row, field: ReferenceField): string {
  if (field === 'progress') return `${row.progress}%`;
  if (field === 'duration') return t('board.detail.days', { count: row.duration });
  if (field === 'updated') return row.updated.slice(0, 10);
  return String(row[field]);
}

function cell(board: string, row: Row, field: ReferenceField): HTMLElement {
  if (field === 'key') return opensTask(link(board, row.key, 'board-ref-key'), board, row.id);
  if (field === 'state') return element('span', `board-ref-state is-${row.category}`, row.state);
  return element('span', `board-ref-${field}`, value(row, field));
}

function fieldsOf(board: string, row: Row, fields: ReferenceField[]) {
  return fields.filter((field) => value(row, field) !== '').map((field) => cell(board, row, field));
}

function summary({ summary }: Resolved) {
  const box = element('div', 'board-ref-summary');
  const bar = element('div', 'board-ref-bar');
  const done = element('div');
  done.style.width = `${summary.progress}%`;
  bar.append(done);
  const counts = element('ul', 'board-ref-counts');
  for (const category of CATEGORIES) {
    if (category === 'cancelled' && !summary.counts.cancelled) continue;
    const item = element('li', `is-${category}`);
    // «Hechas: 1»: así el nombre no tiene que concordar con el número
    item.append(`${t(`boardRef.categories.${category}`)}: `, element('strong', '', String(summary.counts[category])));
    counts.append(item);
  }
  const text = t('boardRef.progress', { percent: summary.progress, done: summary.counts.done, total: summary.total });
  box.append(element('p', 'board-ref-percent', text), bar, counts);
  return box;
}

function list(board: string, resolved: Resolved) {
  return resolved.groups.flatMap((group) => {
    const items = element('ul', 'board-ref-list');
    for (const row of group.rows) {
      const item = element('li');
      item.dataset.task = row.id;
      item.append(...spaced(fieldsOf(board, row, resolved.fields)));
      items.append(item);
    }
    return group.label ? [element('p', 'board-ref-group', group.label), items] : [items];
  });
}

function table(board: string, resolved: Resolved) {
  const node = element('table', 'board-ref-table');
  const head = node.createTHead().insertRow();
  for (const field of resolved.fields) head.append(element('th', '', t(`boardRef.fields.${field}`)));
  const body = node.createTBody();
  for (const group of resolved.groups) {
    if (group.label) {
      const title = element('td', 'board-ref-group', group.label);
      title.colSpan = resolved.fields.length;
      body.insertRow().append(title);
    }
    for (const row of group.rows) {
      const line = body.insertRow();
      line.dataset.task = row.id;
      for (const field of resolved.fields) line.insertCell().append(cell(board, row, field));
    }
  }
  return node;
}

function kanban(board: string, resolved: Resolved) {
  const node = element('div', 'board-ref-kanban');
  for (const group of resolved.groups) {
    const column = element('div', 'board-ref-column');
    column.append(element('p', 'board-ref-group', `${group.label || t('boardRef.noColumn')} (${group.rows.length})`));
    for (const row of group.rows) {
      const card = element('div', 'board-ref-card');
      card.dataset.task = row.id;
      card.append(...fieldsOf(board, row, resolved.fields));
      column.append(card);
    }
    node.append(column);
  }
  return node;
}

/** «Actualizar», que es el «Traer» del tablero y solo ocurre al pulsarlo. */
function refreshButton(box: HTMLElement, path: string, repaint: () => void) {
  const busy = !!refreshes.get(box)?.busy;
  const button = element('button', 'board-ref-refresh', t(busy ? 'boardRef.refreshing' : 'boardRef.refresh'));
  button.type = 'button';
  button.disabled = busy;
  button.addEventListener('click', (event) => {
    event.preventDefault();
    // En la vista en vivo, un clic en el bloque lleva a editarlo: este no
    event.stopPropagation();
    refreshes.set(box, { busy: true });
    repaint();
    refreshOnce(path)
      .then((message) => refreshes.set(box, { busy: false, message }))
      .catch((error: Error) => refreshes.set(box, { busy: false, message: error.message, failed: true }))
      .finally(() => {
        readBoard.forget();
        repaint();
      });
  });
  return button;
}

async function renderBlock(box: HTMLElement, loadBoard: Loader, repaint: () => void) {
  const parsed = parseBoardBlock(decodeURIComponent(box.dataset.code ?? ''));
  const found = parsed.ok ? await loadBoard(parsed.value.board) : parsed;
  if (!parsed.ok || !found.ok) {
    const error = !parsed.ok ? parsed.error : !found.ok ? found.error : null;
    box.replaceChildren(element('p', 'board-ref-error', error ? errorText(error) : ''));
    return;
  }
  const { path, data } = found.value as Loaded;
  const resolved = resolveQuery(data, parsed.value);
  const header = element('div', 'board-ref-header');
  const date = resolved.syncedAt
    ? t('boardRef.synced', { date: resolved.syncedAt.replace('T', ' ').slice(0, 16) })
    : t('boardRef.neverSynced');
  header.append(link(path, path.split('/').pop()!.replace(/\.board\.md$/i, ''), 'board-ref-name'), element('small', '', date));
  if (boardRefresher() && data.board.source?.provider) header.append(refreshButton(box, path, repaint));
  const refresh = refreshes.get(box);
  const status = refresh?.message
    ? [element('p', refresh.failed ? 'board-ref-error' : 'board-ref-status', refresh.message)]
    : [];
  let content: Node[];
  if (resolved.view === 'summary') content = [summary(resolved)];
  else if (!resolved.summary.total && resolved.view !== 'kanban') content = [element('p', 'board-ref-empty', t('boardRef.empty'))];
  else if (resolved.view === 'table') content = [table(path, resolved)];
  else if (resolved.view === 'kanban') content = [kanban(path, resolved)];
  else content = list(path, resolved);
  box.replaceChildren(header, ...status, ...content);
}

async function renderTask(anchor: HTMLElement, loadBoard: Loader) {
  const target = anchor.dataset.boardTarget!;
  const reference = taskReference(target)!;
  const written = anchor.dataset.boardText!;
  const found = await loadBoard(reference.board);
  const task = found.ok ? resolveTask(found.value.data, reference.task) : found;
  anchor.classList.add('board-task-ref');
  if (!task.ok) {
    anchor.replaceChildren(...spaced([written, element('span', 'board-ref-error', errorText(task.error))]));
    return;
  }
  const row = task.value as Row;
  if (found.ok) opensTask(anchor, found.value.path, row.id);
  // Un texto propio (`[[…|así]]`) se respeta
  const plain = taskReference(written);
  const own = !plain || plain.task.toLowerCase() !== reference.task.toLowerCase();
  const parts = own
    ? [written]
    : [element('span', 'board-ref-key', row.key), element('span', 'board-ref-title', row.title)];
  anchor.replaceChildren(...spaced([...parts, element('span', `board-ref-state is-${row.category}`, row.state)]));
}

const decoded = (path: string) => {
  try {
    return decodeURIComponent(path);
  } catch {
    return path;
  }
};

/**
 * Pinta las referencias que haya dentro de `root`; `painted` avisa cada vez que una queda pintada.
 * Devuelve cómo dejar de vigilar sus tableros.
 */
export function mountBoardReferences(
  root: HTMLElement,
  notePath: string | null,
  vault: BoardVault,
  painted: () => void = () => {},
): () => void {
  const blocks = [...root.querySelectorAll<HTMLElement>('.board-ref[data-code]')];
  const links = [...root.querySelectorAll<HTMLElement>('a.internal-link[data-internal-path]')].filter((anchor) => {
    const target = decoded(anchor.dataset.internalPath!);
    if (!taskReference(target) || anchor.closest('.board-ref')) return false;
    anchor.dataset.boardTarget = target;
    anchor.dataset.boardText ??= anchor.textContent ?? '';
    return true;
  });
  if (!blocks.length && !links.length) return () => {};

  let stopped = false;
  // Antes de que el enlace navegue, se anota en qué tarea abrir el tablero
  const open = (event: Event) => {
    const anchor = (event.target as HTMLElement).closest?.<HTMLElement>('a[data-board-task-id]');
    if (anchor) requestBoardTask(anchor.dataset.internalPath!, anchor.dataset.boardTaskId!);
  };
  root.addEventListener('click', open, true);

  const paint = () => {
    // Cada tablero se lee una vez por pasada
    const loads = new Map<string, Promise<Result<Loaded>>>();
    const loadBoard: Loader = (name) => {
      if (!loads.has(name)) loads.set(name, load(vault, name, notePath));
      return loads.get(name)!;
    };
    for (const box of blocks) void renderBlock(box, loadBoard, repaint).then(painted);
    for (const anchor of links) void renderTask(anchor, loadBoard).then(painted);
  };
  const repaint = () => {
    if (!stopped) paint();
  };
  paint();

  let timer: ReturnType<typeof setTimeout> | undefined;
  const unlisten = vault.onFilesChanged(({ paths, deleted }) => {
    if (stopped || ![...paths, ...deleted].some((path) => /\.board(\.md$|\/)/i.test(path))) return;
    readBoard.forget();
    clearTimeout(timer);
    timer = setTimeout(paint, 200);
  });
  return () => {
    root.removeEventListener('click', open, true);
    stopped = true;
    clearTimeout(timer);
    void unlisten.then((stop) => stop());
  };
}
