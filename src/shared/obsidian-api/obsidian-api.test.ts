import { describe, expect, it } from 'bun:test';
import { createSynapseApp } from './app';
import type { VaultFile, VaultNote, VaultRepository } from '@shared/repositories';

class MockVaultRepository implements Partial<VaultRepository> {
  notes: Record<string, VaultNote> = {
    'notas/bienvenida.md': {
      title: 'Bienvenida',
      relative_path: 'notas/bienvenida.md',
      content: '---\nautor: Miuler\nversion: 1\n---\n# Título Principal\nContenido con [[otra-nota]] y ![[dibujo.excalidraw]] #tag1',
    },
    'dibujos/canvas.excalidraw': {
      title: 'Canvas',
      relative_path: 'dibujos/canvas.excalidraw',
      content: '{"elements": []}',
    },
  };

  isConnected() {
    return true;
  }

  async getNoteMeta(relativePath: string): Promise<VaultFile | null> {
    const note = this.notes[relativePath];
    if (!note) return null;
    const path = note.relative_path;
    const name = path.split('/').pop() || path;
    const dot = name.lastIndexOf('.');
    return {
      path,
      name,
      basename: dot > 0 ? name.slice(0, dot) : name,
      extension: dot > 0 ? name.slice(dot + 1) : '',
      parent: path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : undefined,
      stat: { ctime: 1000, mtime: 2000, size: (note.content ?? '').length },
      view_mode: 'reading',
    };
  }

  async getVaultFiles(): Promise<VaultFile[]> {
    return Object.keys(this.notes).map((p) => {
      const note = this.notes[p];
      const name = p.split('/').pop() || p;
      const dot = name.lastIndexOf('.');
      return {
        path: p,
        name,
        basename: dot > 0 ? name.slice(0, dot) : name,
        extension: dot > 0 ? name.slice(dot + 1) : '',
        parent: p.includes('/') ? p.slice(0, p.lastIndexOf('/')) : undefined,
        stat: { ctime: 1000, mtime: 2000, size: (note.content ?? '').length },
      };
    });
  }

  async readNote(relativePath: string): Promise<VaultNote | null> {
    return this.notes[relativePath] ?? null;
  }

  async saveNote(params: { relativePath: string; title: string; content: string }) {
    this.notes[params.relativePath] = {
      title: params.title,
      relative_path: params.relativePath,
      content: params.content,
    };
  }

  async deleteItem(relativePath: string) {
    delete this.notes[relativePath];
  }

  async renameItem(relativePath: string, newName: string): Promise<string> {
    const note = this.notes[relativePath];
    if (note) {
      delete this.notes[relativePath];
      const dir = relativePath.includes('/') ? relativePath.slice(0, relativePath.lastIndexOf('/')) : '';
      const newPath = dir ? `${dir}/${newName}` : newName;
      this.notes[newPath] = { ...note, relative_path: newPath };
      return newPath;
    }
    return relativePath;
  }

  async copyItems(paths: string[], destDir: string): Promise<string[]> {
    const copied: string[] = [];
    for (const p of paths) {
      const note = this.notes[p];
      if (note) {
        const name = p.split('/').pop() || p;
        const newPath = destDir ? `${destDir}/${name}` : name;
        this.notes[newPath] = { ...note, relative_path: newPath };
        copied.push(newPath);
      }
    }
    return copied;
  }

  resolveAssetUrl(path: string): string {
    return `asset://localhost/${path}`;
  }

  async getOpenTabsState() {
    return {
      open_tabs: [{ path: 'notas/bienvenida.md', view_mode: 'reading' }],
      active_tab: 'notas/bienvenida.md',
    };
  }

  async resolveVaultLink(link: string, _baseFile?: string | null) {
    const clean = link.replace(/\.(md|excalidraw)$/i, '').toLowerCase();
    for (const p of Object.keys(this.notes)) {
      if (p.toLowerCase().includes(clean)) return p;
    }
    return null;
  }
}

