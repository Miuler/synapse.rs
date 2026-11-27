import { describe, expect, it } from 'bun:test';
import { changedIssues, fingerprint, mergeActivity, mergeDescription, receive, type RemoteEntry } from './activity';
import { applyTask, awaitsProviderDescription, editDescription, editEntry, folderDocument, ownDescription, takeProviderDescription, type ActivityEntry, type BoardSnapshot, type TaskFile } from './folder-board';
import { applyPull, buildPushPlan, runPush, type Issue, type IssueProvider, type IssueWriter } from './issue-sync';

const entry = (id: string | null, text: string, at = '2026-01-01T00:00:00', extra: Partial<ActivityEntry> = {}): ActivityEntry => ({
  at, kind: 'comment', author: null, id, edited: false, text, ...extra,
});
const remote = (id: string, text: string, at = '2026-01-02T00:00:00', kind: RemoteEntry['kind'] = 'comment'): RemoteEntry => ({
  id, kind, author: { name: 'Ana' }, at, text,
});
const item = (revision: string, description: string | null = 'desc'): Issue => ({
  id: '1', type: 'Task', title: 'A', state: { name: 'New' }, revision, description,
});
const provider = { id: 'p', default_states: {} } as unknown as IssueProvider;

const board = (link: object = {}, comments: ActivityEntry[] = []): BoardSnapshot => ({
  hash: 'b',
  problems: [],
  board: {
    defaultView: 'kanban', columns: [], columnWidths: {}, fields: {}, extraFrontmatter: '', body: '', order: ['a'],
    source: { provider: 'p', connection: '', scope: 's' },
  },
  tasks: [{
    id: 'a', hash: 'h', title: 'A', status: 'todo', type: null, assignee: null, key: null, url: null, start: '2026-01-01',
    duration: 1, dependsOn: '-', progress: 0, extra: [], comments,
    sync: { id: '1', title: 'A', state: 'New', column: 'todo', ...link },
  }],
});
const file = (extra: Partial<TaskFile> = {}): TaskFile => ({
  title: 'A', status: 'todo', type: null, assignee: null, key: null, url: null, start: '2026-01-01', duration: 1,
  dependsOn: '-', progress: 0, sync: null, extra: [], description: '', activity: [], preamble: '', sections: '', ...extra,
});

