import { describe, expect, it } from 'bun:test';
import { parseBoardMarkdown, serializeBoardMarkdown, type BoardDocument, type BoardTask } from './board.svelte';
import type { IssueLink } from './issue-sync';

const task = (id: string, title: string, status: string): BoardTask => ({
  id,
  title,
  status,
  start: '2026-01-01',
  duration: 1,
  dependsOn: '-',
  progress: 0,
  comments: [],
});

const board = (tasks: BoardTask[], links: Record<string, IssueLink>): BoardDocument => ({
  ...parseBoardMarkdown(''),
  tasks,
  source: { provider: 'azure_devops', connection: '', scope: 'https://dev.azure.com/org/Proyecto', links },
});

const link = (id: number, title: string, state: string, column: string): IssueLink => ({ id: String(id), title, state, column });

describe('el formato anterior del origen', () => {
  const stored = `---
type: board
default_view: kanban
columns:
  - { id: todo, label: "Por hacer", color: "red" }
azure_devops:
  project_url: https://dev.azure.com/org/Proyecto
  work_item_type: Bug
  clave_propia: se conserva
  links:
    t1:
      id: 7
      title: Uno
      state: To Do
      column: todo
---

| id | Tarea | Estado | Inicio | Duración | Depende | Progreso |
|---|---|---|---|---|---|---|
| t1 | Uno | todo | 2026-01-01 | 1d | - | 0% |
`;

  it('lee el formato anterior al modelo común y lo vuelve a escribir igual', () => {
    const doc = parseBoardMarkdown(stored);
    expect(doc.source).toMatchObject({
      provider: 'azure_devops',
      scope: 'https://dev.azure.com/org/Proyecto',
      issue_type: 'Bug',
      links: { t1: link(7, 'Uno', 'To Do', 'todo') },
    });
    expect(serializeBoardMarkdown(doc)).toBe(stored);
  });

  it('guarda la conexión elegida dentro del bloque anterior', () => {
    const doc = parseBoardMarkdown(stored);
    expect(doc.source!.connection).toBe('');
    doc.source!.connection = 'trabajo';
    const text = serializeBoardMarkdown(doc);
    expect(text).toContain('  connection: trabajo\n');
    expect(parseBoardMarkdown(text).source!.connection).toBe('trabajo');
  });

  it('al quitar un campo desaparece del archivo', () => {
    const doc = parseBoardMarkdown(stored);
    delete doc.source!.issue_type;
    delete doc.source!.links;
    const text = serializeBoardMarkdown(doc);
    expect(text).not.toContain('work_item_type');
    expect(text).not.toContain('links:');
    expect(text).toContain('clave_propia: se conserva');
  });

  it('conserva la consulta, los comentarios enviados y las claves ajenas', () => {
    const doc = board([task('t1', 'A | B', 'todo')], {
      t1: { ...link(7, 'A | B', 'To Do', 'todo'), sent: ['2026-01-01T10:00:00'] },
    });
    doc.source!.query = 'SELECT [System.Id]\nFROM WorkItems';
    doc.extraFrontmatter = 'tags:\n  - trabajo';
    const text = serializeBoardMarkdown(doc);
    expect(text).toContain('      id: 7\n');
    const read = parseBoardMarkdown(text);
    expect(read.source).toMatchObject(doc.source!);
    expect(read.extraFrontmatter).toBe(doc.extraFrontmatter);
    expect(read.tasks).toEqual(doc.tasks);
    expect(read.columns).toEqual(doc.columns);
    expect(serializeBoardMarkdown(read)).toBe(text);
  });

  it('un bloque a medio escribir se conserva como texto', () => {
    const raw = '---\ntype: board\nazure_devops: {\n---\n';
    const doc = parseBoardMarkdown(raw);
    expect(doc.source).toBeUndefined();
    expect(serializeBoardMarkdown(doc)).toContain('azure_devops: {');
  });
});