describe('Synapse Obsidian API Facade (Etapa 1)', () => {
  const repo = new MockVaultRepository() as unknown as VaultRepository;
  const app = createSynapseApp(repo);

  it('1. app.vault.getAbstractFileByPath realiza lookup O(1) vía DashMap', async () => {
    const file = await app.vault.getAbstractFileByPath('notas/bienvenida.md');
    expect(file).not.toBeNull();
    expect(file?.path).toBe('notas/bienvenida.md');
    expect(file?.basename).toBe('bienvenida');
    expect(file?.extension).toBe('md');
    expect(file?.parent).toBe('notas');

    const nonExistent = await app.vault.getAbstractFileByPath('inexistente.md');
    expect(nonExistent).toBeNull();
  });

  it('2. app.vault.getFiles y getMarkdownFiles retornan archivos correspondientes', async () => {
    const files = await app.vault.getFiles();
    expect(files.length).toBe(2);

    const mdFiles = await app.vault.getMarkdownFiles();
    expect(mdFiles.length).toBe(1);
    expect(mdFiles[0].path).toBe('notas/bienvenida.md');
  });

  it('3. app.vault.read y modify manipulan el contenido de las notas', async () => {
    const content = await app.vault.read('dibujos/canvas.excalidraw');
    expect(content).toBe('{"elements": []}');

    await app.vault.modify('dibujos/canvas.excalidraw', '{"elements": [{"id": 1}]}');
    const updated = await app.vault.read('dibujos/canvas.excalidraw');
    expect(updated).toBe('{"elements": [{"id": 1}]}');
  });

  it('4. app.vault.create, delete y exists', async () => {
    const existsBefore = await app.vault.exists('nuevo.md');
    expect(existsBefore).toBe(false);

    const created = await app.vault.create('nuevo.md', '# Nuevo');
    expect(created.path).toBe('nuevo.md');

    const existsAfter = await app.vault.exists('nuevo.md');
    expect(existsAfter).toBe(true);

    await app.vault.delete('nuevo.md');
    const existsFinal = await app.vault.exists('nuevo.md');
    expect(existsFinal).toBe(false);
  });

  it('5. app.vault.getResourcePath genera URLs locales de Tauri', () => {
    const url = app.vault.getResourcePath('imagen.png');
    expect(url).toBe('asset://localhost/imagen.png');
  });

  it('6. app.workspace.getActiveFile retorna la pestaña activa', async () => {
    const active = await app.workspace.getActiveFile();
    expect(active).not.toBeNull();
    expect(active?.path).toBe('notas/bienvenida.md');
  });

  it('7. app.metadataCache.getFileCache parsea frontmatter, encabezados, links, embeds y tags', async () => {
    const cache = await app.metadataCache.getFileCache('notas/bienvenida.md');
    expect(cache).not.toBeNull();
    expect(cache?.frontmatter?.autor).toBe('Miuler');
    expect(cache?.frontmatter?.version).toBe(1);
    expect(cache?.headings.length).toBe(1);
    expect(cache?.headings[0].heading).toBe('Título Principal');
    expect(cache?.links.some((l: any) => l.link === 'otra-nota')).toBe(true);
    expect(cache?.embeds.some((e: any) => e.link === 'dibujo.excalidraw')).toBe(true);
    expect(cache?.tags).toContain('#tag1');
  });

  it('8. app.fileManager.processFrontMatter actualiza frontmatter sin corromper el cuerpo', async () => {
    await app.fileManager.processFrontMatter('notas/bienvenida.md', (fm) => {
      fm.categoria = 'documentacion';
      fm.editado = true;
    });

    const updatedContent = await app.vault.read('notas/bienvenida.md');
    expect(updatedContent).toContain('categoria: documentacion');
    expect(updatedContent).toContain('editado: true');
    expect(updatedContent).toContain('# Título Principal');
  });

  it('9. app.fileManager.getAvailablePathForAttachment genera rutas sin colisión', async () => {
    const available = await app.fileManager.getAvailablePathForAttachment('adjunto', 'png');
    expect(available).toBe('adjunto.png');
  });
});
