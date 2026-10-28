import { Component } from './component';
import { Events } from './events';
import { TAbstractFile, TFile, TFolder } from './types';
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
    Menu,
    MenuItem,
    FuzzySuggestModal,
    EditorSuggest,
    AbstractInputSuggest,
    Scope,

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
