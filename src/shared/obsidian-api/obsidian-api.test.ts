import { describe, expect, it } from 'bun:test';
import lzString from 'lz-string';
import { createSynapseApp } from './app';
import {
  type Command,
  Component,
  Events,
  type EventRef,
  ItemView,
  Menu,
  MenuItem,
  Modal,
  moment,
  normalizePath,
  Notice,
  parseYaml,
  Platform,
  Plugin,
  type PluginManifest,
  PluginSettingTab,
  Setting,
  stringifyYaml,
  TAbstractFile,
  TextFileView,
  TFile,
  TFolder,
  View,
  WorkspaceLeaf,
  ExcalidrawView,
  VIEW_TYPE_EXCALIDRAW,
  ExcalidrawAutomate,
  isExcalidrawFile,
  isExcalidrawMarkdown,
  parseExcalidrawContent,
  serializeExcalidrawMarkdown,
  createCanonicalExcalidrawContent,
  extractWikilinksFromElements,
  extractFrontmatter,
  resolvePluginViewType,
  debounce,
  getObsidianBundle,
} from './index';
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
  fileChangeCallbacks: Array<(payload: { paths: string[]; deleted: string[] }) => void> = [];

  isConnected() {
    return true;
  }

  onFilesChanged(cb: (payload: { paths: string[]; deleted: string[] }) => void) {
    this.fileChangeCallbacks.push(cb);
    return () => {
      this.fileChangeCallbacks = this.fileChangeCallbacks.filter((c) => c !== cb);
    };
  }

  async emitFilesChanged(payload: { paths: string[]; deleted: string[] }) {
    for (const cb of this.fileChangeCallbacks) {
      await cb(payload);
    }
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

  describe('Etapa 3: Sistema de Eventos y Caché Reactiva', () => {
    it('11. Bus canónico Events soporta on, trigger, off, offref y tryTrigger', () => {
      const bus = new Events();
      let count = 0;
      let lastVal = '';

      const ref: EventRef = bus.on('test-event', (val: string) => {
        count++;
        lastVal = val;
      });

      expect(ref).toBeDefined();
      expect(ref.name).toBe('test-event');

      bus.trigger('test-event', 'hola');
      expect(count).toBe(1);
      expect(lastVal).toBe('hola');

      // tryTrigger
      bus.tryTrigger(ref, ['segundo']);
      expect(count).toBe(2);
      expect(lastVal).toBe('segundo');

      // offref
      bus.offref(ref);
      bus.trigger('test-event', 'tercero');
      expect(count).toBe(2); // no debió incrementar
    });

    it('12. app.vault dispara eventos create, modify, delete y rename ante mutaciones', async () => {
      const createdFiles: string[] = [];
      const modifiedFiles: string[] = [];
      const deletedFiles: string[] = [];
      const renamedFiles: Array<{ file: string; oldPath: string }> = [];

      const refCreate = app.vault.on('create', (f) => createdFiles.push(f.path));
      const refMod = app.vault.on('modify', (f) => modifiedFiles.push(f.path));
      const refDel = app.vault.on('delete', (f) => deletedFiles.push(f.path));
      const refRen = app.vault.on('rename', (f, old) => renamedFiles.push({ file: f.path, oldPath: old }));

      // 1. Create
      const newFile = await app.vault.create('notas/evento-test.md', '# Nota de Evento');
      expect(newFile).toBeInstanceOf(TFile);
      expect(createdFiles).toContain('notas/evento-test.md');

      // 2. Modify
      await app.vault.modify(newFile, '# Contenido Modificado');
      expect(modifiedFiles).toContain('notas/evento-test.md');

      // 3. Rename
      await app.vault.rename(newFile, 'notas/evento-renombrado.md');
      expect(renamedFiles.some((r) => r.file === 'notas/evento-renombrado.md' && r.oldPath === 'notas/evento-test.md')).toBe(true);

      // 4. Delete
      await app.vault.delete('notas/evento-renombrado.md');
      expect(deletedFiles).toContain('notas/evento-renombrado.md');

      // Desuscribir
      app.vault.offref(refCreate);
      app.vault.offref(refMod);
      app.vault.offref(refDel);
      app.vault.offref(refRen);

      // Mutación posterior no debe registrarse
      await app.vault.create('notas/otro.md', 'contenido');
      expect(createdFiles.filter((p) => p === 'notas/otro.md').length).toBe(0);
    });

    it('13. app.metadataCache dispara changed y resolve al modificarse o crearse notas markdown', async () => {
      let changedData = '';
      let changedFile = '';
      let changedCacheKeys: string[] = [];
      let resolvedFile = '';

      const refChanged = app.metadataCache.on('changed', (file, data, cache) => {
        changedFile = file.path;
        changedData = data;
        changedCacheKeys = Object.keys(cache);
      });

      const refResolve = app.metadataCache.on('resolve', (file) => {
        resolvedFile = file.path;
      });

      await app.vault.create('notas/cache-reactive.md', '---\nstatus: activo\n---\n# Titulo\n[[enlace]]');

      expect(changedFile).toBe('notas/cache-reactive.md');
      expect(changedData).toContain('status: activo');
      expect(changedCacheKeys).toContain('frontmatter');
      expect(resolvedFile).toBe('notas/cache-reactive.md');

      // Comprobación de consulta síncrona a getFileCache
      const syncCache = app.metadataCache.getFileCache('notas/cache-reactive.md');
      expect(syncCache).not.toBeNull();
      expect(syncCache?.frontmatter?.status).toBe('activo');

      app.metadataCache.offref(refChanged);
      app.metadataCache.offref(refResolve);
    });

    it('14. app.workspace dispara file-open, active-leaf-change y layout-change', async () => {
      let openedFile: string | null = null;
      let leafObserved: any = null;
      let layoutChanged = false;

      const refOpen = app.workspace.on('file-open', (f) => {
        openedFile = f ? f.path : null;
      });
      const refLeaf = app.workspace.on('active-leaf-change', (leaf) => {
        leafObserved = leaf;
      });
      const refLayout = app.workspace.on('layout-change', () => {
        layoutChanged = true;
      });

      // Simular apertura de link
      const opened = await app.workspace.openLinkText('bienvenida', '');
      expect(opened).toBeInstanceOf(TFile);
      expect(openedFile).toBe('notas/bienvenida.md');

      // Simular active-leaf-change
      app.workspace.triggerActiveLeafChange({ view: { getViewType: () => 'markdown' } });
      expect(leafObserved?.view?.getViewType()).toBe('markdown');

      // Simular layout-change
      app.workspace.triggerLayoutChange();
      expect(layoutChanged).toBe(true);

      app.workspace.offref(refOpen);
      app.workspace.offref(refLeaf);
      app.workspace.offref(refLayout);
    });

    it('15. Sincronización de eventos externos emitidos por el watcher de archivos', async () => {
      let externalModified = '';
      let externalDeleted = '';

      const refMod = app.vault.on('modify', (f) => {
        externalModified = f.path;
      });
      const refDel = app.vault.on('delete', (f) => {
        externalDeleted = f.path;
      });

      // Simular la llegada de un archivo nuevo o modificado externamente (ej. sync o editor externo)
      (repo as MockVaultRepository).notes['externo/documento.md'] = {
        title: 'Documento',
        relative_path: 'externo/documento.md',
        content: '# Modificado fuera de Synapse',
      };

      await (repo as MockVaultRepository).emitFilesChanged({
        paths: ['externo/documento.md'],
        deleted: ['notas/antigua.md'],
      });

      expect(externalModified).toBe('externo/documento.md');
      expect(externalDeleted).toBe('notas/antigua.md');

      app.vault.offref(refMod);
      app.vault.offref(refDel);
    });
  });

  describe('Etapa 4: Vistas y Hojas de Trabajo (WorkspaceLeaf, View, TextFileView)', () => {
    class MockExcalidrawView extends TextFileView {
      rawJson = '';

      getViewType(): string {
        return 'excalidraw';
      }

      override getDisplayText(): string {
        return this.file ? `Dibujo: ${this.file.basename}` : 'Excalidraw';
      }

      getViewData(): string {
        return this.rawJson;
      }

      setViewData(data: string, _clear: boolean): void {
        this.rawJson = data;
        this.contentEl.textContent = data;
      }

      clear(): void {
        this.rawJson = '';
        this.contentEl.textContent = '';
      }
    }

    it('16. Component gestiona ciclo de vida y desuscripciones automáticas (registerEvent, registerDomEvent)', () => {
      class TestComponent extends Component {
        loadedCalls = 0;
        unloadedCalls = 0;

        override onload() {
          this.loadedCalls++;
        }

        override onunload() {
          this.unloadedCalls++;
        }
      }

      const parent = new TestComponent();
      const child = new TestComponent();
      parent.addChild(child);

      expect(parent.loaded).toBe(false);
      expect(child.loaded).toBe(false);

      parent.load();
      expect(parent.loaded).toBe(true);
      expect(child.loaded).toBe(true);
      expect(parent.loadedCalls).toBe(1);
      expect(child.loadedCalls).toBe(1);

      // Registrar evento en Component
      let eventFired = 0;
      const bus = new Events();
      const ref = bus.on('ping', () => eventFired++);
      parent.registerEvent(ref);

      bus.trigger('ping');
      expect(eventFired).toBe(1);

      // Descargar componente debe desuscribir eventos y descargar hijos
      parent.unload();
      expect(parent.loaded).toBe(false);
      expect(child.loaded).toBe(false);
      expect(parent.unloadedCalls).toBe(1);
      expect(child.unloadedCalls).toBe(1);

      bus.trigger('ping');
      expect(eventFired).toBe(1); // No debió incrementar tras unload
    });

    it('17. ItemView provee contenedor contentEl, actionsEl y soporte addAction', () => {
      const leaf = app.workspace.getLeaf(true);
      const view = new MockExcalidrawView(leaf);

      expect(view.containerEl).toBeDefined();
      expect(view.contentEl).toBeDefined();
      expect(view.actionsEl).toBeDefined();

      let actionClicked = false;
      const btn = view.addAction('pencil', 'Editar trazo', () => {
        actionClicked = true;
      });

      expect(btn).toBeDefined();
      expect(btn.getAttribute('aria-label')).toBe('Editar trazo');
    });

    it('18. app.workspace registra vistas y extensiones de archivo personalizadas', () => {
      app.workspace.registerView('excalidraw', (leaf) => new MockExcalidrawView(leaf));
      app.workspace.registerExtensions(['excalidraw', 'excalidraw.md'], 'excalidraw');

      expect(app.workspace.getViewTypeForExtension('excalidraw')).toBe('excalidraw');
      expect(app.workspace.getViewTypeForExtension('excalidraw.md')).toBe('excalidraw');
      expect(app.workspace.getViewTypeForExtension('md')).toBe('markdown');
      expect(app.workspace.getViewCreator('excalidraw')).toBeDefined();
    });

    it('19. WorkspaceLeaf abre archivo de tipo registrado, carga vista y propaga estado', async () => {
      const leaf = app.workspace.getLeaf(true);
      const file = await app.vault.getAbstractFileByPath('dibujos/canvas.excalidraw');
      expect(file).toBeInstanceOf(TFile);

      await leaf.openFile(file as TFile);

      expect(leaf.view).toBeInstanceOf(MockExcalidrawView);
      expect(leaf.view?.getViewType()).toBe('excalidraw');
      expect((leaf.view as MockExcalidrawView).getViewData()).toBe('{"elements": [100]}');
      expect(leaf.getDisplayText()).toBe('Dibujo: canvas');
      expect(app.workspace.activeLeaf).toBe(leaf);
    });

    it('20. TextFileView.save persiste datos modificados directamente en la bóveda', async () => {
      const leaf = app.workspace.getLeavesOfType('excalidraw')[0];
      expect(leaf).toBeDefined();
      const view = leaf.view as MockExcalidrawView;

      // Modificar datos en la vista visual
      view.setViewData('{"elements": [{"type": "rectangle", "x": 10}]}', false);

      // Guardar a través de la API estándar de Obsidian
      await view.save();

      // Verificar persistencia en app.vault
      const savedContent = await app.vault.read('dibujos/canvas.excalidraw');
      expect(savedContent).toBe('{"elements": [{"type": "rectangle", "x": 10}]}');
    });

    it('21. app.workspace.getLeavesOfType y getActiveViewOfType localizan vistas activas', () => {
      const excalidrawLeaves = app.workspace.getLeavesOfType('excalidraw');
      expect(excalidrawLeaves.length).toBeGreaterThanOrEqual(1);

      const activeView = app.workspace.getActiveViewOfType(MockExcalidrawView);
      expect(activeView).not.toBeNull();
      expect(activeView?.getViewType()).toBe('excalidraw');

      // Desacoplar hoja de trabajo
      const leaf = excalidrawLeaves[0];
      leaf.detach();
      expect(app.workspace.getLeavesOfType('excalidraw').length).toBe(0);
    });
  });

  describe('Etapa 5: Ciclo de Vida del Plugin (Plugin, Setting, Command, Ribbon)', () => {
    it('22. Setting provee API fluida con Text, Toggle, Dropdown, Slider y Button', () => {
      const container = app.workspace.getLeaf(true).containerEl;

      let textVal = '';
      let toggleVal = false;
      let dropVal = '';
      let sliderVal = 0;
      let btnClicked = false;

      const setting = new Setting(container)
        .setName('Ajuste de Dibujo')
        .setDesc('Configura las opciones por defecto de Excalidraw')
        .setHeading()
        .addText((text) =>
          text
            .setPlaceholder('Fuente')
            .setValue('Virgil')
            .onChange((v) => (textVal = v))
        )
        .addToggle((toggle) =>
          toggle
            .setValue(true)
            .onChange((v) => (toggleVal = v))
        )
        .addDropdown((drop) =>
          drop
            .addOption('light', 'Claro')
            .addOption('dark', 'Oscuro')
            .setValue('dark')
            .onChange((v) => (dropVal = v))
        )
        .addSlider((slider) =>
          slider
            .setLimits(1, 10, 1)
            .setValue(3)
            .onChange((v) => (sliderVal = v))
        )
        .addButton((btn) =>
          btn
            .setButtonText('Restablecer')
            .setTooltip('Restablece valores por defecto')
            .onClick(() => (btnClicked = true))
        );

      expect(setting.nameEl.textContent).toBe('Ajuste de Dibujo');
      expect(setting.descEl.textContent).toBe('Configura las opciones por defecto de Excalidraw');
      expect(setting.settingEl.classList.contains('setting-item-heading')).toBe(true);

      // Simular interacciones de controles
      const textComp = (setting as any).controlEl.children[0] as HTMLInputElement;
      textComp.value = 'Cascadia';
      textComp.dispatchEvent(new Event('input'));
      expect(textVal).toBe('Cascadia');

      const toggleComp = (setting as any).controlEl.children[1] as HTMLElement;
      toggleComp.dispatchEvent(new Event('click'));
      expect(toggleVal).toBe(false);

      const dropComp = (setting as any).controlEl.children[2] as HTMLSelectElement;
      dropComp.value = 'light';
      dropComp.dispatchEvent(new Event('change'));
      expect(dropVal).toBe('light');

      const sliderComp = (setting as any).controlEl.children[3] as HTMLInputElement;
      sliderComp.value = '7';
      sliderComp.dispatchEvent(new Event('input'));
      expect(sliderVal).toBe(7);

      const btnComp = (setting as any).controlEl.children[4] as HTMLButtonElement;
      btnComp.dispatchEvent(new Event('click'));
      expect(btnClicked).toBe(true);
    });

    it('23. PluginSettingTab y SynapseSettingManager gestionan pestañas de configuración', () => {
      class MockExcalidrawSettingTab extends PluginSettingTab {
        rendered = false;

        display(): void {
          this.rendered = true;
          new Setting(this.containerEl).setName('Modo Cuadrícula').addToggle((t) => t.setValue(true));
        }

        override hide(): void {
          super.hide();
          this.rendered = false;
        }
      }

      const mockPlugin = { manifest: { id: 'excalidraw-test', name: 'Excalidraw Test', version: '1.0' } };
      const tab = new MockExcalidrawSettingTab(app, mockPlugin);

      app.setting.addSettingTab(tab);
      expect(app.setting.settingTabs).toContain(tab);
      expect(app.setting.getSettingTab('excalidraw-test')).toBe(tab);

      // Abrir pestaña
      app.setting.openTab(tab);
      expect(tab.rendered).toBe(true);
      expect(tab.containerEl.children.length).toBeGreaterThan(0);

      // Ocultar pestaña
      tab.hide();
      expect(tab.rendered).toBe(false);
      expect(tab.containerEl.children.length).toBe(0);

      app.setting.removeSettingTab(tab);
      expect(app.setting.settingTabs).not.toContain(tab);
    });

    it('24. SynapseCommandManager registra, busca y ejecuta comandos con callback y checkCallback', () => {
      let executed = false;
      let checkExecuted = false;

      const cmd1: Command = {
        id: 'excalidraw:new-drawing',
        name: 'Crear nuevo dibujo Excalidraw',
        callback: () => {
          executed = true;
        },
      };

      const cmd2: Command = {
        id: 'excalidraw:export-png',
        name: 'Exportar dibujo a PNG',
        checkCallback: (checking) => {
          if (checking) return true;
          checkExecuted = true;
        },
      };

      app.commands.addCommand(cmd1);
      app.commands.addCommand(cmd2);

      expect(app.commands.findCommand('excalidraw:new-drawing')).toBe(cmd1);
      expect(app.commands.listCommands()).toContain(cmd1);

      // Ejecutar comando directo
      expect(app.commands.executeCommandById('excalidraw:new-drawing')).toBe(true);
      expect(executed).toBe(true);

      // Ejecutar comando con checkCallback
      expect(app.commands.executeCommandById('excalidraw:export-png')).toBe(true);
      expect(checkExecuted).toBe(true);

      // Ejecutar comando inexistente
      expect(app.commands.executeCommandById('inexistente')).toBe(false);

      app.commands.removeCommand('excalidraw:new-drawing');
      app.commands.removeCommand('excalidraw:export-png');
    });

    it('25. Plugin gestiona ciclo de vida completo: saveData, loadData, comandos, ribbon, statusBar y descarga limpia', async () => {
      const manifest: PluginManifest = {
        id: 'obsidian-excalidraw-plugin',
        name: 'Excalidraw',
        version: '2.0.0',
        author: 'Zsolt Viczian',
      };

      interface ExcalidrawSettings {
        theme: string;
        autoSave: boolean;
      }

      class ExcalidrawPluginTest extends Plugin {
        settings: ExcalidrawSettings = { theme: 'light', autoSave: true };
        commandRan = false;
        ribbonClicked = false;

        override async onload() {
          const loaded = await this.loadData();
          if (loaded) {
            this.settings = Object.assign({}, this.settings, loaded);
          }

          this.addCommand({
            id: 'toggle-fullscreen',
            name: 'Alternar pantalla completa',
            callback: () => {
              this.commandRan = true;
            },
          });

          this.addRibbonIcon('brush', 'Nuevo Dibujo', () => {
            this.ribbonClicked = true;
          });

          this.addStatusBarItem();
        }
      }

      const plugin = new ExcalidrawPluginTest(app, manifest);

      // 1. Guardar datos iniciales
      await plugin.saveData({ theme: 'dark', autoSave: false });

      // 2. Cargar plugin (load)
      await plugin.load();

      // Verificar que cargó datos correctamente
      expect(plugin.settings.theme).toBe('dark');
      expect(plugin.settings.autoSave).toBe(false);

      // Verificar registro de comando
      const fullCmdId = 'obsidian-excalidraw-plugin:toggle-fullscreen';
      expect(app.commands.findCommand(fullCmdId)).toBeDefined();
      app.commands.executeCommandById(fullCmdId);
      expect(plugin.commandRan).toBe(true);

      // Verificar registro de Ribbon Icon
      const ribbonItem = app.ribbon.icons.find((i) => i.title === 'Nuevo Dibujo');
      expect(ribbonItem).toBeDefined();
      ribbonItem?.element.dispatchEvent(new Event('click'));
      expect(plugin.ribbonClicked).toBe(true);

      // Verificar registro de StatusBar
      expect(app.statusBar.items.length).toBeGreaterThan(0);

      // 3. Descarga de plugin (unload)
      plugin.unload();

      // Verificar que los comandos registrados se limpiaron automáticamente
      expect(app.commands.findCommand(fullCmdId)).toBeUndefined();

      // Verificar que el icono de Ribbon se removió
      expect(app.ribbon.icons.find((i) => i.title === 'Nuevo Dibujo')).toBeUndefined();

      // Verificar que el elemento de StatusBar se removió
      expect(app.statusBar.items.length).toBe(0);
    });
  });

  describe('Etapa 6: Entorno Global, Bridge obsidian, Modales y PluginLoader', () => {
    it('26. Notice crea notificaciones flotantes con soporte setMessage y hide', () => {
      const notice = new Notice('Aviso inicial de prueba', 5000);
      expect(notice.noticeEl).toBeDefined();
      expect(notice.noticeEl.textContent).toBe('Aviso inicial de prueba');

      notice.setMessage('Aviso actualizado');
      expect(notice.noticeEl.textContent).toBe('Aviso actualizado');

      notice.hide();
    });

    it('27. Modal gestiona apertura, cierre y contenido accesible', () => {
      class TestExcalidrawExportModal extends Modal {
        opened = false;
        closed = false;

        override onOpen(): void {
          this.opened = true;
          this.setTitle('Exportar Lienzo Excalidraw');
          this.setContent('Opciones de exportación');
        }

        override onClose(): void {
          this.closed = true;
        }
      }

      const modal = new TestExcalidrawExportModal(app);
      expect(modal.isOpen).toBe(false);

      modal.open();
      expect(modal.isOpen).toBe(true);
      expect(modal.opened).toBe(true);
      expect(modal.titleEl.textContent).toBe('Exportar Lienzo Excalidraw');
      expect(modal.contentEl.textContent).toBe('Opciones de exportación');

      modal.close();
      expect(modal.isOpen).toBe(false);
      expect(modal.closed).toBe(true);
    });

    it('28. Menu y MenuItem gestionan menús contextuales y acciones de usuario', () => {
      const menu = new Menu();
      let exportSvgClicked = false;

      menu
        .addItem((item) => {
          item
            .setTitle('Exportar SVG')
            .setIcon('image')
            .onClick(() => {
              exportSvgClicked = true;
            });
        })
        .addSeparator()
        .addItem((item) => {
          item.setTitle('Deshacer').setDisabled(true);
        });

      expect(menu.items.length).toBe(2);
      expect(menu.items[0].title).toBe('Exportar SVG');
      expect(menu.items[1].disabled).toBe(true);

      // Simular click en ítem de menú
      menu.items[0].dom.dispatchEvent(new Event('click'));
      expect(exportSvgClicked).toBe(true);

      menu.showAtPosition({ x: 200, y: 300 });
      expect(menu.dom.style.left).toBe('200px');
      expect(menu.dom.style.top).toBe('300px');

      menu.hide();
    });

    it('29. Utilidades globales: normalizePath, parseYaml, stringifyYaml, moment y Platform', () => {
      // normalizePath
      expect(normalizePath('carpeta//subcarpeta\\nota.md')).toBe('carpeta/subcarpeta/nota.md');
      expect(normalizePath('/raiz/sub/../otra/')).toBe('raiz/otra');

      // parseYaml & stringifyYaml
      const yamlStr = '---\ntitulo: "Mi Canvas"\nversion: 2\nelementos:\n  - linea\n  - rectangulo\n---';
      const parsed = parseYaml(yamlStr);
      expect(parsed.titulo).toBe('Mi Canvas');
      expect(parsed.version).toBe(2);
      expect(Array.isArray(parsed.elementos)).toBe(true);
      expect(parsed.elementos[0]).toBe('linea');

      const serialized = stringifyYaml({ app: 'synapse', version: 1 });
      expect(serialized).toContain('app: synapse');
      expect(serialized).toContain('version: 1');

      // moment
      const m = moment('2026-10-22T15:30:45Z');
      expect(m.isValid()).toBe(true);
      expect(m.format('YYYY-MM-DD')).toBe('2026-10-22');

      // Platform
      expect(Platform.isDesktop).toBe(true);
    });

    it('30. Bridge virtual de obsidian expone el ecosistema y configura window.app y window.require', () => {
      const w = (typeof window !== 'undefined' ? window : globalThis) as any;
      expect(w.app).toBe(app);
      expect(w.moment).toBeDefined();

      expect(typeof w.require).toBe('function');
      const obsidianMod = w.require('obsidian');

      expect(obsidianMod).toBeDefined();
      expect(obsidianMod.Plugin).toBeDefined();
      expect(obsidianMod.TFile).toBeDefined();
      expect(obsidianMod.Notice).toBeDefined();
      expect(obsidianMod.Modal).toBeDefined();
      expect(obsidianMod.Setting).toBeDefined();
      expect(obsidianMod.WorkspaceLeaf).toBeDefined();
      expect(obsidianMod.normalizePath).toBeDefined();
      expect(obsidianMod.moment).toBeDefined();
    });

    it('31. PluginLoader escanea, evalúa código CommonJS de plugin, inyecta estilos y activa ciclo de vida', async () => {
      // 1. Preparar un plugin simulado en la bóveda
      const pluginDir = '.synapse/plugins/mi-plugin-canvas';
      await app.vault.adapter.mkdir(pluginDir);

      const manifest: PluginManifest = {
        id: 'mi-plugin-canvas',
        name: 'Mi Plugin Canvas',
        version: '1.0.0',
        author: 'Synapse Developer',
      };

      await app.vault.adapter.write(`${pluginDir}/manifest.json`, JSON.stringify(manifest));
      await app.vault.adapter.write(`${pluginDir}/styles.css`, '.canvas-custom-theme { background: #111; }');

      // Código CommonJS empaquetado como hacen los plugins compilados de Obsidian
      const mainJsCode = `
        const { Plugin } = require('obsidian');

        module.exports = class CanvasPlugin extends Plugin {
          async onload() {
            this.app.canvasPluginLoaded = true;
            this.addCommand({
              id: 'crear-canvas',
              name: 'Crear Lienzo',
              callback: () => {
                this.app.canvasCommandRan = true;
              }
            });
          }

          onunload() {
            this.app.canvasPluginLoaded = false;
          }
        };
      `;

      await app.vault.adapter.write(`${pluginDir}/main.js`, mainJsCode);

      // 2. Escaneo de plugins
      const scanned = await app.plugins.scanPlugins();
      expect(scanned.some((p) => p.manifest.id === 'mi-plugin-canvas')).toBe(true);

      // 3. Cargar plugin
      const entry = await app.plugins.loadPluginFromDir(pluginDir);
      expect(entry.manifest.id === 'mi-plugin-canvas').toBe(true);
      expect((app as any).canvasPluginLoaded).toBe(true);

      // Verificar que el comando registrado por el plugin empaquetado funciona
      const cmdId = 'mi-plugin-canvas:crear-canvas';
      expect(app.commands.findCommand(cmdId)).toBeDefined();
      app.commands.executeCommandById(cmdId);
      expect((app as any).canvasCommandRan).toBe(true);

      // 4. Descargar plugin
      await app.plugins.unloadPlugin('mi-plugin-canvas');
      expect((app as any).canvasPluginLoaded).toBe(false);
      expect(app.commands.findCommand(cmdId)).toBeUndefined();
      expect(app.plugins.getPlugin('mi-plugin-canvas')).toBeUndefined();
    });
  });

  describe('Etapa 7: Integración y Validación Exhaustiva de obsidian-excalidraw-plugin', () => {
    it('32. Detección y parseo bidireccional de Markdown Excalidraw (excalidraw-plugin: parsed)', () => {
      const rawMarkdown = `---

excalidraw-plugin: parsed
tags: [ea/drawing]
author: synapsers

---
==Deconstruct of drawing==
# Text Elements
Nodo de Inicio ^txt1

%%
# Drawing
\`\`\`json
{
  "type": "excalidraw",
  "version": 2,
  "elements": [
    { "id": "txt1", "type": "text", "text": "Nodo de Inicio", "isDeleted": false },
    { "id": "rect1", "type": "rectangle", "x": 50, "y": 50, "width": 120, "height": 60 }
  ],
  "appState": { "theme": "dark" }
}
\`\`\`
%%
`;

      // 1. Detección de formato
      expect(isExcalidrawMarkdown(rawMarkdown)).toBe(true);

      const mockTFile = new TFile(app.vault, 'diagramas/flujo.excalidraw.md');
      expect(isExcalidrawFile(mockTFile)).toBe(true);

      // 2. Parseo de documento
      const doc = parseExcalidrawContent(rawMarkdown);
      expect(doc.isMarkdown).toBe(true);
      expect(doc.frontmatter?.['excalidraw-plugin']).toBe('parsed');
      expect(doc.frontmatter?.author).toBe('synapsers');
      expect(doc.scene.elements.length).toBe(2);
      expect(doc.scene.appState.theme).toBe('dark');
      expect(doc.textElements.length).toBe(1);
      expect(doc.textElements[0].text).toBe('Nodo de Inicio');

      // 3. Serialización de vuelta preservando metadatos y sincronizando # Text Elements
      const reserialized = serializeExcalidrawMarkdown(
        {
          elements: [
            ...doc.scene.elements,
            { id: 'txt2', type: 'text', text: 'Nodo de Fin', isDeleted: false },
          ],
          appState: doc.scene.appState,
        },
        rawMarkdown
      );

      expect(reserialized).toContain('excalidraw-plugin: parsed');
      expect(reserialized).toContain('author: synapsers');
      expect(reserialized).toContain('Nodo de Inicio ^txt1');
      expect(reserialized).toContain('Nodo de Fin ^txt2');
      expect(reserialized).toContain('# Drawing');
    });

    it('32b. Detección, parseo y serialización de Markdown Excalidraw comprimido (compressed-json con lz-string)', () => {
      const sceneData = JSON.stringify({
        type: 'excalidraw',
        version: 2,
        elements: [
          { id: 'box1', type: 'rectangle', x: 10, y: 20, width: 100, height: 50 },
          { id: 'txt1', type: 'text', text: 'Servicio Autenticación', isDeleted: false },
        ],
        appState: { viewBackgroundColor: '#ffffff' },
      });
      const compressedB64 = lzString.compressToBase64(sceneData);

      const rawCompressedMarkdown = `---

excalidraw-plugin: parsed
tags: [excalidraw]

---
==Deconstruct of drawing==
# Text Elements
Servicio Autenticación ^txt1

%%
## Drawing
\`\`\`compressed-json
${compressedB64}
\`\`\`
%%
`;

      expect(isExcalidrawMarkdown(rawCompressedMarkdown)).toBe(true);

      const doc = parseExcalidrawContent(rawCompressedMarkdown);
      expect(doc.isMarkdown).toBe(true);
      expect(doc.scene.elements.length).toBe(2);
      expect(doc.scene.elements[0].type).toBe('rectangle');
      expect(doc.scene.elements[1].text).toBe('Servicio Autenticación');

      const recompressed = serializeExcalidrawMarkdown(doc.scene, rawCompressedMarkdown);
      expect(recompressed).toContain('```compressed-json');
      expect(recompressed).toContain('Servicio Autenticación ^txt1');

      // Re-parsear el resultado re-serializado para asegurar consistencia completa
      const reparsed = parseExcalidrawContent(recompressed);
      expect(reparsed.scene.elements.length).toBe(2);
    });

    it('32c. Detección especializada por YAML Frontmatter (--- ---) para Excalidraw, Kanban y Plugins', () => {
      // 1. extractFrontmatter
      const noteWithFm = `---
excalidraw-plugin: parsed
tags:
  - excalidraw
  - diagramas
author: Miuler
---
# Contenido del dibujo
`;
      const fm = extractFrontmatter(noteWithFm);
      expect(fm).toBeDefined();
      expect(fm?.['excalidraw-plugin']).toBe('parsed');
      expect(fm?.author).toBe('Miuler');
      expect(Array.isArray(fm?.tags)).toBe(true);

      const noteWithoutFm = '# Título simple\nSin frontmatter aquí.';
      expect(extractFrontmatter(noteWithoutFm)).toBeUndefined();

      // 2. resolvePluginViewType
      expect(resolvePluginViewType(noteWithFm)).toBe('excalidraw');

      const kanbanNote = `---
kanban-plugin: basic
---
# Tablero Kanban
`;
      expect(resolvePluginViewType(kanbanNote)).toBe('kanban');

      const slideNote = `---
slide: true
theme: solarized
---
# Diapositiva 1
`;
      expect(resolvePluginViewType(slideNote)).toBe('slides');

      expect(resolvePluginViewType(noteWithoutFm)).toBeNull();

      // 3. isExcalidrawMarkdown reconoce frontmatter sin necesidad de extensión .excalidraw
      expect(isExcalidrawMarkdown(noteWithFm)).toBe(true);
      expect(isExcalidrawMarkdown(kanbanNote)).toBe(false);
      expect(isExcalidrawMarkdown(noteWithoutFm)).toBe(false);
    });

    it('33. Extracción de Wikilinks embebidos en textos de nodos y resolución en la bóveda', async () => {
      const elements = [
        {
          id: 't1',
          type: 'text',
          text: 'Consulte [[Arquitectura Cebolla]] para más info',
          isDeleted: false,
        },
        {
          id: 't2',
          type: 'text',
          text: 'Vea también [[Guía de Rust#Testing|Manual de Pruebas]] y [[Bienvenida]]',
          isDeleted: false,
        },
        {
          id: 't3',
          type: 'text',
          text: 'Texto borrado [[Ignorar]]',
          isDeleted: true,
        },
        {
          id: 'r1',
          type: 'rectangle',
          link: '[[Enlace de Caja]]',
        },
      ];

      const links = extractWikilinksFromElements(elements);
      expect(links).toContain('Arquitectura Cebolla');
      expect(links).toContain('Guía de Rust');
      expect(links).toContain('Bienvenida');
      expect(links).toContain('Enlace de Caja');
      expect(links).not.toContain('Ignorar');

      // Comprobar resolución mediante app.metadataCache
      const dest = await app.metadataCache.getFirstLinkpathDest('Bienvenida', 'diagrama.excalidraw.md');
      expect(dest).toBeDefined();
      expect(dest?.path).toBe('notas/bienvenida.md');
    });

    it('34. Automatización y scripting fluido mediante ExcalidrawAutomate (ea)', async () => {
      const ea = new ExcalidrawAutomate(app);
      ea.reset();

      const rectId = ea.addRect(50, 50, 160, 80, {
        backgroundColor: '#e3f2fd',
        strokeColor: '#1976d2',
      });
      const textId = ea.addText(70, 75, 'Proceso: [[Bienvenida]]', {
        fontSize: 16,
      });
      const diamondId = ea.addDiamond(280, 50, 120, 80, {
        backgroundColor: '#fff3e0',
      });
      const arrowId = ea.connectElements(rectId, diamondId);

      expect(rectId).toBeDefined();
      expect(textId).toBeDefined();
      expect(diamondId).toBeDefined();
      expect(arrowId).toBeDefined();

      const elements = ea.getElements();
      expect(elements.length).toBe(4);

      // Enlaces automáticos extraídos
      const links = ea.getLinks();
      expect(links).toEqual(['Bienvenida']);

      // Crear archivo de dibujo en la bóveda
      const createdFile = await ea.createDrawing('diagramas/flujo-automatizado.excalidraw.md');
      expect(createdFile).toBeDefined();
      expect(createdFile.path).toBe('diagramas/flujo-automatizado.excalidraw.md');

      const content = await app.vault.read(createdFile);
      expect(content).toContain('excalidraw-plugin: parsed');
      expect(content).toContain('Proceso: [[Bienvenida]]');
    });

    it('35. Exportación vectorial SVG y binaria PNG transparente', async () => {
      const ea = new ExcalidrawAutomate(app);
      ea.reset();
      ea.addRect(10, 10, 100, 50, { backgroundColor: '#c8e6c9', strokeColor: '#388e3c' });
      ea.addText(20, 25, 'Diagrama SVG & PNG');

      // 1. Exportación SVG
      const svg = ea.toSVG();
      expect(svg).toContain('<svg');
      expect(svg).toContain('<rect');
      expect(svg).toContain('<text');
      expect(svg).toContain('Diagrama SVG &amp; PNG');
      expect(svg).toContain('</svg>');

      // 2. Exportación binaria
      const pngBuffer = await ea.toPNG();
      expect(pngBuffer).toBeInstanceOf(ArrayBuffer);
      expect(pngBuffer.byteLength).toBeGreaterThan(0);
    });

    it('36. ExcalidrawView gestiona ciclo de vida y conmutación entre modo visual y texto', async () => {
      const leaf = app.workspace.getLeaf(true);
      const drawingFile = await app.vault.create(
        'diagramas/arquitectura.excalidraw.md',
        `---

excalidraw-plugin: parsed
tags: [ea/drawing]

---
==Deconstruct of drawing==
# Text Elements
Capa Dominio ^c1

%%
# Drawing
\`\`\`json
{
  "type": "excalidraw",
  "version": 2,
  "elements": [
    { "id": "c1", "type": "text", "text": "Capa Dominio", "isDeleted": false }
  ],
  "appState": { "viewBackgroundColor": "#ffffff" }
}
\`\`\`
%%
`
      );

      // Registrar la vista canónica de Excalidraw
      app.workspace.registerView(VIEW_TYPE_EXCALIDRAW, (leaf) => new ExcalidrawView(leaf));

      // Abrir en hoja de trabajo
      await leaf.openFile(drawingFile);
      expect(leaf.view).toBeInstanceOf(ExcalidrawView);

      const view = leaf.view as ExcalidrawView;
      expect(view.getViewType()).toBe(VIEW_TYPE_EXCALIDRAW);
      expect(view.getDisplayText()).toBe('arquitectura.excalidraw');
      expect(view.viewMode).toBe('visual');
      expect(view.getWikilinks()).toEqual([]);

      // Conmutar a modo texto
      const mode1 = view.toggleViewMode();
      expect(mode1).toBe('text');
      expect(view.viewMode).toBe('text');
      expect(view.getViewData()).toContain('excalidraw-plugin: parsed');

      // Modificar en modo texto y volver a visual
      const newText = view.getViewData().replace('Capa Dominio', 'Capa Dominio & [[Entidades]]');
      view.setViewData(newText, false);

      const mode2 = view.toggleViewMode();
      expect(mode2).toBe('visual');
      expect(view.getWikilinks()).toEqual(['Entidades']);

      // Guardar y verificar persistencia en la bóveda
      await view.save();
      const updatedOnDisk = await app.vault.read(drawingFile);
      expect(updatedOnDisk).toContain('Capa Dominio & [[Entidades]]');
    });

    it('37. Enrutamiento inteligente de WorkspaceLeaf por extensión compuesta (.excalidraw.md) y frontmatter', async () => {
      // 1. Archivo .excalidraw puro
      const pureExcalFile = await app.vault.create(
        'diagramas/red.excalidraw',
        JSON.stringify({ type: 'excalidraw', version: 2, elements: [] })
      );
      expect(app.workspace.getViewTypeForFile(pureExcalFile)).toBe(VIEW_TYPE_EXCALIDRAW);

      // 2. Archivo compuesto .excalidraw.md
      const compoundFile = await app.vault.create(
        'diagramas/esquema.excalidraw.md',
        '# Esquema Excalidraw'
      );
      expect(app.workspace.getViewTypeForFile(compoundFile)).toBe(VIEW_TYPE_EXCALIDRAW);

      // 3. Archivo Markdown estándar que contiene frontmatter excalidraw-plugin
      const mdWithFrontmatter = await app.vault.create(
        'notas/dibujo-incrustado.md',
        `---
excalidraw-plugin: parsed
---
# Contenido`
      );
      expect(app.workspace.getViewTypeForFile(mdWithFrontmatter)).toBe(VIEW_TYPE_EXCALIDRAW);

      // 4. Archivo Markdown común y corriente
      const normalMd = await app.vault.create('notas/ordinaria.md', '# Nota normal');
      expect(app.workspace.getViewTypeForFile(normalMd)).toBe('markdown');
    });

    it('38. Integración final: carga de obsidian-excalidraw-plugin real a través de PluginLoader', async () => {
      const pluginDir = '.synapse/plugins/obsidian-excalidraw-plugin';

      // 1. Manifest del plugin comunitario oficial
      const manifest: PluginManifest = {
        id: 'obsidian-excalidraw-plugin',
        name: 'Excalidraw',
        version: '2.8.0',
        minAppVersion: '1.0.0',
        description: 'Edición y dibujo nativo de Excalidraw en Obsidian y Synapse.',
        author: 'Zsolt Viczian',
        isDesktopOnly: false,
      };
      await app.vault.adapter.write(`${pluginDir}/manifest.json`, JSON.stringify(manifest, null, 2));

      // 2. Estilos CSS
      await app.vault.adapter.write(
        `${pluginDir}/styles.css`,
        `.excalidraw-view { background: #fafafa; }`
      );

      // 3. Código empaquetado del plugin utilizando el bridge 'obsidian'
      const mainJsCode = `
        const { Plugin, ExcalidrawView, VIEW_TYPE_EXCALIDRAW } = require('obsidian');

        module.exports = class ExcalidrawPlugin extends Plugin {
          async onload() {
            this.app.excalidrawPluginActive = true;

            // Registrar vista y extensiones
            this.registerView(VIEW_TYPE_EXCALIDRAW, (leaf) => new ExcalidrawView(leaf));
            this.registerExtensions(['excalidraw', 'excalidraw.md'], VIEW_TYPE_EXCALIDRAW);

            // Registrar comando oficial de Excalidraw
            this.addCommand({
              id: 'create-drawing',
              name: 'Crear nuevo dibujo de Excalidraw',
              callback: async () => {
                this.app.excalidrawNewDrawingCreated = true;
                return await this.app.ea.createDrawing('Dibujo Sin Título.excalidraw.md');
              }
            });

            // Registrar botón en Ribbon
            this.addRibbonIcon('brush', 'Nuevo dibujo Excalidraw', () => {
              this.app.excalidrawRibbonClicked = true;
            });

            // Cargar datos previos
            const data = await this.loadData();
            this.settings = Object.assign({ defaultTheme: 'light' }, data);
          }

          onunload() {
            this.app.excalidrawPluginActive = false;
          }
        };
      `;
      await app.vault.adapter.write(`${pluginDir}/main.js`, mainJsCode);

      // 4. Cargar plugin mediante PluginLoader
      const entry = await app.plugins.loadPluginFromDir(pluginDir);
      expect(entry).toBeDefined();
      expect(entry.manifest.id).toBe('obsidian-excalidraw-plugin');
      expect((app as any).excalidrawPluginActive).toBe(true);

      // 5. Verificar comando registrado
      const cmd = app.commands.findCommand('obsidian-excalidraw-plugin:create-drawing');
      expect(cmd).toBeDefined();
      await cmd?.callback?.();
      expect((app as any).excalidrawNewDrawingCreated).toBe(true);

      const createdFile = await app.vault.getAbstractFileByPath('Dibujo Sin Título.excalidraw.md');
      expect(createdFile).toBeInstanceOf(TFile);

      // 6. Verificar persistencia de configuraciones
      const pluginInstance = entry.instance;
      pluginInstance.settings.defaultTheme = 'dark';
      await pluginInstance.saveData(pluginInstance.settings);

      const loadedData = await pluginInstance.loadData();
      expect(loadedData.defaultTheme).toBe('dark');

      // 7. Descargar limpiamente el plugin
      await app.plugins.unloadPlugin('obsidian-excalidraw-plugin');
      expect((app as any).excalidrawPluginActive).toBe(false);
      expect(app.commands.findCommand('obsidian-excalidraw-plugin:create-drawing')).toBeUndefined();
      expect(app.plugins.getPlugin('obsidian-excalidraw-plugin')).toBeUndefined();
    });

    it('39. createCanonicalExcalidrawContent genera formato canónico válido para Obsidian Excalidraw', () => {
      const canonical = createCanonicalExcalidrawContent();
      expect(canonical).toContain('excalidraw-plugin: parsed');
      expect(canonical).toContain('tags:\n  - ea/drawing');
      expect(canonical).toContain('# Text Elements');
      expect(canonical).toContain('# Drawing');
      expect(canonical).toContain('"type": "excalidraw"');
      expect(canonical).toContain('"source": "https://github.com/zsviczian/obsidian-excalidraw-plugin"');

      // Comprobar que preserva plantilla si es válida
      const customTemplate = `---
excalidraw-plugin: parsed
tags:
  - ea/drawing
  - template
---
# Text Elements
Elemento de plantilla ^txt0

%%
# Drawing
\`\`\`json
{
  "type": "excalidraw",
  "version": 2,
  "source": "https://github.com/zsviczian/obsidian-excalidraw-plugin",
  "elements": [{"id": "txt0", "type": "text", "text": "Elemento de plantilla"}],
  "appState": {},
  "files": {}
}
\`\`\`
%%`;
      const fromTemplate = createCanonicalExcalidrawContent(customTemplate);
      expect(fromTemplate).toBe(customTemplate);
    });

    it('40. ExcalidrawAutomate funciona de forma desacoplada sin necesidad de SynapseApp', () => {
      const standaloneEa = new ExcalidrawAutomate();
      const rectId = standaloneEa.addRect(10, 10, 100, 50);
      expect(rectId).toBeDefined();

      const svg = standaloneEa.toSVG();
      expect(svg).toContain('<svg');
      expect(svg).toContain('<rect');
      expect(svg).toContain('width="100"');
      expect(svg).toContain('height="50"');
    });

    it('41. Parsea y preserva la sección ## Embedded Files en documentos Excalidraw Markdown', () => {
      const excalidrawMd = `---
excalidraw-plugin: parsed
tags: [ea/drawing]
---
==Deconstruct of drawing==
# Text Elements
Elemento con texto ^txt1

## Embedded Files
img123: [[adjuntos/grafico.png]]
img456: "[[diagrama.svg]]"

%%
# Drawing
\`\`\`json
{
  "type": "excalidraw",
  "version": 2,
  "elements": [
    { "id": "txt1", "type": "text", "text": "Elemento con texto" },
    { "id": "elImg", "type": "image", "fileId": "img123", "x": 10, "y": 20, "width": 100, "height": 80 }
  ],
  "appState": {},
  "files": {}
}
\`\`\`
%%
`;

      const parsed = parseExcalidrawContent(excalidrawMd);
      expect(parsed.isMarkdown).toBe(true);
      expect(parsed.embeddedFiles).toBeDefined();
      expect(parsed.embeddedFiles!['img123']).toBe('adjuntos/grafico.png');
      expect(parsed.embeddedFiles!['img456']).toBe('diagrama.svg');
      expect(parsed.wikilinks).toContain('adjuntos/grafico.png');
      expect(parsed.wikilinks).toContain('diagrama.svg');

      // Comprobar renderizado de SVG con archivos incrustados
      const ea = new ExcalidrawAutomate();
      const filesMap = {
        img123: { id: 'img123', dataURL: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==' }
      };
      const svg = ea.toSVG(parsed.scene.elements, filesMap);
      expect(svg).toContain('<image');
      expect(svg).toContain('href="data:image/png;base64');
      expect(svg).toContain('x="10"');
      expect(svg).toContain('y="20"');

      // Comprobar serialización y preservación de ## Embedded Files
      const serialized = serializeExcalidrawMarkdown(parsed.scene, excalidrawMd);
      expect(serialized).toContain('## Embedded Files');
      expect(serialized).toContain('img123: [[adjuntos/grafico.png]]');
    });
  });

  describe('Etapa 8: Compatibilidad extendida de Obsidian API (debounce, getRightLeaf, getLeftLeaf, registerEditorExtension)', () => {
    it('42. debounce controla el flujo de eventos, cancela y ejecuta con finish', async () => {
      let callCount = 0;
      let lastVal = '';

      const fn = (v: string) => {
        callCount++;
        lastVal = v;
      };

      const debounced = debounce(fn, 50, true);

      debounced('a');
      debounced('b');
      debounced('c');

      // No se ejecuta inmediatamente
      expect(callCount).toBe(0);

      // Esperar que transcurra el tiempo
      await new Promise((r) => setTimeout(r, 70));
      expect(callCount).toBe(1);
      expect(lastVal).toBe('c');

      // Prueba con cancel()
      debounced('d');
      debounced.cancel();
      await new Promise((r) => setTimeout(r, 70));
      expect(callCount).toBe(1);

      // Prueba con finish()
      debounced('e');
      debounced.finish();
      expect(callCount).toBe(2);
      expect(lastVal).toBe('e');
    });

    it('43. app.workspace.getRightLeaf y getLeftLeaf gestionan hojas laterales', () => {
      const repo = new MockVaultRepository() as unknown as VaultRepository;
      const app = createSynapseApp(repo);

      const right1 = app.workspace.getRightLeaf(false);
      expect(right1).toBeInstanceOf(WorkspaceLeaf);

      // Si split es false, debe retornar la misma hoja existente
      const right2 = app.workspace.getRightLeaf(false);
      expect(right2).toBe(right1);

      // Si split es true, crea una nueva hoja
      const right3 = app.workspace.getRightLeaf(true);
      expect(right3).toBeInstanceOf(WorkspaceLeaf);
      expect(right3).not.toBe(right1);

      // Similar para getLeftLeaf
      const left1 = app.workspace.getLeftLeaf(false);
      expect(left1).toBeInstanceOf(WorkspaceLeaf);
      const left2 = app.workspace.getLeftLeaf(false);
      expect(left2).toBe(left1);
      const left3 = app.workspace.getLeftLeaf(true);
      expect(left3).not.toBe(left1);

      // Al desmontar una hoja, se limpia del conjunto
      right1.detach();
      const rightNew = app.workspace.getRightLeaf(false);
      expect(rightNew).toBe(right3);
    });

    it('44. Plugin.registerEditorExtension registra extensiones de CodeMirror 6 y las limpia en onunload', () => {
      const repo = new MockVaultRepository() as unknown as VaultRepository;
      const app = createSynapseApp(repo);

      class TestPlugin extends Plugin {
        myExtension = { dummy: true };
        async onload() {
          this.registerEditorExtension(this.myExtension);
        }
      }

      let eventTriggered = false;
      app.workspace.on('editor-extensions-change', () => {
        eventTriggered = true;
      });

      const manifest: PluginManifest = {
        id: 'test-cm-plugin',
        name: 'Test CM Plugin',
        version: '1.0.0',
        minAppVersion: '0.15.0',
        description: 'Test',
      };

      const plugin = new TestPlugin(app, manifest);
      plugin.load();
      expect(plugin.loaded).toBe(true);
      expect(eventTriggered).toBe(true);

      const exts = app.workspace.getEditorExtensions();
      expect(exts).toContain(plugin.myExtension);

      // Al descargar el plugin, la extensión se remueve automáticamente
      plugin.unload();
      expect(plugin.loaded).toBe(false);
      const extsAfter = app.workspace.getEditorExtensions();
      expect(extsAfter).not.toContain(plugin.myExtension);
    });

    it('45. getObsidianBundle expone debounce', () => {
      const repo = new MockVaultRepository() as unknown as VaultRepository;
      const app = createSynapseApp(repo);
      const bundle = getObsidianBundle(app);

      expect(typeof bundle.debounce).toBe('function');
      const d = bundle.debounce(() => {}, 10);
      expect(typeof d.cancel).toBe('function');
      expect(typeof d.finish).toBe('function');
    });

    it('46. app.internalPlugins soporta daily-notes y previene errores con Calendar y obsidian-daily-notes-interface', async () => {
      const repo = new MockVaultRepository() as unknown as VaultRepository;
      const app = createSynapseApp(repo);

      expect(app.internalPlugins).toBeDefined();
      expect(typeof app.internalPlugins.getPluginById).toBe('function');

      const dailyNotesEntry = app.internalPlugins.getPluginById('daily-notes');
      expect(dailyNotesEntry).toBeDefined();
      expect(dailyNotesEntry?.enabled).toBe(true);
      expect(dailyNotesEntry?.instance).toBeDefined();
      expect(dailyNotesEntry?.instance.options).toBeDefined();
      expect(dailyNotesEntry?.instance.options.format).toBe('YYYY-MM-DD');
      expect(dailyNotesEntry?.instance.options.folder).toBe('');
      expect(dailyNotesEntry?.instance.options.template).toBe('');

      // Simular comportamiento de obsidian-daily-notes-interface getDailyNoteSettings
      const retrieved = app.internalPlugins.getPluginById('daily-notes')?.instance?.options;
      expect(retrieved?.format).toBe('YYYY-MM-DD');

      // Probar lectura de .obsidian/daily-notes.json si existe
      await app.vault.adapter.write(
        '.obsidian/daily-notes.json',
        JSON.stringify({ format: 'YYYY/MM/DD', folder: 'Diario', template: 'Plantillas/Dia' })
      );
      // Re-instanciar SynapseApp para probar hidratación de configuración
      const app2 = createSynapseApp(repo);
      // Esperar brevemente a que cargue la promesa de configuración asíncrona
      await new Promise((r) => setTimeout(r, 10));
      const entry2 = app2.internalPlugins.getPluginById('daily-notes');
      expect(entry2?.instance.options.format).toBe('YYYY/MM/DD');
      expect(entry2?.instance.options.folder).toBe('Diario');
      expect(entry2?.instance.options.template).toBe('Plantillas/Dia');
    });

    it('47. moment soporta API completa requerida por Calendar (locale, localeData, isSame, weekday, week, weekdaysShort)', () => {
      const m = moment('2026-10-28', 'YYYY-MM-DD', true);
      expect(m.isValid()).toBe(true);
      expect(m.format('YYYY-MM-DD')).toBe('2026-10-28');

      // isSame con granularidades día, mes, año
      const mSame = moment('2026-10-28');
      expect(m.isSame(mSame, 'day')).toBe(true);
      expect(m.isSame(mSame, 'month')).toBe(true);
      expect(m.isSame(mSame, 'year')).toBe(true);

      const mDifferentDay = moment('2026-10-29');
      expect(m.isSame(mDifferentDay, 'day')).toBe(false);
      expect(m.isSame(mDifferentDay, 'month')).toBe(true);

      // localeData y week
      const locData = m.localeData();
      expect(locData).toBeDefined();
      expect(locData._week).toBeDefined();
      expect(typeof locData._week.dow).toBe('number');
      expect(typeof locData.firstDayOfWeek).toBe('function');

      // Static weekdaysShort
      const shortDays = moment.weekdaysShort(true);
      expect(Array.isArray(shortDays)).toBe(true);
      expect(shortDays.length).toBe(7);

      // clone, add, subtract
      const nextMonth = m.clone().add(1, 'month');
      expect(nextMonth.isSame(m, 'month')).toBe(false);
      expect(nextMonth.format('YYYY-MM')).toBe('2026-11');

      // global _bundledLocaleWeekSpec
      expect((globalThis as any)._bundledLocaleWeekSpec).toBeDefined();
      expect(typeof (globalThis as any)._bundledLocaleWeekSpec.dow).toBe('number');

      // app.foldManager y isMobile
      const repo = new MockVaultRepository() as unknown as VaultRepository;
      const app = createSynapseApp(repo);
      expect(app.foldManager).toBeDefined();
      expect(typeof app.foldManager.load).toBe('function');
      expect(typeof app.foldManager.save).toBe('function');
      expect(app.isMobile).toBe(false);
    });

    it('48. app, workspace, vault y bridge implementan soporte completo para plugins comunitarios avanzados', async () => {
      const repo = new MockVaultRepository() as unknown as VaultRepository;
      const app = createSynapseApp(repo);

      // 1. workspace.containerEl y updateOptions
      expect(app.workspace.containerEl).toBeDefined();
      let updateOptionsFired = false;
      app.workspace.on('update-options', () => {
        updateOptionsFired = true;
      });
      app.workspace.updateOptions();
      expect(updateOptionsFired).toBe(true);
      expect(app.workspace.getMostRecentLeaf()).toBeDefined();

      // 2. hover link sources
      app.workspace.registerHoverLinkSource('test-source', { display: 'Test' });
      app.workspace.unregisterHoverLinkSource('test-source');

      // 3. vault.getConfig
      expect(app.vault.getConfig('vimMode')).toBe(false);
      expect(app.vault.getConfig('livePreview')).toBe(true);

      // 4. app localStorage
      app.saveLocalStorage('test-key', { foo: 'bar' });
      expect(app.loadLocalStorage('test-key')).toEqual({ foo: 'bar' });

      // 5. metadataCache.getCachedFiles
      expect(Array.isArray(app.metadataCache.getCachedFiles())).toBe(true);

      // 6. internalPlugins workspaces
      const workspacesPlugin = app.internalPlugins.getPluginById('workspaces');
      expect(workspacesPlugin).toBeDefined();
      expect(workspacesPlugin?.instance).toBeDefined();

      // 7. Bridge global require
      const g = (typeof window !== 'undefined' ? window : globalThis) as any;
      const pathMod = g.require('path');
      expect(pathMod.join('a', 'b', 'c.md')).toBe('a/b/c.md');
      expect(pathMod.dirname('a/b/c.md')).toBe('a/b');
      expect(pathMod.basename('a/b/c.md', '.md')).toBe('c');

      const eventsMod = g.require('events');
      expect(eventsMod.EventEmitter).toBeDefined();

      const searchMod = g.require('@codemirror/search');
      expect(searchMod).toBeDefined();

      const autoMod = g.require('@codemirror/autocomplete');
      expect(autoMod).toBeDefined();

      // 8. Bridge obsidian bundle exports
      const obs = g.require('obsidian');
      expect(typeof obs.loadMermaid).toBe('function');
      expect(typeof obs.loadMathJax).toBe('function');
      expect(obs.SuggestModal).toBeDefined();

      // 9. Globals
      expect(g.activeWindow).toBeDefined();
      expect(g.process).toBeDefined();
      expect(g.Buffer).toBeDefined();
      const buf = g.Buffer.from('Hola Synapse');
      expect(buf.toString()).toBe('Hola Synapse');
    });

    it('49. file-recovery y adapter.read están protegidos contra recursión infinita por monkey-patching', async () => {
      const mockRepo = new MockVaultRepository();
      await mockRepo.saveNote({
        relativePath: 'test.md',
        title: 'test',
        content: 'contenido seguro',
      });
      const app = createSynapseApp(mockRepo);

      // 1. Simular monkey-patching recursivo de gpg-crypt sobre adapter.read
      let hookCallCount = 0;
      app.vault.adapter.read = async (path: string) => {
        hookCallCount++;
        // Intencionalmente reingresa a través de app.vault.adapter.read simulando un hook circular
        return await app.vault.adapter.read(path);
      };

      // Debe resolver el contenido sin lanzar RangeError: Maximum call stack size exceeded
      const readRes = await app.vault.adapter.read('test.md');
      expect(readRes).toBe('contenido seguro');
      expect(hookCallCount).toBe(1);

      // 2. Simular monkey-patching recursivo sobre file-recovery
      let recoveryHookCount = 0;
      const fileRecoveryInstance = app.internalPlugins.plugins['file-recovery'].instance;
      fileRecoveryInstance.onFileChanged = async (file: any) => {
        recoveryHookCount++;
        return await fileRecoveryInstance.onFileChanged(file);
      };

      // Disparar hook recursivo
      await fileRecoveryInstance.onFileChanged({ path: 'test.md' });
      expect(recoveryHookCount).toBe(1);
    });

    it('50. PluginLoader solo lee plugins de .synapse/plugins e ignora .obsidian/plugins', async () => {
      const mockRepo = new MockVaultRepository();
      const app = createSynapseApp(mockRepo);

      // Plugin en .obsidian/plugins
      await app.vault.adapter.mkdir('.obsidian/plugins/obsidian-only-plugin');
      await app.vault.adapter.write(
        '.obsidian/plugins/obsidian-only-plugin/manifest.json',
        JSON.stringify({ id: 'obsidian-only-plugin', name: 'Obsidian Only' })
      );

      // Plugin en .synapse/plugins
      await app.vault.adapter.mkdir('.synapse/plugins/synapse-only-plugin');
      await app.vault.adapter.write(
        '.synapse/plugins/synapse-only-plugin/manifest.json',
        JSON.stringify({ id: 'synapse-only-plugin', name: 'Synapse Only' })
      );

      const scanned = await app.plugins.scanPlugins();

      expect(scanned.some((p) => p.manifest.id === 'synapse-only-plugin')).toBe(true);
      expect(scanned.some((p) => p.manifest.id === 'obsidian-only-plugin')).toBe(false);

      // Intento de carga directa por ID falla para plugins fuera de .synapse/plugins
      await expect(app.plugins.loadPlugin('obsidian-only-plugin')).rejects.toThrow();
    });
  });
});
