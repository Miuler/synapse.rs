<script lang="ts">
  import { Dialog } from 'bits-ui';
  import {
    Search,
    X,
    Download,
    ExternalLink,
    RefreshCw,
    Loader2,
    Settings,
    Trash2,
    Check,
    AlertCircle,
    ArrowLeft,
    Boxes,
  } from 'lucide-svelte';
  import type { VaultRepository } from '@shared/repositories';
  import type { SynapseApp } from '@shared/obsidian-api';
  import { communityPluginsState } from '../model/community-plugins.svelte';

  interface Props {
    open?: boolean;
    app: SynapseApp | null;
    repository: VaultRepository;
    onClose?: () => void;
  }

  let {
    open = $bindable(false),
    app,
    repository,
    onClose,
  }: Props = $props();

  let settingContainerEl = $state<HTMLDivElement | null>(null);

  // Inicializar y sincronizar datos al abrir el modal
  $effect(() => {
    if (open) {
      communityPluginsState.refreshInstalled(app, repository);
      communityPluginsState.loadCatalogue(repository);
    } else {
      communityPluginsState.activeSettingPluginId = null;
    }
  });

  // Renderizar la pestaña de configuración del plugin activo si existe
  $effect(() => {
    const pluginId = communityPluginsState.activeSettingPluginId;
    if (pluginId && app && settingContainerEl) {
      const tab = app.setting.getSettingTab(pluginId);
      if (tab) {
        settingContainerEl.innerHTML = '';
        tab.display();
        settingContainerEl.appendChild(tab.containerEl);
      }
    }
  });

  function closeModal() {
    open = false;
    if (communityPluginsState.activeSettingPluginId && app) {
      const tab = app.setting.getSettingTab(communityPluginsState.activeSettingPluginId);
      tab?.hide();
    }
    communityPluginsState.activeSettingPluginId = null;
    if (onClose) onClose();
  }

  function formatDownloads(count?: number): string {
    if (!count) return '0';
    if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
    if (count >= 1_000) return `${(count / 1_000).toFixed(1)}K`;
    return count.toString();
  }
</script>

<Dialog.Root
  {open}
  onOpenChange={(isOpen) => {
    open = isOpen;
    if (!isOpen) closeModal();
  }}
