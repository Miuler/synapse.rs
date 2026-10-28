import { Component } from './component';
import { Events } from './events';
import { TAbstractFile, TFile, TFolder } from './types';
import { SynapseVault } from './vault';
import {
  View,
  ItemView,
  TextFileView,
  DefaultMarkdownView,
  FileView,
  MarkdownView,
  MarkdownRenderChild,
  MarkdownRenderer,
  WorkspaceSplit,
  editorLivePreviewField,
} from './view';
import { WorkspaceLeaf } from './leaf';
import { SynapseWorkspace } from './workspace';
import { Plugin } from './plugin';
import {
  Setting,
  PluginSettingTab,
  TextComponent,
  TextAreaComponent,
  ToggleComponent,
  DropdownComponent,
  SliderComponent,
  ButtonComponent,
  ExtraButtonComponent,
  ColorComponent,
} from './setting';
import {
  Notice,
  Modal,
  SuggestModal,
  Menu,
  MenuItem,
  FuzzySuggestModal,
  EditorSuggest,
  AbstractInputSuggest,
  Scope,
} from './ui';
import {
  normalizePath,
  parseYaml,
  stringifyYaml,
  requestUrl,
  request,
  Platform,
  moment,
  arrayBufferToBase64,
  base64ToArrayBuffer,
  requireApiVersion,
  getLanguage,
  loadPdfJs,
  getFrontMatterInfo,
  parseFrontMatterEntry,
  resolveSubpath,
  sanitizeHTMLToDom,
  prepareFuzzySearch,
  htmlToMarkdown,
  addIcon,
  getIcon,
  getIconIds,
  setIcon,
  debounce,
} from './utils';
import { createDiv, createEl, createSpan, empty } from './dom';
import { ExcalidrawView, VIEW_TYPE_EXCALIDRAW } from './excalidraw-view';
import { ExcalidrawAutomate } from './excalidraw-automate';
import {
  isExcalidrawFile,
  isExcalidrawMarkdown,
  parseExcalidrawContent,
  serializeExcalidrawMarkdown,
  extractWikilinksFromElements,
} from './excalidraw-parser';
import * as cmCommands from '@codemirror/commands';
import * as cmLanguage from '@codemirror/language';
import * as cmState from '@codemirror/state';
import * as cmView from '@codemirror/view';
import * as cmSearch from '@codemirror/search';
import * as cmAutocomplete from '@codemirror/autocomplete';
import * as lezerCommon from '@lezer/common';
import * as lezerHighlight from '@lezer/highlight';
import * as lezerLr from '@lezer/lr';
import type { SynapseApp } from './app';

/**
 * Genera el paquete completo de exportaciones del módulo virtual 'obsidian'.
 * Todo lo que un plugin compilado espera al hacer `require('obsidian')` está incluido aquí.
 */
export function getObsidianBundle(app: SynapseApp): Record<string, any> {
  return {
    app,
    // Clases de componentes y ciclo de vida
    Component,
    Events,
    Plugin,
    PluginSettingTab,
    BaseComponent: Component,
    ValueComponent: Component,

    // Entidades del sistema de archivos
    Vault: SynapseVault,
    TAbstractFile,
    TFile,
    TFolder,

    // Vistas y espacio de trabajo
    View,
    ItemView,
    TextFileView,
    DefaultMarkdownView,
    FileView,
    MarkdownView,
    MarkdownRenderChild,
    MarkdownRenderer,
    Workspace: SynapseWorkspace,
    WorkspaceSplit,
    editorLivePreviewField,
    WorkspaceLeaf,
    ExcalidrawView,
    VIEW_TYPE_EXCALIDRAW,

    // Automatización y parseo de Excalidraw
    ExcalidrawAutomate,
    isExcalidrawFile,
    isExcalidrawMarkdown,
    parseExcalidrawContent,
    serializeExcalidrawMarkdown,
    extractWikilinksFromElements,

    // Interfaz de configuración
    Setting,
    TextComponent,
    TextAreaComponent,
    ToggleComponent,
    DropdownComponent,
    SliderComponent,
    ButtonComponent,
    ExtraButtonComponent,
    ColorComponent,

    // Componentes visuales y modales
    Notice,
    Modal,
    SuggestModal,
    Menu,
    MenuItem,
    FuzzySuggestModal,
    EditorSuggest,
    AbstractInputSuggest,
    Scope,

    // Utilidades de renderizado y diagramas
    loadMermaid: async () => {
      const g = (typeof window !== 'undefined' ? window : globalThis) as any;
      if (g.mermaid) return g.mermaid;
      const imported = await import('mermaid');
      g.mermaid = imported.default || imported;
      return g.mermaid;
    },
    loadMathJax: async () => {},

    // Utilidades del sistema y DOM
    normalizePath,
    parseYaml,
    stringifyYaml,
    requestUrl,
    request,
    Platform,
    moment,
    arrayBufferToBase64,
    base64ToArrayBuffer,
    requireApiVersion,
    getLanguage,
    loadPdfJs,
    getFrontMatterInfo,
    parseFrontMatterEntry,
    resolveSubpath,
    sanitizeHTMLToDom,
    prepareFuzzySearch,
    htmlToMarkdown,
    addIcon,
    getIcon,
    getIconIds,
    setIcon,
    createEl,
    createDiv,
    createSpan,
    empty,
    debounce,
  };
}

