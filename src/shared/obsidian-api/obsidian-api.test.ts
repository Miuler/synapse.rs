import { describe, expect, it } from 'bun:test';
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
} from './types';
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

      // 2. Cargar plugin (onload)
      plugin.load();
      await plugin.onload();

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
});
