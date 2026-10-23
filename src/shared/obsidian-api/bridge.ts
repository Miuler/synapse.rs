import { Component } from './component';
import { Events } from './events';
import { TAbstractFile, TFile, TFolder } from './types';
import { View, ItemView, TextFileView, DefaultMarkdownView } from './view';
import { WorkspaceLeaf } from './leaf';
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
import { Notice, Modal, Menu, MenuItem } from './ui';
import { normalizePath, parseYaml, stringifyYaml, requestUrl, Platform, moment } from './utils';
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

    // Entidades del sistema de archivos
    TAbstractFile,
    TFile,
    TFolder,

    // Vistas y espacio de trabajo
    View,
    ItemView,
    TextFileView,
    DefaultMarkdownView,
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

    // Utilidades del sistema y DOM
    normalizePath,
    parseYaml,
    stringifyYaml,
    requestUrl,
    Platform,
    moment,
    createEl,
    createDiv,
    createSpan,
    empty,
  };
}

/**
 * Inicializa el entorno global en el navegador, WebView de Tauri o runtime de ejecución.
 * Establece `window.app` (o `globalThis.app`), `moment`, `ExcalidrawAutomate` y configura el shim de `require('obsidian')`.
 */
export function setupObsidianGlobalEnvironment(app: SynapseApp): void {
  const target: any = typeof window !== 'undefined' ? window : (globalThis as any);
  if (!target) return;

  target.app = app;
  target.moment = moment;

  const ea = new ExcalidrawAutomate(app);
  target.ExcalidrawAutomate = ExcalidrawAutomate;
  target.ea = ea;

  const bundle = getObsidianBundle(app);

  const prevRequire = target.require;
  target.require = function (moduleId: string) {
    if (moduleId === 'obsidian') {
      return bundle;
    }
    if (moduleId === 'moment') {
      return moment;
    }
    if (typeof prevRequire === 'function') {
      return prevRequire(moduleId);
    }
    throw new Error(`[Synapse Module Bridge] No se encontró el módulo solicitado: '${moduleId}'`);
  };
}
