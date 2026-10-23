import type { VaultRepository } from '@shared/repositories';
import type { SynapseApp } from '@shared/obsidian-api';
import type {
  CommunityPluginDto,
  CommunityPluginItem,
  InstalledPluginItem,
  PluginModalTab,
  PluginSortOrder,
} from '@entities/community-plugin';

export class CommunityPluginsState {
  catalogue = $state<CommunityPluginItem[]>([]);
  installed = $state<InstalledPluginItem[]>([]);
  isLoadingCatalogue = $state<boolean>(false);
  isLoadingInstalled = $state<boolean>(false);
  error = $state<string | null>(null);

  searchQuery = $state<string>('');
  sortOrder = $state<PluginSortOrder>('downloads');
  activeTab = $state<PluginModalTab>('community');

  installingMap = $state<Record<string, boolean>>({});
  uninstallingMap = $state<Record<string, boolean>>({});
  activeSettingPluginId = $state<string | null>(null);

  // Plugins filtrados y ordenados
  filteredCatalogue = $derived.by(() => {
    const query = this.searchQuery.trim().toLowerCase();
    let list = this.catalogue.filter((p) => {
      if (!query) return true;
      return (
        p.name.toLowerCase().includes(query) ||
        p.id.toLowerCase().includes(query) ||
        p.description.toLowerCase().includes(query) ||
        p.author.toLowerCase().includes(query)
      );
    });

    if (this.sortOrder === 'downloads') {
      list = [...list].sort((a, b) => (b.downloads || 0) - (a.downloads || 0));
    } else if (this.sortOrder === 'name') {
      list = [...list].sort((a, b) => a.name.localeCompare(b.name));
    }
    return list;
  });

  filteredInstalled = $derived.by(() => {
    const query = this.searchQuery.trim().toLowerCase();
    return this.installed.filter((p) => {
      if (!query) return true;
      return (
        p.manifest.name.toLowerCase().includes(query) ||
        p.id.toLowerCase().includes(query) ||
        (p.manifest.description && p.manifest.description.toLowerCase().includes(query)) ||
        (p.manifest.author && p.manifest.author.toLowerCase().includes(query))
      );
    });
  });

  /**
   * Carga el catálogo oficial de plugins desde Obsidian Releases si aún no está cargado.
   */
  async loadCatalogue(repository: VaultRepository, force = false): Promise<void> {
    if (this.catalogue.length > 0 && !force) return;

    this.isLoadingCatalogue = true;
    this.error = null;

    try {
      const items: CommunityPluginDto[] = await repository.fetchCommunityPlugins();
      this.catalogue = items.map((item) => ({
        ...item,
        isInstalled: this.installed.some((inst) => inst.id === item.id),
      }));
    } catch (e: any) {
      this.error = `No se pudo cargar el catálogo de plugins comunitarios: ${e?.message || e}`;
    } finally {
      this.isLoadingCatalogue = false;
    }
  }

  /**
   * Escanea los plugins instalados en la bóveda actual y actualiza su estado.
   */
  async refreshInstalled(app: SynapseApp | null, repository: VaultRepository): Promise<void> {
    this.isLoadingInstalled = true;
    try {
      const enabledList = await repository.getEnabledPlugins();
      const discovered = app ? await app.plugins.scanPlugins() : [];

      const installedList: InstalledPluginItem[] = discovered.map((item) => {
        const isLoaded = app ? app.plugins.isPluginLoaded(item.manifest.id) : false;
        const isEnabled = enabledList.includes(item.manifest.id) || isLoaded;
        const hasSettingsTab = app ? !!app.setting.getSettingTab(item.manifest.id) : false;

        return {
          id: item.manifest.id,
          manifest: item.manifest,
          dir: item.dir,
          isEnabled,
          hasSettingsTab,
        };
      });

      this.installed = installedList;

      // Actualizar marcas en catálogo
      if (this.catalogue.length > 0) {
        this.catalogue = this.catalogue.map((p) => ({
          ...p,
          isInstalled: installedList.some((inst) => inst.id === p.id),
        }));
      }
    } catch (e: any) {
      console.warn('[CommunityPlugins] Error al escanear plugins instalados:', e);
    } finally {
      this.isLoadingInstalled = false;
    }
  }

  /**
   * Instala un plugin desde el repositorio oficial y lo activa inmediatamente.
   */
  async installPlugin(
    plugin: CommunityPluginDto,
    app: SynapseApp | null,
    repository: VaultRepository
  ): Promise<void> {
    this.installingMap[plugin.id] = true;
    this.error = null;

    try {
      await repository.installCommunityPlugin(plugin.id, plugin.repo);

      // Agregar a lista de habilitados
      const enabledList = await repository.getEnabledPlugins();
      if (!enabledList.includes(plugin.id)) {
        enabledList.push(plugin.id);
        await repository.setEnabledPlugins(enabledList);
      }

      // Cargar en caliente en el motor si SynapseApp está disponible
      if (app) {
        try {
          await app.plugins.loadPlugin(plugin.id);
        } catch (loadErr: any) {
          console.warn(`[CommunityPlugins] Plugin instalado pero error al cargar en caliente:`, loadErr);
        }
      }

      await this.refreshInstalled(app, repository);
    } catch (e: any) {
      this.error = `Error al instalar ${plugin.name}: ${e?.message || e}`;
    } finally {
      this.installingMap[plugin.id] = false;
    }
  }

  /**
   * Desinstala un plugin, lo descarga de memoria y lo elimina del disco.
   */
  async uninstallPlugin(
    pluginId: string,
    app: SynapseApp | null,
    repository: VaultRepository
  ): Promise<void> {
    this.uninstallingMap[pluginId] = true;
    this.error = null;

    try {
      // 1. Descargar de memoria si está activo
      if (app && app.plugins.isPluginLoaded(pluginId)) {
        await app.plugins.unloadPlugin(pluginId);
      }

      // 2. Eliminar de lista de habilitados
      const enabledList = await repository.getEnabledPlugins();
      const updated = enabledList.filter((id) => id !== pluginId);
      await repository.setEnabledPlugins(updated);

      // 3. Eliminar archivos de disco
      await repository.uninstallCommunityPlugin(pluginId);

      if (this.activeSettingPluginId === pluginId) {
        this.activeSettingPluginId = null;
      }

      await this.refreshInstalled(app, repository);
    } catch (e: any) {
      this.error = `Error al desinstalar plugin: ${e?.message || e}`;
    } finally {
      this.uninstallingMap[pluginId] = false;
    }
  }

  /**
   * Habilita o deshabilita un plugin sin eliminar sus archivos.
   */
  async togglePlugin(
    pluginId: string,
    enable: boolean,
    app: SynapseApp | null,
    repository: VaultRepository
  ): Promise<void> {
    try {
      const enabledList = await repository.getEnabledPlugins();
      let updated: string[];

      if (enable) {
        updated = Array.from(new Set([...enabledList, pluginId]));
        if (app && !app.plugins.isPluginLoaded(pluginId)) {
          await app.plugins.loadPlugin(pluginId);
        }
      } else {
        updated = enabledList.filter((id) => id !== pluginId);
        if (app && app.plugins.isPluginLoaded(pluginId)) {
          await app.plugins.unloadPlugin(pluginId);
        }
      }

      await repository.setEnabledPlugins(updated);
      await this.refreshInstalled(app, repository);
    } catch (e: any) {
      this.error = `Error al alternar estado de plugin: ${e?.message || e}`;
    }
  }
}

export const communityPluginsState = new CommunityPluginsState();
