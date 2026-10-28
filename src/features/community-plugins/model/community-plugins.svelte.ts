import type { VaultRepository } from '@shared/repositories';
import type { SynapseApp, PluginManifest } from '@shared/obsidian-api';
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

  // Paginación y renderizado fluido
  pageSize = 40;
  displayedCount = $state<number>(40);

  // Estado de comprobación y actualización de plugins
  isCheckingUpdates = $state<boolean>(false);
  isUpdatingAll = $state<boolean>(false);
  updatingMap = $state<Record<string, boolean>>({});

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

  // Lista paginada para evitar congelar el DOM
  displayedCatalogue = $derived.by(() => {
    return this.filteredCatalogue.slice(0, this.displayedCount);
  });

  hasMoreCatalogue = $derived.by(() => {
    return this.displayedCount < this.filteredCatalogue.length;
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

  updatesAvailableCount = $derived.by(() => {
    return this.installed.filter((p) => !!p.updateAvailable).length;
  });

  loadMoreCatalogue(): void {
    if (this.hasMoreCatalogue) {
      this.displayedCount += this.pageSize;
    }
  }

  resetPagination(): void {
    this.displayedCount = this.pageSize;
  }

  /**
   * Carga el catálogo oficial de plugins desde la caché en memoria/disco o GitHub Releases.
   */
  async loadCatalogue(repository: VaultRepository, force = false): Promise<void> {
    if (this.catalogue.length > 0 && !force) return;

    this.isLoadingCatalogue = true;
    this.error = null;

    try {
      const items: CommunityPluginDto[] = await repository.fetchCommunityPlugins(force);
      this.catalogue = items.map((item) => {
        const inst = this.installed.find((i) => i.id === item.id);
        return {
          ...item,
          isInstalled: !!inst,
          updateAvailable: inst?.updateAvailable,
        };
      });
      this.resetPagination();
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
      let discovered: Array<{ dir: string; manifest: PluginManifest }> = [];

      // 1. Escaneo backend directo a disco en .synapse/plugins
      if (typeof repository.getInstalledCommunityPlugins === 'function') {
        const backendPlugins = await repository.getInstalledCommunityPlugins();
        if (backendPlugins && backendPlugins.length > 0) {
          discovered = backendPlugins.map((b) => ({
            dir: b.dir,
            manifest: {
              id: b.id,
              name: b.name,
              version: b.version,
              description: b.description,
              author: b.author,
              repo: b.repo,
            },
          }));
        }
      }

      // 2. Fallback a scanPlugins si el backend no encontró nada
      if (discovered.length === 0 && app) {
        discovered = await app.plugins.scanPlugins();
      }

      const existingUpdatesMap = new Map<string, { latestVersion: string; repo: string }>();
      for (const inst of this.installed) {
        if (inst.updateAvailable) {
          existingUpdatesMap.set(inst.id, inst.updateAvailable);
        }
      }

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
          updateAvailable: existingUpdatesMap.get(item.manifest.id),
        };
      });

      this.installed = installedList;

      // Actualizar marcas en catálogo
      if (this.catalogue.length > 0) {
        this.catalogue = this.catalogue.map((p) => {
          const inst = installedList.find((i) => i.id === p.id);
          return {
            ...p,
            isInstalled: !!inst,
            updateAvailable: inst?.updateAvailable,
          };
        });
      }
    } catch (e: any) {
      console.warn('[CommunityPlugins] Error al escanear plugins instalados:', e);
    } finally {
      this.isLoadingInstalled = false;
    }
  }

  /**
   * Comprueba si hay actualizaciones disponibles para los plugins instalados.
   */
  async checkForUpdates(repository: VaultRepository): Promise<void> {
    if (this.installed.length === 0) return;
    this.isCheckingUpdates = true;
    this.error = null;

    try {
      const toCheck: Array<[string, string, string]> = [];
      for (const inst of this.installed) {
        let repo = (inst.manifest as any).repo || (inst.manifest as any).repository;
        if (!repo) {
          const matchInCatalogue = this.catalogue.find((c) => c.id === inst.id);
          if (matchInCatalogue?.repo) {
            repo = matchInCatalogue.repo;
          }
        }
        if (repo) {
          toCheck.push([inst.id, inst.manifest.version, repo]);
        }
      }

      if (toCheck.length > 0 && typeof repository.checkCommunityPluginUpdates === 'function') {
        const updates = await repository.checkCommunityPluginUpdates(toCheck);
        this.installed = this.installed.map((item) => {
          const update = updates.find((u) => u.id === item.id && u.has_update);
          return {
            ...item,
            updateAvailable: update
              ? { latestVersion: update.latest_version, repo: update.repo }
              : undefined,
          };
        });

        // Reflejar también en el catálogo
        if (this.catalogue.length > 0) {
          this.catalogue = this.catalogue.map((p) => {
            const inst = this.installed.find((i) => i.id === p.id);
            return {
              ...p,
              updateAvailable: inst?.updateAvailable,
            };
          });
        }
      }
    } catch (e: any) {
      this.error = `Error al buscar actualizaciones de plugins: ${e?.message || e}`;
    } finally {
      this.isCheckingUpdates = false;
    }
  }

  /**
   * Actualiza un único plugin a su versión más reciente.
   */
  async updateSinglePlugin(
    pluginId: string,
    app: SynapseApp | null,
    repository: VaultRepository
  ): Promise<void> {
    const item = this.installed.find((i) => i.id === pluginId);
    let repo = item?.updateAvailable?.repo;
    if (!repo) {
      repo = this.catalogue.find((c) => c.id === pluginId)?.repo;
    }
    if (!repo) return;

    this.updatingMap[pluginId] = true;
    this.error = null;

    try {
      await repository.installCommunityPlugin(pluginId, repo);

      if (app) {
        try {
          await app.plugins.loadPlugin(pluginId);
        } catch (loadErr: any) {
          console.warn(`[CommunityPlugins] Error al recargar plugin ${pluginId}:`, loadErr);
        }
      }

      // Limpiar indicador de actualización
      this.installed = this.installed.map((p) =>
        p.id === pluginId ? { ...p, updateAvailable: undefined } : p
      );

      await this.refreshInstalled(app, repository);
    } catch (e: any) {
      this.error = `Error al actualizar ${item?.manifest.name || pluginId}: ${e?.message || e}`;
    } finally {
      this.updatingMap[pluginId] = false;
    }
  }

  /**
   * Actualiza todos los plugins que tienen una versión más reciente disponible.
   */
  async updateAllPlugins(app: SynapseApp | null, repository: VaultRepository): Promise<void> {
    const pluginsToUpdate = this.installed.filter((p) => !!p.updateAvailable);
    if (pluginsToUpdate.length === 0) return;

    this.isUpdatingAll = true;
    this.error = null;

    try {
      for (const p of pluginsToUpdate) {
        await this.updateSinglePlugin(p.id, app, repository);
      }
    } catch (e: any) {
      this.error = `Error al actualizar plugins: ${e?.message || e}`;
    } finally {
      this.isUpdatingAll = false;
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