describe('actividad traída', () => {
  it('añade lo nuevo, actualiza lo cambiado y respeta lo editado aquí', () => {
    const local = [
      entry('1', 'antes'),
      entry('rev-1', 'corregido a mano', '2026-01-01T00:00:00', { kind: 'history', edited: true }),
      entry(null, 'sin enviar', '2026-01-03T00:00:00'),
    ];
    const merged = mergeActivity(local, [
      entry('1', 'después'),
      entry('rev-1', 'original', '2026-01-01T00:00:00', { kind: 'history' }),
      entry('2', 'nuevo', '2026-01-02T00:00:00'),
    ]);
    expect(merged.map((e) => e.text)).toEqual(['después', 'corregido a mano', 'nuevo', 'sin enviar']);
  });

  it('reconoce lo que envió sin saber su id y no confunde lo que no se envió', () => {
    const local = [entry(null, 'a * b\nfin', 't1'), entry(null, 'otro', 't2')];
    const merged = mergeActivity(local, [entry('9', 'a \\* b  \nfin', 'r1'), entry('10', 'otro', 'r2')], ['t1']);
    expect(merged.map((e) => [e.id, e.at])).toEqual([['10', 'r2'], ['9', 't1'], [null, 't2']]);
  });

  it('la descripción se toma solo si aquí no cambió', () => {
    const base = fingerprint('uno');
    expect(mergeDescription('', 'nueva')).toEqual({ base: fingerprint('nueva'), take: 'nueva' });
    expect(mergeDescription('uno  \n', 'dos', base)).toEqual({ base: fingerprint('dos'), take: 'dos' });
    expect(mergeDescription('mía', 'uno', base)).toEqual({ base });
    expect(mergeDescription('mía', 'dos', base)).toEqual({ base, conflict: true });
    expect(mergeDescription('mía', 'dos')).toEqual({ base: undefined, conflict: true });
    expect(mergeDescription('dos', 'dos', base)).toEqual({ base: fingerprint('dos') });
  });

  it('solo pide lo que cambió en el proveedor', () => {
    const doc = folderDocument(board({ rev: '3' }));
    expect(changedIssues(doc, [item('3')])).toEqual([]);
    expect(changedIssues(doc, [item('4')])).toEqual(['1']);
    expect(changedIssues(folderDocument(board()), [item('3')])).toEqual(['1']);
    expect(changedIssues(doc, [])).toEqual([]);
    // Un comentario nuevo se nota aunque la revisión no cambie
    const counted = { ...item('3'), extra: { comment_count: '2' } };
    expect(changedIssues(folderDocument(board({ rev: '3+1' })), [counted])).toEqual(['1']);
    expect(changedIssues(folderDocument(board({ rev: '3+2' })), [counted])).toEqual([]);
  });

  it('lo traído llega al archivo, y sin actividad no se anota la revisión', () => {
    const before = folderDocument(board());
    expect(receive(before, [item('4')], {}, {}).doc.source!.links!.a.rev).toBeUndefined();
    const stored = file();
    const { doc, changed, conflicts } = receive(before, [item('4')], { '1': [remote('7', 'hola')] }, { a: stored });
    expect([changed, conflicts]).toEqual([['a'], []]);
    expect(doc.source!.links!.a).toMatchObject({ rev: '4', description: fingerprint('desc') });
    const written = applyTask(stored, { task: before.tasks[0], link: before.source!.links!.a }, { task: doc.tasks[0], link: doc.source!.links!.a });
    expect(written.description).toBe('desc');
    expect(written.activity).toEqual([{ at: '2026-01-02T00:00:00', kind: 'comment', author: 'Ana', id: '7', edited: false, text: 'hola' }]);
    expect(written.sync).toMatchObject({ rev: '4', description: fingerprint('desc') });
    // Si entretanto cambió la de aquí, no se pisa
    const edited = applyTask(file({ description: 'otra' }), { task: before.tasks[0] }, { task: doc.tasks[0] });
    expect(edited.description).toBe('otra');
  });

  it('lo traído no se envía, y lo enviado queda con su id', async () => {
    let doc = folderDocument(board({}, [entry('7', 'de Ana', 't0'), entry(null, 'mío', 't1')]));
    const plan = buildPushPlan(doc, provider, [item('4')]);
    expect(plan.changes[0].comments.map((c) => c.text)).toEqual(['mío']);
    const saved: unknown[] = [];
    const writer = { comment: async () => '8' } as unknown as IssueWriter;
    await runPush(provider, writer, () => doc, (_, link, sent) => saved.push([link.sent, sent]), plan.changes, []);
    expect(saved).toEqual([[['t1'], { timestamp: 't1', text: 'mío', remote: '8' }]]);

    const task = { ...doc.tasks[0], comments: doc.tasks[0].comments.map((c) => (c.timestamp === 't1' ? { ...c, remote: '8' } : c)) };
    const written = applyTask(file({ activity: [entry('7', 'de Ana', 't0'), entry(null, 'mío', 't1')] }), { task: doc.tasks[0] }, { task });
    expect(written.activity.map((e) => e.id)).toEqual(['7', '8']);
  });
});

