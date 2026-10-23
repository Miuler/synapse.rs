import { describe, expect, it } from 'bun:test';
import { createSynapseApp } from './app';
import { TAbstractFile, TFile, TFolder } from './types';
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

  binaryStore: Record<string, Uint8Array> = {
    'adjuntos/imagen.png': new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]),
  };

  folders = new Set<string>(['notas', 'dibujos', 'adjuntos']);

  isConnected() {
    return true;
  }

  async getNoteMeta(relativePath: string): Promise<VaultFile | null> {
    const note = this.notes[relativePath];
    if (note) {
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

    const bin = this.binaryStore[relativePath];
    if (bin) {
      const path = relativePath;
      const name = path.split('/').pop() || path;
      const dot = name.lastIndexOf('.');
      return {
        path,
        name,
        basename: dot > 0 ? name.slice(0, dot) : name,
        extension: dot > 0 ? name.slice(dot + 1) : '',
        parent: path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : undefined,
        stat: { ctime: 1000, mtime: 2000, size: bin.length },
      };
    }

    return null;
  }

  async getVaultFiles(): Promise<VaultFile[]> {
    const fromNotes = Object.keys(this.notes).map((p) => {
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

    const fromBin = Object.keys(this.binaryStore).map((p) => {
      const bin = this.binaryStore[p];
      const name = p.split('/').pop() || p;
      const dot = name.lastIndexOf('.');
      return {
        path: p,
        name,
        basename: dot > 0 ? name.slice(0, dot) : name,
        extension: dot > 0 ? name.slice(dot + 1) : '',
        parent: p.includes('/') ? p.slice(0, p.lastIndexOf('/')) : undefined,
        stat: { ctime: 1000, mtime: 2000, size: bin.length },
      };
    });

    return [...fromNotes, ...fromBin];
  }

  async getDirectoryChildren(parentPath?: string) {
    const parent = parentPath ? `${parentPath.replace(/^\/+|\/+$/g, '')}/` : '';
    const results: Array<{ name: string; relative_path: string; is_folder: boolean }> = [];

    for (const f of this.folders) {
      if (!parent && !f.includes('/')) {
        results.push({ name: f, relative_path: f, is_folder: true });
      } else if (parent && f.startsWith(parent)) {
        const sub = f.slice(parent.length);
        if (!sub.includes('/')) {
          results.push({ name: sub, relative_path: f, is_folder: true });
        }
      }
    }

    for (const p of [...Object.keys(this.notes), ...Object.keys(this.binaryStore)]) {
      if (parent && p.startsWith(parent)) {
        const sub = p.slice(parent.length);
        if (!sub.includes('/')) {
          results.push({ name: sub, relative_path: p, is_folder: false });
        }
      } else if (!parent && !p.includes('/')) {
        results.push({ name: p, relative_path: p, is_folder: false });
      }
    }
    return results;
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

  async readBinary(relativePath: string): Promise<ArrayBuffer> {
    const data = this.binaryStore[relativePath];
    if (!data) return new ArrayBuffer(0);
    return data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
  }

  async writeBinary(relativePath: string, data: ArrayBuffer | Uint8Array): Promise<void> {
    const u8 = data instanceof Uint8Array ? data : new Uint8Array(data);
    this.binaryStore[relativePath] = u8;
  }

  async createFolder(relativePath: string): Promise<void> {
    this.folders.add(relativePath);
  }

  async deleteItem(relativePath: string) {
    delete this.notes[relativePath];
    delete this.binaryStore[relativePath];
    this.folders.delete(relativePath);
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
    const bin = this.binaryStore[relativePath];
    if (bin) {
      delete this.binaryStore[relativePath];
      const dir = relativePath.includes('/') ? relativePath.slice(0, relativePath.lastIndexOf('/')) : '';
      const newPath = dir ? `${dir}/${newName}` : newName;
      this.binaryStore[newPath] = bin;
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
      const bin = this.binaryStore[p];
      if (bin) {
        const name = p.split('/').pop() || p;
        const newPath = destDir ? `${destDir}/${name}` : name;
        this.binaryStore[newPath] = bin;
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

  async getActiveVaultPath() {
    return '/home/usuario/mi-boveda';
  }
}

describe('Synapse Obsidian API Facade (Etapa 1 & Etapa 2)', () => {
  const repo = new MockVaultRepository() as unknown as VaultRepository;
  const app = createSynapseApp(repo);

  it('1. Clases canónicas TFile y TFolder heredan de TAbstractFile', async () => {
    const file = await app.vault.getAbstractFileByPath('notas/bienvenida.md');
    expect(file).toBeInstanceOf(TAbstractFile);
    expect(file).toBeInstanceOf(TFile);

    const folder = await app.vault.getAbstractFileByPath('notas');
    expect(folder).toBeInstanceOf(TAbstractFile);
    expect(folder).toBeInstanceOf(TFolder);

    const root = app.vault.getRoot();
    expect(root).toBeInstanceOf(TFolder);
    expect(root.isRoot()).toBe(true);
  });

  it('2. I/O Binario nativo: readBinary, modifyBinary y createBinary con ArrayBuffer', async () => {
    // Lectura binaria
    const buffer = await app.vault.readBinary('adjuntos/imagen.png');
    expect(buffer.byteLength).toBe(8);
    const u8 = new Uint8Array(buffer);
    expect(u8[0]).toBe(137);
    expect(u8[1]).toBe(80);

    // Creación binaria
    const newBytes = new Uint8Array([1, 2, 3, 4, 5]);
    const createdBin = await app.vault.createBinary('adjuntos/datos.bin', newBytes.buffer);
    expect(createdBin).toBeInstanceOf(TFile);
    expect(createdBin.path).toBe('adjuntos/datos.bin');

    // Modificación binaria
    const modBytes = new Uint8Array([9, 8, 7]);
    await app.vault.modifyBinary(createdBin, modBytes);
    const readMod = await app.vault.readBinary(createdBin);
    expect(new Uint8Array(readMod)[0]).toBe(9);
  });

  it('3. Métodos polimórficos aceptan tanto instancias TFile como strings', async () => {
    const tfile = await app.vault.getAbstractFileByPath('dibujos/canvas.excalidraw');
    expect(tfile).toBeInstanceOf(TFile);

    if (tfile instanceof TFile) {
      // Usando instancia TFile
      const content = await app.vault.read(tfile);
      expect(content).toBe('{"elements": []}');

      await app.vault.modify(tfile, '{"elements": [100]}');
      expect(await app.vault.read(tfile)).toBe('{"elements": [100]}');
    }

    // Usando string
    expect(await app.vault.read('dibujos/canvas.excalidraw')).toBe('{"elements": [100]}');
  });

  it('4. DataAdapter expuesto en app.vault.adapter cumple con la API de Obsidian', async () => {
    expect(app.vault.adapter).toBeDefined();
    expect(app.vault.adapter.getName()).toBe('SynapseDataAdapter');

    const exists = await app.vault.adapter.exists('notas/bienvenida.md');
    expect(exists).toBe(true);

    const stat = await app.vault.adapter.stat('notas/bienvenida.md');
    expect(stat?.type).toBe('file');
    expect(stat?.size).toBeGreaterThan(0);

    const list = await app.vault.adapter.list('');
    expect(list.folders).toContain('notas');

    await app.vault.adapter.mkdir('nueva-carpeta');
    expect(await app.vault.adapter.exists('nueva-carpeta')).toBe(true);
  });

  it('5. Creación de carpetas createFolder retorna TFolder', async () => {
    const folder = await app.vault.createFolder('excalidraw/scripts');
    expect(folder).toBeInstanceOf(TFolder);
    expect(folder.path).toBe('excalidraw/scripts');
    expect(folder.isRoot()).toBe(false);
  });

  it('6. getAllLoadedFiles retorna árbol completo con raíz, carpetas y archivos', async () => {
    const all = await app.vault.getAllLoadedFiles();
    expect(all.length).toBeGreaterThan(3);
    expect(all.some((f) => f instanceof TFolder && f.isRoot())).toBe(true);
    expect(all.some((f) => f instanceof TFile && f.path === 'notas/bienvenida.md')).toBe(true);
  });

  it('7. app.workspace.getActiveFile retorna TFile activo', async () => {
    const active = await app.workspace.getActiveFile();
    expect(active).toBeInstanceOf(TFile);
    expect(active?.path).toBe('notas/bienvenida.md');
  });

  it('8. app.metadataCache.getFileCache parsea frontmatter, encabezados, links, embeds y tags', async () => {
    const cache = await app.metadataCache.getFileCache('notas/bienvenida.md');
    expect(cache).not.toBeNull();
    expect(cache?.frontmatter?.autor).toBe('Miuler');
    expect(cache?.frontmatter?.version).toBe(1);
    expect(cache?.headings.length).toBe(1);
    expect(cache?.headings[0].heading).toBe('Título Principal');
    expect(cache?.links.some((l) => l.link === 'otra-nota')).toBe(true);
    expect(cache?.embeds.some((e) => e.link === 'dibujo.excalidraw')).toBe(true);
    expect(cache?.tags).toContain('#tag1');
  });

  it('9. app.fileManager.processFrontMatter actualiza frontmatter sin corromper el cuerpo', async () => {
    await app.fileManager.processFrontMatter('notas/bienvenida.md', (fm) => {
      fm.categoria = 'documentacion';
      fm.editado = true;
    });

    const updatedContent = await app.vault.read('notas/bienvenida.md');
    expect(updatedContent).toContain('categoria: documentacion');
    expect(updatedContent).toContain('editado: true');
    expect(updatedContent).toContain('# Título Principal');
  });

  it('10. app.fileManager.getAvailablePathForAttachment genera rutas sin colisión', async () => {
    const available = await app.fileManager.getAvailablePathForAttachment('adjunto', 'png');
    expect(available).toBe('adjunto.png');
  });
});