/**
 * Inicializa el entorno global en el navegador, WebView de Tauri o runtime de ejecución.
 * Establece `window.app` (o `globalThis.app`), `moment`, `ExcalidrawAutomate` y configura el shim de `require('obsidian')`.
 */
export function setupObsidianGlobalEnvironment(app: SynapseApp): void {
  const target: any = typeof window !== 'undefined' ? window : (globalThis as any);
  if (!target) return;

  if (typeof (globalThis as any).window === 'undefined') {
    (globalThis as any).window = globalThis;
  }

  if (typeof (globalThis as any).MutationObserver === 'undefined') {
    (globalThis as any).MutationObserver = class {
      observe() {}
      disconnect() {}
      takeRecords() {
        return [];
      }
    };
  }

  if (typeof (globalThis as any).document === 'undefined') {
    (globalThis as any).document = {
      __isSynapseMock: true,
      createElement: (t: any) => createEl(t),
      createElementNS: (_ns: string, t: any) => createEl(t),
      createTextNode: (t: string) => ({ textContent: t }),
      head: createEl('head'),
      body: createEl('body'),
      addEventListener: () => {},
      removeEventListener: () => {},
      querySelector: () => null,
      querySelectorAll: () => [],
    };
  }

  target.app = app;
  target.moment = moment;
  target.activeWindow = target.window || globalThis;
  if (!target.process) {
    target.process = {
      env: {},
      platform: 'linux',
      nextTick: (fn: any, ...args: any[]) => setTimeout(() => fn(...args), 0),
    };
  }
  if (!target.global) {
    target.global = target;
  }
  if (!target.Buffer) {
    class BufferPolyfill extends Uint8Array {
      static from(data: any, encoding?: any): any {
        if (typeof data === 'string') {
          if (encoding === 'base64') {
            const bin = typeof atob !== 'undefined' ? atob(data) : '';
            const buf = new BufferPolyfill(bin.length);
            for (let i = 0; i < bin.length; i++) buf[i] = bin.charCodeAt(i);
            return buf;
          }
          const enc = new TextEncoder();
          const bytes = enc.encode(data);
          const buf = new BufferPolyfill(bytes.length);
          buf.set(bytes);
          return buf;
        }
        if (data instanceof Uint8Array || Array.isArray(data)) {
          const buf = new BufferPolyfill(data.length);
          buf.set(data);
          return buf;
        }
        return new BufferPolyfill(0);
      }
      static isBuffer(obj: any): boolean {
        return obj instanceof Uint8Array;
      }
      toString(encoding?: string): string {
        if (encoding === 'base64') {
          let bin = '';
          for (let i = 0; i < this.length; i++) bin += String.fromCharCode(this[i]);
          return typeof btoa !== 'undefined' ? btoa(bin) : '';
        }
        const dec = new TextDecoder();
        return dec.decode(this);
      }
    }
    target.Buffer = BufferPolyfill as any;
  }
  if (!target.CodeMirror) {
    target.CodeMirror = {
      defineMode: (_name: string, _fn: any) => {},
      defineSimpleMode: (_name: string, _states: any) => {},
    };
  }
  if (!target._bundledLocaleWeekSpec) {
    target._bundledLocaleWeekSpec = (moment.localeData() as any)?._week || { dow: 1, doy: 4 };
  }
  target.sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
  if (typeof (globalThis as any).sleep === 'undefined') {
    (globalThis as any).sleep = target.sleep;
  }

  const ea = new ExcalidrawAutomate(app);
  target.ExcalidrawAutomate = ExcalidrawAutomate;
  target.ea = ea;

  const bundle = getObsidianBundle(app);
  target.obsidian = bundle;
  target.debounce = debounce;

  const prevRequire = target.require;
  target.require = function (moduleId: string) {
    if (moduleId === 'obsidian') {
      return bundle;
    }
    if (moduleId === 'moment') {
      return moment;
    }
    if (moduleId === '@codemirror/state') {
      return cmState;
    }
    if (moduleId === '@codemirror/view') {
      return cmView;
    }
    if (moduleId === '@codemirror/language') {
      return cmLanguage;
    }
    if (moduleId === '@codemirror/commands') {
      return cmCommands;
    }
    if (moduleId === '@codemirror/search') {
      return cmSearch;
    }
    if (moduleId === '@codemirror/autocomplete') {
      return cmAutocomplete;
    }
    if (moduleId === 'path') {
      return {
        sep: '/',
        join: (...parts: string[]) => parts.filter(Boolean).join('/').replace(/\/+/g, '/'),
        dirname: (p: string) => {
          const idx = p.lastIndexOf('/');
          return idx > 0 ? p.slice(0, idx) : idx === 0 ? '/' : '.';
        },
        basename: (p: string, ext?: string) => {
          const base = p.split('/').pop() || '';
          if (ext && base.endsWith(ext)) return base.slice(0, -ext.length);
          return base;
        },
        extname: (p: string) => {
          const base = p.split('/').pop() || '';
          const idx = base.lastIndexOf('.');
          return idx > 0 ? base.slice(idx) : '';
        },
        resolve: (...parts: string[]) => parts.filter(Boolean).join('/').replace(/\/+/g, '/'),
        relative: (from: string, to: string) => to,
      };
    }
    if (moduleId === 'events') {
      return {
        EventEmitter: Events,
      };
    }
    if (moduleId === 'constants') {
      return {};
    }
    if (moduleId === 'url') {
      return {
        fileURLToPath: (u: string) => u.replace(/^file:\/\//, ''),
        pathToFileURL: (p: string) => new URL(`file://${p}`),
      };
    }
    if (moduleId === 'punycode') {
      return {
        toASCII: (s: string) => s,
        toUnicode: (s: string) => s,
      };
    }
    if (moduleId === 'crypto') {
      return {
        createHash: (_algo: string) => ({
          update: function () {
            return this;
          },
          digest: (enc?: string) => (enc === 'hex' ? '0000000000000000' : new Uint8Array(16)),
        }),
        randomBytes: (size: number) => {
          const buf = new Uint8Array(size);
          if (typeof crypto !== 'undefined' && crypto.getRandomValues) {
            crypto.getRandomValues(buf);
          }
          return buf;
        },
      };
    }
    if (moduleId === 'stream') {
      class StreamStub extends Events {
        pipe(dest: any) {
          return dest;
        }
      }
      return {
        Stream: StreamStub,
        Readable: StreamStub,
        Writable: StreamStub,
        Transform: StreamStub,
      };
    }
    if (moduleId === 'process') {
      return target.process;
    }
    if (moduleId === 'util') {
      return {
        promisify: (fn: any) => fn,
        inspect: (val: any) => String(val),
        format: (...args: any[]) => args.join(' '),
      };
    }
    if (moduleId === 'querystring') {
      return {
        parse: (str: string) => Object.fromEntries(new URLSearchParams(str)),
        stringify: (obj: any) => new URLSearchParams(obj).toString(),
      };
    }
    if (['child_process', 'electron', 'http', 'https', 'fs', 'os'].includes(moduleId)) {
      console.warn(`[Synapse Module Bridge] Stub seguro activado para módulo no-web: '${moduleId}'`);
      return {};
    }
    if (moduleId === '@lezer/common') {
      return lezerCommon;
    }
    if (moduleId === '@lezer/highlight') {
      return lezerHighlight;
    }
    if (moduleId === '@lezer/lr') {
      return lezerLr;
    }
    if (typeof prevRequire === 'function') {
      return prevRequire(moduleId);
    }
    throw new Error(`[Synapse Module Bridge] No se encontró el módulo solicitado: '${moduleId}'`);
  };
}