>
  <Dialog.Portal>
    <Dialog.Overlay class="plugin-modal-backdrop" />
    <Dialog.Content class="plugin-modal-content">
      <!-- Encabezado del Modal -->
      <div class="plugin-modal-header">
        <div class="header-title-row">
          <div class="header-title-group">
            <Boxes size={22} class="header-icon" />
            <Dialog.Title class="header-title">Plugins de la Comunidad</Dialog.Title>
          </div>
          <button class="icon-btn close-btn" onclick={closeModal} title="Cerrar (Esc)">
            <X size={18} />
          </button>
        </div>

        {#if !communityPluginsState.activeSettingPluginId}
          <!-- Barra de Pestañas y Acciones -->
          <div class="header-tabs-row">
            <div class="tabs-group">
              <button
                class="tab-btn"
                class:active={communityPluginsState.activeTab === 'community'}
                onclick={() => (communityPluginsState.activeTab = 'community')}
              >
                Explorar catálogo
              </button>
              <button
                class="tab-btn"
                class:active={communityPluginsState.activeTab === 'installed'}
                onclick={() => (communityPluginsState.activeTab = 'installed')}
              >
                Instalados ({communityPluginsState.installed.length})
              </button>
            </div>

            <div class="header-actions">
              <button
                class="icon-btn refresh-btn"
                title="Actualizar catálogo y plugins"
                onclick={() => {
                  communityPluginsState.refreshInstalled(app, repository);
                  communityPluginsState.loadCatalogue(repository, true);
                }}
                disabled={communityPluginsState.isLoadingCatalogue}
              >
                <RefreshCw size={15} class={communityPluginsState.isLoadingCatalogue ? 'spin' : ''} />
              </button>
            </div>
          </div>

          <!-- Barra de Búsqueda y Filtros -->
          <div class="search-filter-row">
            <div class="search-input-wrapper">
              <Search size={16} class="search-icon" />
              <input
                type="text"
                placeholder={communityPluginsState.activeTab === 'community'
                  ? 'Buscar plugins por nombre, descripción o autor...'
                  : 'Buscar en plugins instalados...'}
                bind:value={communityPluginsState.searchQuery}
                class="search-input"
              />
              {#if communityPluginsState.searchQuery}
                <button
                  class="clear-search-btn"
                  onclick={() => (communityPluginsState.searchQuery = '')}
                  title="Limpiar búsqueda"
                >
                  <X size={14} />
                </button>
              {/if}
            </div>

            {#if communityPluginsState.activeTab === 'community'}
              <div class="sort-selector">
                <span class="sort-label">Ordenar:</span>
                <select bind:value={communityPluginsState.sortOrder} class="sort-select">
                  <option value="downloads">Más descargados</option>
                  <option value="name">Nombre (A-Z)</option>
                </select>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Mensajes de Error Global -->
      {#if communityPluginsState.error}
        <div class="alert-error">
          <AlertCircle size={16} />
          <span>{communityPluginsState.error}</span>
          <button class="alert-close" onclick={() => (communityPluginsState.error = null)}>
            <X size={14} />
          </button>
        </div>
      {/if}

      <!-- Cuerpo del Modal -->
      <div class="plugin-modal-body">
        {#if communityPluginsState.activeSettingPluginId}
          <!-- Subvista de Configuración de un Plugin Específico -->
          {@const activeId = communityPluginsState.activeSettingPluginId}
          {@const pluginItem = communityPluginsState.installed.find((i) => i.id === activeId)}
          <div class="settings-subview">
            <div class="settings-subview-header">
              <button
                class="back-btn"
                onclick={() => {
                  if (app) {
                    const tab = app.setting.getSettingTab(activeId);
                    tab?.hide();
                  }
                  communityPluginsState.activeSettingPluginId = null;
                }}
              >
                <ArrowLeft size={16} />
                <span>Volver a plugins</span>
              </button>
              <div class="settings-title-info">
                <h3>{pluginItem?.manifest.name || activeId}</h3>
                <span class="plugin-version-badge">v{pluginItem?.manifest.version || '1.0.0'}</span>
              </div>
            </div>
            <div class="settings-container-host" bind:this={settingContainerEl}></div>
          </div>
        {:else if communityPluginsState.activeTab === 'community'}
          <!-- Vista de Explorar Catálogo Comunitario -->
          {#if communityPluginsState.isLoadingCatalogue && communityPluginsState.catalogue.length === 0}
            <div class="loading-state">
              <Loader2 size={32} class="spin" />
              <p>Descargando directorio oficial de plugins de Obsidian...</p>
            </div>
          {:else if communityPluginsState.filteredCatalogue.length === 0}
            <div class="empty-state">
              <Boxes size={40} />
              <p>No se encontraron plugins que coincidan con la búsqueda.</p>
            </div>
          {:else}
            <div class="plugins-grid">
              {#each communityPluginsState.filteredCatalogue as plugin (plugin.id)}
                {@const isInstalling = communityPluginsState.installingMap[plugin.id]}
                {@const isInstalled = communityPluginsState.installed.some((i) => i.id === plugin.id)}
                <div class="plugin-card" class:installed={isInstalled}>
                  <div class="card-header">
                    <div class="title-area">
                      <h4 class="plugin-title">{plugin.name}</h4>
                      <span class="plugin-author">por {plugin.author}</span>
                    </div>
                    {#if plugin.downloads !== undefined}
                      <div class="downloads-badge" title="{plugin.downloads.toLocaleString()} descargas totales">
                        <Download size={13} />
                        <span>{formatDownloads(plugin.downloads)}</span>
                      </div>
                    {/if}
                  </div>

                  <p class="plugin-desc">{plugin.description}</p>

                  <div class="card-footer">
                    <a
                      href="https://github.com/{plugin.repo}"
                      target="_blank"
                      rel="noopener noreferrer"
                      class="repo-link"
                      title="Ver repositorio en GitHub"
                    >
                      <ExternalLink size={13} />
                      <span>{plugin.repo}</span>
                    </a>

                    <div class="action-buttons">
                      {#if isInstalled}
                        <span class="badge-installed">
                          <Check size={14} />
                          Instalado
                        </span>
                      {:else}
                        <button
                          class="btn-install"
                          onclick={() => communityPluginsState.installPlugin(plugin, app, repository)}
                          disabled={isInstalling}
                        >
                          {#if isInstalling}
                            <Loader2 size={14} class="spin" />
                            <span>Instalando...</span>
                          {:else}
                            <Download size={14} />
                            <span>Instalar</span>
                          {/if}
                        </button>
                      {/if}
                    </div>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {:else}
          <!-- Vista de Plugins Instalados -->
          {#if communityPluginsState.isLoadingInstalled}
            <div class="loading-state">
              <Loader2 size={32} class="spin" />
              <p>Escaneando plugins instalados...</p>
            </div>
          {:else if communityPluginsState.filteredInstalled.length === 0}
            <div class="empty-state">
              <Boxes size={40} />
              <p>No tienes plugins instalados en tu bóveda.</p>
              <button
                class="btn-primary-action"
                onclick={() => (communityPluginsState.activeTab = 'community')}
              >
                Explorar catálogo comunitario
              </button>
            </div>
          {:else}
            <div class="installed-list">
              {#each communityPluginsState.filteredInstalled as item (item.id)}
                {@const isUninstalling = communityPluginsState.uninstallingMap[item.id]}
                <div class="installed-item" class:disabled={!item.isEnabled}>
                  <div class="item-main">
                    <div class="item-title-row">
                      <h4 class="item-title">{item.manifest.name}</h4>
                      <span class="item-version">v{item.manifest.version}</span>
                      <span class="item-source-badge" title="Ruta en la bóveda">{item.dir}</span>
                    </div>
                    <p class="item-author">por {item.manifest.author || 'Desconocido'}</p>
                    {#if item.manifest.description}
                      <p class="item-desc">{item.manifest.description}</p>
                    {/if}
                  </div>

                  <div class="item-controls">
                    <!-- Switch Activar / Desactivar -->
                    <label class="toggle-switch" title={item.isEnabled ? 'Desactivar plugin' : 'Activar plugin'}>
                      <input
                        type="checkbox"
                        checked={item.isEnabled}
                        onchange={(e) =>
                          communityPluginsState.togglePlugin(
                            item.id,
                            e.currentTarget.checked,
                            app,
                            repository
                          )}
                      />
                      <span class="slider"></span>
                    </label>

                    <!-- Botón Configuración si el plugin tiene PluginSettingTab -->
                    {#if item.hasSettingsTab}
                      <button
                        class="icon-action-btn"
                        title="Opciones de configuración del plugin"
                        onclick={() => (communityPluginsState.activeSettingPluginId = item.id)}
                      >
                        <Settings size={16} />
                      </button>
                    {/if}

                    <!-- Botón Desinstalar -->
                    <button
                      class="icon-action-btn danger"
                      title="Desinstalar plugin de la bóveda"
                      onclick={() => {
                        if (confirm(`¿Estás seguro de que deseas desinstalar '${item.manifest.name}'?`)) {
                          communityPluginsState.uninstallPlugin(item.id, app, repository);
                        }
                      }}
                      disabled={isUninstalling}
                    >
                      {#if isUninstalling}
                        <Loader2 size={16} class="spin" />
                      {:else}
                        <Trash2 size={16} />
                      {/if}
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {/if}
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.plugin-modal-backdrop) {
    position: fixed;
    inset: 0;
    background-color: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(4px);
    z-index: 1000;
  }

  :global(.plugin-modal-content) {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 90vw;
    max-width: 880px;
    height: 82vh;
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: 12px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.35);
    z-index: 1001;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--text-primary);
  }

  .plugin-modal-header {
    padding: 16px 20px 12px 20px;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .header-title-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .header-title-group {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  :global(.header-icon) {
    color: var(--accent);
  }

  :global(.header-title) {
    font-size: 1.15rem;
    font-weight: 600;
    margin: 0;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    padding: 6px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .icon-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .header-tabs-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .tabs-group {
    display: flex;
    gap: 8px;
    background: var(--bg-primary);
    padding: 3px;
    border-radius: 8px;
    border: 1px solid var(--border-primary);
  }

  .tab-btn {
    background: transparent;
    border: none;
    padding: 6px 14px;
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--text-secondary);
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .tab-btn.active {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .search-filter-row {
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .search-input-wrapper {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
  }

  :global(.search-icon) {
    position: absolute;
    left: 12px;
    color: var(--text-muted);
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: 8px 34px 8px 36px;
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 0.88rem;
    outline: none;
    transition: border-color 0.15s ease;
  }

  .search-input:focus {
    border-color: var(--accent);
  }

  .clear-search-btn {
    position: absolute;
    right: 10px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 2px;
    display: flex;
    align-items: center;
  }

  .sort-selector {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.82rem;
    color: var(--text-secondary);
  }

  .sort-select {
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    color: var(--text-primary);
    padding: 6px 10px;
    border-radius: 6px;
    font-size: 0.82rem;
    outline: none;
    cursor: pointer;
  }

  .alert-error {
    margin: 10px 20px 0 20px;
    padding: 10px 14px;
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 6px;
    color: #ef4444;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 0.85rem;
  }

  .alert-close {
    margin-left: auto;
    background: transparent;
    border: none;
    color: inherit;
    cursor: pointer;
  }

  .plugin-modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }

  .loading-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    gap: 14px;
    text-align: center;
  }

  .btn-primary-action {
    background: var(--accent);
    color: #fff;
    border: none;
    padding: 8px 16px;
    border-radius: 6px;
    font-weight: 500;
    cursor: pointer;
  }

  .plugins-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
    gap: 16px;
  }

  .plugin-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 8px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    transition: transform 0.1s ease, border-color 0.15s ease;
  }

  .plugin-card:hover {
    border-color: var(--border-secondary);
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
  }

  .title-area {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .plugin-title {
    font-size: 0.96rem;
    font-weight: 600;
    margin: 0;
    color: var(--text-primary);
  }

  .plugin-author {
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  .downloads-badge {
    display: flex;
    align-items: center;
    gap: 4px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-primary);
    padding: 3px 7px;
    border-radius: 12px;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .plugin-desc {
    font-size: 0.84rem;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.4;
    flex: 1;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .card-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 8px;
    border-top: 1px solid var(--border-primary);
    font-size: 0.8rem;
  }

  .repo-link {
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--text-muted);
    text-decoration: none;
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .repo-link:hover {
    color: var(--accent);
    text-decoration: underline;
  }

  .btn-install {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--accent);
    color: #fff;
    border: none;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 0.82rem;
    font-weight: 500;
    cursor: pointer;
    transition: opacity 0.15s ease;
  }

  .btn-install:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .badge-installed {
    display: flex;
    align-items: center;
    gap: 4px;
    color: #10b981;
    font-size: 0.82rem;
    font-weight: 500;
  }

  /* Vista de Instalados */
  .installed-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .installed-item {
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 8px;
    padding: 14px 18px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }

  .installed-item.disabled {
    opacity: 0.65;
  }

  .item-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .item-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .item-title {
    font-size: 0.95rem;
    font-weight: 600;
    margin: 0;
  }

  .item-version {
    font-size: 0.78rem;
    color: var(--text-muted);
    background: var(--bg-tertiary);
    padding: 1px 6px;
    border-radius: 4px;
  }

  .item-source-badge {
    font-size: 0.72rem;
    color: var(--text-muted);
  }

  .item-author {
    font-size: 0.78rem;
    color: var(--text-muted);
    margin: 0;
  }

  .item-desc {
    font-size: 0.83rem;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.35;
  }

  .item-controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  /* Switch */
  .toggle-switch {
    position: relative;
    display: inline-block;
    width: 40px;
    height: 22px;
    cursor: pointer;
  }

  .toggle-switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .slider {
    position: absolute;
    inset: 0;
    background-color: var(--border-primary);
    border-radius: 22px;
    transition: 0.2s;
  }

  .slider:before {
    position: absolute;
    content: "";
    height: 16px;
    width: 16px;
    left: 3px;
    bottom: 3px;
    background-color: #fff;
    border-radius: 50%;
    transition: 0.2s;
  }

  input:checked + .slider {
    background-color: var(--accent);
  }

  input:checked + .slider:before {
    transform: translateX(18px);
  }

  .icon-action-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    padding: 6px;
    border-radius: 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
    transition: all 0.15s ease;
  }

  .icon-action-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .icon-action-btn.danger:hover {
    background: rgba(239, 68, 68, 0.15);
    color: #ef4444;
  }

  /* Subvista de Configuración */
  .settings-subview {
    display: flex;
    flex-direction: column;
    gap: 16px;
    height: 100%;
  }

  .settings-subview-header {
    display: flex;
    align-items: center;
    gap: 16px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border-primary);
  }

  .back-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    color: var(--text-primary);
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .back-btn:hover {
    background: var(--bg-tertiary);
  }

  .settings-title-info {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .settings-title-info h3 {
    margin: 0;
    font-size: 1.05rem;
  }

  .plugin-version-badge {
    font-size: 0.8rem;
    background: var(--bg-secondary);
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--text-muted);
  }

  .settings-container-host {
    flex: 1;
    overflow-y: auto;
  }

  :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>