describe('editar en el detalle', () => {
  it('la descripción anota de cuál se partió, para que traer no la pise', () => {
    const stored = file({ description: 'del proveedor', sync: { id: '1', title: 'A', state: 'New', column: 'todo' } });
    const edited = editDescription(stored, 'del proveedor', 'mía');
    expect([edited.description, edited.sync!.description]).toEqual(['mía', fingerprint('del proveedor')]);
    expect(mergeDescription(edited.description, 'del proveedor', edited.sync!.description)).toEqual({ base: edited.sync!.description });
    expect(mergeDescription(edited.description, 'otra', edited.sync!.description).conflict).toBe(true);
    expect(() => editDescription(stored, 'otra cosa', 'mía')).toThrow();
  });

  it('se puede volver a la del proveedor: el siguiente «Traer» la pone', () => {
    const sync = { id: '1', title: 'A', state: 'New', column: 'todo', rev: '4', description: fingerprint('vieja') };
    const stored = file({ description: 'mía', sync });
    expect(ownDescription(stored)).toBe(true);
    const marked = takeProviderDescription(stored, 'mía');
    expect([awaitsProviderDescription(marked), ownDescription(marked), marked.sync!.rev]).toEqual([true, false, undefined]);
    // Sin revisión se vuelve a pedir, y la de aquí ya no cuenta como cambiada
    const doc = folderDocument(board(marked.sync!));
    expect(changedIssues(doc, [item('4')])).toEqual(['1']);
    const { conflicts, doc: next } = receive(doc, [item('4', 'del proveedor')], { '1': [] }, { a: marked });
    expect(conflicts).toEqual([]);
    expect(next.tasks[0].incoming!.description).toBe('del proveedor');
  });

  it('marca lo que también está en el proveedor y traer no lo pisa', () => {
    const history = entry('rev-1', 'original', 't', { kind: 'history' });
    const mine = entry(null, 'mío', 'u');
    const stored = file({ activity: [history, mine] });
    const edited = editEntry(editEntry(stored, history, 'corregido'), mine, 'mío 2');
    expect(edited.activity.map((e) => [e.text, e.edited])).toEqual([['corregido', true], ['mío 2', false]]);
    const pulled = mergeActivity(edited.activity, [entry('rev-1', 'original', 't', { kind: 'history' })]);
    expect(pulled[0].text).toBe('corregido');
    expect(() => editEntry(edited, history, 'x')).toThrow();
  });

  it('traer conserva el tipo y el asignado cambiados aquí', () => {
    const read = board({ type: 'Task', assignee: '' });
    read.tasks[0] = { ...read.tasks[0], type: 'Bug', assignee: 'Eva' };
    const doc = folderDocument(read);
    const pulled = (type: string, name: string) =>
      applyPull(doc, provider, [{ ...item('4'), type, assignee: { name, id: name } }], ['1'], '2026-01-01');

    // El proveedor sigue como estaba: se queda lo de aquí, sin conflicto
    const same = pulled('Task', '');
    expect([same.doc.tasks[0].type, same.doc.tasks[0].assignee, same.conflicts]).toEqual(['Bug', 'Eva', []]);
    expect(same.doc.source!.links!.a).toMatchObject({ type: 'Task', assignee: '' });
    // Cambió en los dos lados
    const both = pulled('Epic', 'Ana');
    expect([both.doc.tasks[0].type, both.doc.tasks[0].assignee, both.conflicts]).toEqual(['Bug', 'Eva', ['a']]);
    // Ya coinciden: no queda nada pendiente
    const agreed = pulled('Bug', 'Eva');
    expect(agreed.doc.source!.links!.a.type).toBeUndefined();
    expect(agreed.doc.source!.links!.a.assignee).toBeUndefined();
  });

  it('sin cambios locales, el tipo y el asignado siguen al proveedor', () => {
    const doc = folderDocument(board());
    const result = applyPull(doc, provider, [{ ...item('4'), type: 'Epic', assignee: { name: 'Ana', id: 'a' } }], ['1'], '2026-01-01');
    expect([result.doc.tasks[0].type, result.doc.tasks[0].assignee]).toEqual(['Epic', 'Ana']);
  });
});
