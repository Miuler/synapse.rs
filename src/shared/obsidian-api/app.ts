import type { VaultRepository } from '@shared/repositories';
import { SynapseVault } from './vault';
import { SynapseMetadataCache } from './metadata-cache';
import { SynapseWorkspace } from './workspace';
import { SynapseFileManager } from './file-manager';
import { SynapseCommandManager } from './command';
import { SynapseSettingManager } from './setting';
import { SynapseRibbonManager, SynapseStatusBarManager } from './ribbon';
import { PluginLoader } from './loader';
import { SynapseInternalPluginsManager } from './internal-plugins';
import { ExcalidrawAutomate } from './excalidraw-automate';
import { setupObsidianGlobalEnvironment } from './bridge';

export class SynapseApp {
  readonly vault: SynapseVault;
  readonly metadataCache: SynapseMetadataCache;
  readonly workspace: SynapseWorkspace;
  readonly fileManager: SynapseFileManager;
  readonly commands: SynapseCommandManager;
  readonly setting: SynapseSettingManager;
  readonly ribbon: SynapseRibbonManager;
  readonly statusBar: SynapseStatusBarManager;
  readonly plugins: PluginLoader;
  readonly internalPlugins: SynapseInternalPluginsManager;
  readonly foldManager: {
    load(file?: any): any;
    save(file?: any, info?: any): void;
  };
  readonly isMobile: boolean = false;
  readonly ea: ExcalidrawAutomate;

  constructor(repository: VaultRepository) {
    this.vault = new SynapseVault(repository);
    this.workspace = new SynapseWorkspace(repository, this.vault);
    this.metadataCache = new SynapseMetadataCache(repository, this.vault);
    this.fileManager = new SynapseFileManager(this.vault);
    this.commands = new SynapseCommandManager();
    this.setting = new SynapseSettingManager();
    this.ribbon = new SynapseRibbonManager();
    this.statusBar = new SynapseStatusBarManager();
    this.plugins = new PluginLoader(this);
    this.internalPlugins = new SynapseInternalPluginsManager(this);
    this.foldManager = {
      load: () => null,
      save: () => {},
    };
    this.ea = new ExcalidrawAutomate(this);

    // Conectar caché reactiva a la bóveda para sincronización bidireccional automática de eventos
    this.vault.setMetadataCache(this.metadataCache);
    this.workspace.setApp(this);

    // Inicializar variables globales de Obsidian (window.app, window.moment, window.require('obsidian'))
    setupObsidianGlobalEnvironment(this);
  }

  /**
   * Atajo canónico para añadir icono a la barra Ribbon.
   */
  addRibbonIcon(icon: string, title: string, callback: (evt: MouseEvent) => any): HTMLElement {
    return this.ribbon.addRibbonIcon(icon, title, callback);
  }

  /**
   * Atajo canónico para añadir un widget en la barra de estado.
   */
  addStatusBarItem(): HTMLElement {
    return this.statusBar.addStatusBarItem();
  }
}

export function createSynapseApp(repository: VaultRepository): SynapseApp {
  return new SynapseApp(repository);
}
