<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import {
    vaultRepository,
    isTauriEnvironment,
    type RecentWorkspaceEntry,
    type SelectVaultFolderResult,
  } from '@shared/repositories';
  import { appSettings, shortcutsConfig, matchesShortcut } from '@entities/settings';
  import { WindowControls } from '@shared/ui/window-controls';
  import {
    Folder,
    FolderOpen,
    FolderGit2,
    GitBranch,
    FileText,
    Clock,
    Star,
    ExternalLink,
    ArrowRight,
    Trash2,
    Search,
    PlusCircle,
    ShieldCheck,
    Database,
    Sparkles,
    HelpCircle,
    Settings,
    Sun,
    Moon,
    Minus,
    Square,
    X,
    Cpu,
    Zap,
  } from 'lucide-svelte';

  interface Props {
    onWorkspaceOpened: (folderPath: string, initialNotes?: SelectVaultFolderResult['notes']) => void;
    onStandaloneFileOpened?: (filePath: string) => void;
    onCloseWelcome?: () => void;
    canCloseWelcome?: boolean;
  }

  let {
    onWorkspaceOpened,
    onStandaloneFileOpened,
    onCloseWelcome,
    canCloseWelcome = false,
  }: Props = $props();

  type FilterTab = 'all' | 'pinned' | 'git';
  type SortOrder = 'recent' | 'name' | 'notes';

  let workspaces = $state<RecentWorkspaceEntry[]>([]);
  let isLoading = $state(true);
  let activeFilter = $state<FilterTab>('all');
  let sortOrder = $state<SortOrder>('recent');
  let searchQuery = $state('');
  let selectedIndex = $state(0);
  let searchInputRef = $state<HTMLInputElement | null>(null);

  // Estado modal Clonar Git
  let isCloneModalOpen = $state(false);
  let cloneRepoUrl = $state('');
  let isCloning = $state(false);
  let cloneError = $state<string | null>(null);

  // Estado modal Atajos de teclado
  let isShortcutsOpen = $state(false);

  let filteredWorkspaces = $derived.by(() => {
    let list = [...workspaces];

    if (activeFilter === 'pinned') {
      list = list.filter((w) => w.pinned);
    } else if (activeFilter === 'git') {
      list = list.filter((w) => w.is_git_repo);
    }

    if (searchQuery.trim().length > 0) {
      const q = searchQuery.toLowerCase().trim();
      list = list.filter(
        (w) =>
          w.name.toLowerCase().includes(q) ||
          w.display_path.toLowerCase().includes(q) ||
          w.path.toLowerCase().includes(q) ||
          (w.git_branch && w.git_branch.toLowerCase().includes(q))
      );
    }

    list.sort((a, b) => {
      if (a.pinned !== b.pinned && activeFilter === 'all') {
        return a.pinned ? -1 : 1;
      }
      if (sortOrder === 'name') {
        return a.name.localeCompare(b.name);
      }
      if (sortOrder === 'notes') {
        return b.notes_count - a.notes_count;
      }
      return b.last_opened_ms - a.last_opened_ms;
    });

    return list;
  });

  $effect(() => {
    if (filteredWorkspaces.length === 0) {
      selectedIndex = 0;
    } else if (selectedIndex >= filteredWorkspaces.length) {
      selectedIndex = filteredWorkspaces.length - 1;
    } else if (selectedIndex < 0) {
      selectedIndex = 0;
    }
  });

  function formatRelativeTime(timestampMs: number): string {
    if (!timestampMs) return 'Reciente';
    const diffSec = Math.max(0, Math.floor((Date.now() - timestampMs) / 1000));
    if (diffSec < 60) return 'Hace unos seg';
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `Hace ${diffMin} min`;
    const diffHours = Math.floor(diffMin / 60);
    if (diffHours < 24) return diffHours === 1 ? 'Hace 1 hora' : `Hace ${diffHours} horas`;
    const diffDays = Math.floor(diffHours / 24);
    if (diffDays === 1) {
      const d = new Date(timestampMs);
      const hh = String(d.getHours()).padStart(2, '0');
      const mm = String(d.getMinutes()).padStart(2, '0');
      return `Ayer, ${hh}:${mm}`;
    }
    if (diffDays < 30) return `Hace ${diffDays} días`;
    const d = new Date(timestampMs);
    return d.toLocaleDateString('es-ES', { day: '2-digit', month: 'short' });
  }

  function formatNotesCount(count: number): string {
    return count.toLocaleString('en-US') + (count === 1 ? ' nota' : ' notas');
  }

  async function loadWorkspaces() {
    isLoading = true;
    try {
      workspaces = await vaultRepository.getRecentWorkspaces();
    } catch (e) {
      console.error('Error cargando workspaces recientes:', e);
    } finally {
      isLoading = false;
    }
  }

  async function handleOpenWorkspace(entry: RecentWorkspaceEntry) {
    try {
      await vaultRepository.setActiveVaultPath(entry.path);
      appSettings.setLastOpenedFolder(entry.path);
      onWorkspaceOpened(entry.path);
    } catch (e) {
      console.error('Error abriendo workspace:', e);
      // Si falló (ej. carpeta inexistente), recargar workspaces para refrescar la lista limpia
      await loadWorkspaces();
    }
  }

  async function handleOpenInNewWindow(entry: RecentWorkspaceEntry, e: MouseEvent) {
    e.stopPropagation();
    await vaultRepository.openWorkspaceInNewWindow(entry.path);
  }

  async function handleTogglePin(entry: RecentWorkspaceEntry, e: MouseEvent) {
    e.stopPropagation();
    workspaces = await vaultRepository.togglePinWorkspace(entry.path);
  }

  async function handleRemoveWorkspace(entry: RecentWorkspaceEntry, e: MouseEvent) {
    e.stopPropagation();
    workspaces = await vaultRepository.removeRecentWorkspace(entry.path);
  }

  async function handleCreateNewWorkspace() {
    const res = await vaultRepository.createNewWorkspace(appSettings.lastOpenedFolder);
    if (res && res.folder_path) {
      appSettings.setLastOpenedFolder(res.folder_path);
      onWorkspaceOpened(res.folder_path, res.notes);
    }
  }

  async function handleSelectLocalFolder() {
    const res = await vaultRepository.selectVaultFolder(appSettings.lastOpenedFolder);
    if (res && res.folder_path) {
      appSettings.setLastOpenedFolder(res.folder_path);
      onWorkspaceOpened(res.folder_path, res.notes);
    }
  }

  async function handleOpenStandaloneFile() {
    const filePath = await vaultRepository.selectStandaloneFile();
    if (filePath && onStandaloneFileOpened) {
      onStandaloneFileOpened(filePath);
    }
  }

  async function handleCloneSubmit(e: Event) {
    e.preventDefault();
    if (!cloneRepoUrl.trim() || isCloning) return;
    isCloning = true;
    cloneError = null;
    try {
      const res = await vaultRepository.cloneGitWorkspace(
        cloneRepoUrl.trim(),
        appSettings.lastOpenedFolder
      );
      if (res && res.folder_path) {
        isCloneModalOpen = false;
        cloneRepoUrl = '';
        appSettings.setLastOpenedFolder(res.folder_path);
        onWorkspaceOpened(res.folder_path, res.notes);
      }
    } catch (err: any) {
      cloneError = typeof err === 'string' ? err : err?.message || 'Error al clonar el repositorio';
    } finally {
      isCloning = false;
    }
  }

  async function handleWindowControl(action: 'minimize' | 'maximize' | 'close') {
    if (!isTauriEnvironment()) return;
    try {
      const appWindow = getCurrentWebviewWindow();
      if (action === 'minimize') await appWindow.minimize();
      else if (action === 'maximize') await appWindow.toggleMaximize();
      else if (action === 'close') await appWindow.close();
    } catch (err) {
      console.error(`Error en control de ventana (${action}):`, err);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (isCloneModalOpen) {
      if (e.key === 'Escape') {
        isCloneModalOpen = false;
      }
      return;
    }
    if (isShortcutsOpen) {
      if (e.key === 'Escape') {
        isShortcutsOpen = false;
      }
      return;
    }

    if (matchesShortcut(e, shortcutsConfig.welcome.search_workspaces)) {
      e.preventDefault();
      searchInputRef?.focus();
      searchInputRef?.select();
      return;
    }
    if (matchesShortcut(e, shortcutsConfig.welcome.new_workspace)) {
      e.preventDefault();
      handleCreateNewWorkspace();
      return;
    }
    if (matchesShortcut(e, shortcutsConfig.welcome.open_folder)) {
      e.preventDefault();
      handleSelectLocalFolder();
      return;
    }
    if (matchesShortcut(e, shortcutsConfig.welcome.clone_git)) {
      e.preventDefault();
      isCloneModalOpen = true;
      return;
    }

    if (e.key === 'Escape') {
      if (searchQuery.trim().length > 0) {
        e.preventDefault();
        searchQuery = '';
        return;
      }
      if (canCloseWelcome && onCloseWelcome) {
        e.preventDefault();
        onCloseWelcome();
        return;
      }
    }

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (filteredWorkspaces.length > 0) {
        selectedIndex = (selectedIndex + 1) % filteredWorkspaces.length;
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (filteredWorkspaces.length > 0) {
        selectedIndex =
          (selectedIndex - 1 + filteredWorkspaces.length) % filteredWorkspaces.length;
      }
    } else if (e.key === 'Enter') {
      const activeEl = document.activeElement;
      const isButtonFocused =
        activeEl instanceof HTMLButtonElement || activeEl instanceof HTMLSelectElement;
      if (!isButtonFocused && filteredWorkspaces[selectedIndex]) {
        e.preventDefault();
        handleOpenWorkspace(filteredWorkspaces[selectedIndex]);
      }
    }
  }

  onMount(() => {
    shortcutsConfig.loadFromBackend();
    loadWorkspaces();
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  });
</script>

<div class="welcome-root">
  <!-- BARRA SUPERIOR DE VENTANA (DRAG REGION) -->
  <header class="welcome-titlebar" data-tauri-drag-region>
    <div class="titlebar-left" data-tauri-drag-region>
      <div class="app-brand-pill" data-tauri-drag-region>
        <img src="/icon.svg" alt="Synapse" class="brand-pill-logo" />
        <span class="brand-title">Synapse Desktop</span>
        <span class="brand-sep">—</span>
        <span class="brand-version">v0.4.2-beta</span>
      </div>
    </div>

    <div class="titlebar-right" data-tauri-drag-region="false">
      {#if canCloseWelcome && onCloseWelcome}
        <button
          type="button"
          class="back-to-editor-btn"
          onclick={onCloseWelcome}
          title="Volver al editor activo (Esc)"
        >
          <span>Volver al workspace activo</span>
        </button>
      {/if}

      <div class="engine-status-pill" title="Motor de indexación full-text Tantivy activo">
        <span class="status-dot"></span>
        <span>Rust Tantivy Core Online</span>
      </div>

      <button
        type="button"
        class="top-icon-btn"
        onclick={() => (isShortcutsOpen = true)}
        title="Ayuda y atajos de teclado"
        aria-label="Ayuda"
      >
        <HelpCircle size={15} />
      </button>

      <button
        type="button"
        class="top-icon-btn"
        onclick={() => appSettings.cycleTheme()}
        title="Cambiar tema (Claro / Oscuro / Sistema)"
        aria-label="Cambiar tema"
      >
        {#if appSettings.resolvedTheme === 'dark'}
          <Moon size={15} />
        {:else}
          <Sun size={15} />
        {/if}
      </button>

      <!-- Reutilización del componente unificado WindowControls -->
      <WindowControls />
    </div>
  </header>

  <!-- CONTENIDO PRINCIPAL DIVIDIDO EN DOS COLUMNAS -->
  <div class="welcome-body">
    <!-- PANEL LATERAL IZQUIERDO -->
    <aside class="welcome-sidebar">
      <div class="hero-brand">
        <div class="logo-card">
          <img src="/icon.svg" alt="Synapse" class="hero-logo-img" />
        </div>

        <h1 class="hero-title">SYNAPSE</h1>

        <div class="hero-badges">
          <span class="rust-engine-badge">RUST ENGINE</span>
          <span class="local-vault-label">Local-first vault</span>
        </div>

        <p class="hero-description">
          Gestión hiperrápida de conocimiento y grafos interconectados sin nube obligatoria.
        </p>
      </div>

      <!-- INICIO RÁPIDO -->
      <div class="quick-start-section">
        <span class="section-heading">INICIO RÁPIDO</span>

        <button
          type="button"
          class="quick-action-btn primary-action"
          onclick={handleCreateNewWorkspace}
        >
          <div class="qa-left">
            <div class="qa-icon-wrap primary-icon">
              <PlusCircle size={18} />
            </div>
            <div class="qa-texts">
              <span class="qa-title">Nuevo Workspace</span>
              <span class="qa-subtitle">Crear directorio vacío</span>
            </div>
          </div>
          <kbd class="qa-kbd">{shortcutsConfig.welcome.new_workspace}</kbd>
        </button>

        <button type="button" class="quick-action-btn" onclick={handleSelectLocalFolder}>
          <div class="qa-left">
            <div class="qa-icon-wrap amber-icon">
              <FolderOpen size={17} />
            </div>
            <span class="qa-title">Abrir carpeta local...</span>
          </div>
          <kbd class="qa-kbd">{shortcutsConfig.welcome.open_folder}</kbd>
        </button>

        <button type="button" class="quick-action-btn" onclick={handleOpenStandaloneFile}>
          <div class="qa-left">
            <div class="qa-icon-wrap blue-icon">
              <FileText size={17} />
            </div>
            <div class="qa-texts">
              <span class="qa-title">Abrir archivo puntual...</span>
              <span class="qa-subtitle">Markdown, Mermaid, Excalidraw</span>
            </div>
          </div>
          <kbd class="qa-kbd">Ctrl+O</kbd>
        </button>

        <button
          type="button"
          class="quick-action-btn"
          onclick={() => {
            cloneError = null;
            isCloneModalOpen = true;
          }}
        >
          <div class="qa-left">
            <div class="qa-icon-wrap orange-icon">
              <GitBranch size={17} />
            </div>
            <span class="qa-title">Clonar repositorio Git</span>
          </div>
          <kbd class="qa-kbd">{shortcutsConfig.welcome.clone_git}</kbd>
        </button>
      </div>

      <!-- FOOTER DE TELEMETRÍA TÉCNICA -->
      <div class="sidebar-footer">
        <div class="tech-spec-row">
          <div class="spec-label">
            <ShieldCheck size={14} class="spec-icon teal" />
            <span>Tauri IPC v2.0</span>
          </div>
          <span class="spec-value">Rust 1.81</span>
        </div>

        <div class="tech-spec-row">
          <div class="spec-label">
            <Database size={14} class="spec-icon amber" />
            <span>Indexador Tantivy</span>
          </div>
          <span class="spec-value">0ms Latencia</span>
        </div>

        <div class="sidebar-footer-links">
          <button type="button" class="footer-link-btn" onclick={() => (isShortcutsOpen = true)}>
            Atajos de teclado
          </button>
          <span class="footer-license">Licencia MIT</span>
        </div>
      </div>
    </aside>

    <!-- PANEL PRINCIPAL DERECHO: WORKSPACES RECIENTES -->
    <main class="welcome-main">
      <!-- CABECERA Y BUSCADOR -->
      <div class="main-header">
        <div class="main-header-titles">
          <div class="title-with-count">
            <h2>Workspaces Recientes</h2>
            <span class="count-pill">{workspaces.length}</span>
          </div>
          <p class="main-subtitle">
            Selecciona un espacio de trabajo local para cargar el grafo neuronal de notas.
          </p>
        </div>

        <div class="search-box">
          <Search size={15} class="search-icon" />
          <input
            type="text"
            bind:this={searchInputRef}
            bind:value={searchQuery}
            placeholder="Buscar workspace o ruta..."
            class="search-input"
          />
          <kbd class="search-kbd">{shortcutsConfig.welcome.search_workspaces}</kbd>
        </div>
      </div>

      <!-- BARRA DE FILTROS Y ORDENACIÓN -->
      <div class="filter-bar">
        <div class="filter-tabs" role="tablist">
          <button
            type="button"
            role="tab"
            aria-selected={activeFilter === 'all'}
            class="filter-tab"
            class:active={activeFilter === 'all'}
            onclick={() => (activeFilter = 'all')}
          >
            Todos
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeFilter === 'pinned'}
            class="filter-tab"
            class:active={activeFilter === 'pinned'}
            onclick={() => (activeFilter = 'pinned')}
          >
            Fijados
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeFilter === 'git'}
            class="filter-tab"
            class:active={activeFilter === 'git'}
            onclick={() => (activeFilter = 'git')}
          >
            Git Synced
          </button>
        </div>

        <div class="sort-control">
          <span class="sort-label">Ordenar:</span>
          <select bind:value={sortOrder} class="sort-select" aria-label="Ordenar workspaces">
            <option value="recent">Último acceso</option>
            <option value="name">Nombre (A-Z)</option>
            <option value="notes">Cantidad de notas</option>
          </select>
        </div>
      </div>

      <!-- LISTA DE TARJETAS DE WORKSPACE -->
      <div class="workspaces-list">
        {#if isLoading}
          <div class="empty-list-state">
            <span>Cargando espacios de trabajo...</span>
          </div>
        {:else if filteredWorkspaces.length === 0}
          <div class="empty-list-state">
            <FolderOpen size={42} strokeWidth={1.4} class="empty-folder-icon" />
            {#if workspaces.length === 0}
              <h3>Aún no has abierto ningún Workspace</h3>
              <p>
                Crea un nuevo directorio o abre una carpeta existente con notas Markdown para comenzar.
                Tu historial se guardará automáticamente en <code>~/.config/synapsers/workspaces.json</code>.
              </p>
              <button type="button" class="open-primary-cta" onclick={handleSelectLocalFolder}>
                <FolderOpen size={16} />
                <span>Abrir carpeta local</span>
              </button>
            {:else}
              <h3>No se encontraron workspaces para este filtro</h3>
              <p>Prueba cambiando el término de búsqueda o seleccionando la pestaña "Todos".</p>
            {/if}
          </div>
        {:else}
          {#each filteredWorkspaces as ws, idx (ws.path)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="workspace-card"
              class:selected={idx === selectedIndex}
              class:missing={!ws.exists}
              onmouseenter={() => (selectedIndex = idx)}
              onclick={() => handleOpenWorkspace(ws)}
            >
              <!-- ICONO IZQUIERDO -->
              <div class="ws-icon-box" class:pinned-box={ws.pinned} class:teal-box={!ws.pinned}>
                {#if ws.pinned}
                  <Folder size={22} />
                {:else}
                  <FolderGit2 size={22} />
                {/if}
              </div>

              <!-- INFORMACIÓN CENTRAL -->
              <div class="ws-info">
                <div class="ws-title-row">
                  <span class="ws-name">{ws.name}</span>
                  {#if ws.pinned}
                    <span class="pinned-badge">
                      <Star size={10} fill="currentColor" />
                      <span>Fijado</span>
                    </span>
                  {/if}
                  {#if !ws.exists}
                    <span class="missing-badge">No encontrado en disco</span>
                  {/if}
                </div>

                <div class="ws-path-row">
                  <Folder size={12} class="path-folder-icon" />
                  <span class="ws-path">{ws.display_path}</span>
                </div>

                <div class="ws-meta-row">
                  <span class="meta-item">
                    <FileText size={12} />
                    <span>{formatNotesCount(ws.notes_count)}</span>
                  </span>

                  <span class="meta-dot">•</span>

                  <span class="meta-item">
                    <Clock size={12} />
                    <span>{formatRelativeTime(ws.last_opened_ms)}</span>
                  </span>

                  {#if ws.is_git_repo || ws.git_branch}
                    <span class="meta-dot">•</span>
                    <span
                      class="meta-item git-branch-item"
                      class:feature-branch={ws.git_branch &&
                        ws.git_branch !== 'main' &&
                        ws.git_branch !== 'master'}
                    >
                      <GitBranch size={12} />
                      <span>{ws.git_branch || 'git'}</span>
                    </span>
                  {/if}

                  <span class="meta-dot">•</span>

                  <span class="status-ok-badge">
                    <Zap size={11} />
                    <span>{ws.exists ? '100% OK' : 'Offline'}</span>
                  </span>
                </div>
              </div>

              <!-- ACCIONES DERECHA -->
              <div class="ws-actions">
                <button
                  type="button"
                  class="ws-action-icon-btn"
                  class:pinned={ws.pinned}
                  onclick={(e) => handleTogglePin(ws, e)}
                  title={ws.pinned ? 'Quitar de fijados' : 'Fijar workspace'}
                  aria-label="Fijar workspace"
                >
                  <Star size={15} fill={ws.pinned ? 'currentColor' : 'none'} />
                </button>

                <button
                  type="button"
                  class="ws-action-icon-btn"
                  onclick={(e) => handleOpenInNewWindow(ws, e)}
                  title="Abrir en nueva ventana"
                  aria-label="Abrir en nueva ventana"
                >
                  <ExternalLink size={15} />
                </button>

                <button
                  type="button"
                  class="ws-open-btn"
                  onclick={(e) => {
                    e.stopPropagation();
                    handleOpenWorkspace(ws);
                  }}
                >
                  <span>Abrir</span>
                  <ArrowRight size={15} />
                </button>

                <button
                  type="button"
                  class="ws-action-icon-btn delete-btn"
                  onclick={(e) => handleRemoveWorkspace(ws, e)}
                  title="Quitar de la lista de recientes"
                  aria-label="Quitar de recientes"
                >
                  <Trash2 size={15} />
                </button>
              </div>
            </div>
          {/each}
        {/if}
      </div>

      <!-- PIE DEL PANEL PRINCIPAL -->
      <footer class="main-footer">
        <div class="suggested-templates">
          <Sparkles size={14} class="sparkle-icon" />
          <span class="template-label">Plantilla sugerida:</span>
          <button type="button" class="template-pill" onclick={handleCreateNewWorkspace}>
            Zettelkasten / Slip-box
          </button>
          <button type="button" class="template-pill" onclick={handleCreateNewWorkspace}>
            Rust Dev Log
          </button>
        </div>

        <div class="keyboard-hints">
          <span class="hint-pair"><kbd>Enter</kbd> para abrir</span>
          <span class="hint-pair"><kbd>Supr</kbd> quitar</span>
        </div>
      </footer>
    </main>
  </div>
</div>

<!-- MODAL PARA CLONAR REPOSITORIO GIT -->
{#if isCloneModalOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={() => !isCloning && (isCloneModalOpen = false)}>
    <div class="modal-dialog" onclick={(e) => e.stopPropagation()}>
      <div class="modal-header">
        <h3>Clonar repositorio Git</h3>
        <button
          type="button"
          class="modal-close-btn"
          onclick={() => (isCloneModalOpen = false)}
          disabled={isCloning}
        >
          <X size={16} />
        </button>
      </div>
      <form onsubmit={handleCloneSubmit} class="modal-body">
        <label for="git-url-input" class="modal-label">
          URL del repositorio remoto (HTTPS o SSH)
        </label>
        <input
          id="git-url-input"
          type="text"
          class="modal-input"
          placeholder="https://github.com/usuario/mi-boveda.git"
          bind:value={cloneRepoUrl}
          disabled={isCloning}
        />
        {#if cloneError}
          <p class="modal-error">{cloneError}</p>
        {/if}
        <div class="modal-actions">
          <button
            type="button"
            class="modal-btn secondary"
            onclick={() => (isCloneModalOpen = false)}
            disabled={isCloning}
          >
            Cancelar
          </button>
          <button
            type="submit"
            class="modal-btn primary"
            disabled={!cloneRepoUrl.trim() || isCloning}
          >
            {isCloning ? 'Clonando...' : 'Seleccionar destino y clonar'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- MODAL DE ATAJOS DE TECLADO -->
{#if isShortcutsOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={() => (isShortcutsOpen = false)}>
    <div class="modal-dialog" onclick={(e) => e.stopPropagation()}>
      <div class="modal-header">
        <h3>Atajos de teclado rápidos</h3>
        <button type="button" class="modal-close-btn" onclick={() => (isShortcutsOpen = false)}>
          <X size={16} />
        </button>
      </div>
      <div class="shortcuts-grid">
        <div class="shortcut-row">
          <span>Nuevo Workspace</span>
          <kbd>{shortcutsConfig.welcome.new_workspace}</kbd>
        </div>
        <div class="shortcut-row">
          <span>Abrir carpeta local</span>
          <kbd>{shortcutsConfig.welcome.open_folder}</kbd>
        </div>
        <div class="shortcut-row">
          <span>Clonar repositorio Git</span>
          <kbd>{shortcutsConfig.welcome.clone_git}</kbd>
        </div>
        <div class="shortcut-row">
          <span>Buscar en Workspaces recientes</span>
          <kbd>{shortcutsConfig.welcome.search_workspaces}</kbd>
        </div>
        <div class="shortcut-row">
          <span>Navegar en la lista</span>
          <kbd>↑ / ↓ / Enter</kbd>
        </div>
        <div class="shortcut-row">
          <span>Buscar archivos en Workspace</span>
          <kbd>{shortcutsConfig.workspace.quick_open}</kbd>
        </div>
        <div class="shortcut-row">
          <span>Nueva nota en Workspace</span>
          <kbd>{shortcutsConfig.workspace.new_file}</kbd>
        </div>
        <div class="shortcut-row">
          <span>Paleta de comandos</span>
          <kbd>{shortcutsConfig.workspace.command_palette}</kbd>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .welcome-root {
    position: fixed;
    inset: 0;
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background-color: #0f1115;
    color: #e6edf3;
    font-family: var(--main-font, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif);
    user-select: none;
    overflow: hidden;
    z-index: 2000;
  }

  :global(html[data-theme='light']) .welcome-root {
    background-color: #f6f8fa;
    color: #1f2328;
  }

  /* BARRA SUPERIOR */
  .welcome-titlebar {
    height: 40px;
    min-height: 40px;
    padding: 0 16px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    background-color: #0b0d10;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  }

  :global(html[data-theme='light']) .welcome-titlebar {
    background-color: #eaeef2;
    border-bottom: 1px solid #d0d7de;
  }

  .titlebar-left {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .app-brand-pill {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12.5px;
    color: #8b949e;
    font-family: var(--code-font, monospace);
  }

  .brand-pill-logo {
    width: 15px;
    height: 15px;
    object-fit: contain;
    user-select: none;
    -webkit-user-drag: none;
  }

  :global(.brand-chip-icon) {
    color: #38bdf8;
  }

  .brand-title {
    color: #c9d1d9;
    font-weight: 500;
  }

  :global(html[data-theme='light']) .brand-title {
    color: #24292f;
  }

  .brand-sep,
  .brand-version {
    color: #6e7681;
  }

  .titlebar-right {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .back-to-editor-btn {
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    background: rgba(56, 189, 248, 0.12);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.3);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .back-to-editor-btn:hover {
    background: rgba(56, 189, 248, 0.22);
  }

  .engine-status-pill {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 4px 10px;
    border-radius: 999px;
    background-color: rgba(16, 185, 129, 0.1);
    border: 1px solid rgba(16, 185, 129, 0.24);
    color: #34d399;
    font-size: 11.5px;
    font-family: var(--code-font, monospace);
  }

  .status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background-color: #10b981;
    box-shadow: 0 0 8px #10b981;
  }

  .top-icon-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: #8b949e;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background-color 0.15s ease, color 0.15s ease;
  }

  .top-icon-btn:hover {
    background-color: rgba(255, 255, 255, 0.08);
    color: #e6edf3;
  }

  :global(html[data-theme='light']) .top-icon-btn:hover {
    background-color: rgba(0, 0, 0, 0.06);
    color: #1f2328;
  }

  /* CUERPO PRINCIPAL */
  .welcome-body {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }

  /* SIDEBAR IZQUIERDO */
  .welcome-sidebar {
    width: 320px;
    min-width: 320px;
    background: linear-gradient(180deg, #14171d 0%, #101217 100%);
    border-right: 1px solid rgba(255, 255, 255, 0.07);
    padding: 28px 22px 20px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }

  :global(html[data-theme='light']) .welcome-sidebar {
    background: #ffffff;
    border-right: 1px solid #d0d7de;
  }

  .hero-brand {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }

  .logo-card {
    width: 84px;
    height: 84px;
    border-radius: 20px;
    background: radial-gradient(circle at 30% 25%, rgba(45, 212, 191, 0.14), rgba(15, 23, 42, 0.9));
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 12px 28px rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 16px;
  }

  .hero-logo-img {
    width: 56px;
    height: 56px;
    object-fit: contain;
    user-select: none;
    -webkit-user-drag: none;
    filter: drop-shadow(0 2px 8px rgba(0, 0, 0, 0.35));
  }

  :global(html[data-theme='light']) .logo-card {
    background: #f0f6fc;
    border-color: #d0d7de;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.06);
  }

  .hero-title {
    font-size: 24px;
    font-weight: 800;
    letter-spacing: 0.08em;
    margin: 0 0 10px;
    color: #f0f6fc;
  }

  :global(html[data-theme='light']) .hero-title {
    color: #1f2328;
  }

  .hero-badges {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
  }

  .rust-engine-badge {
    padding: 3px 8px;
    border-radius: 5px;
    background-color: rgba(245, 158, 11, 0.15);
    border: 1px solid rgba(245, 158, 11, 0.4);
    color: #f59e0b;
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0.05em;
    font-family: var(--code-font, monospace);
  }

  .local-vault-label {
    font-size: 12px;
    color: #8b949e;
  }

  .hero-description {
    font-size: 12.5px;
    line-height: 1.55;
    color: #8b949e;
    max-width: 260px;
  }

  /* SECCIÓN INICIO RÁPIDO */
  .quick-start-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 22px 0;
  }

  .section-heading {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: #8b949e;
    margin-bottom: 2px;
    padding-left: 2px;
  }

  .quick-action-btn {
    width: 100%;
    padding: 11px 13px;
    border-radius: 10px;
    background-color: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: #e6edf3;
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
    transition: all 0.16s ease;
    text-align: left;
  }

  :global(html[data-theme='light']) .quick-action-btn {
    background-color: #f6f8fa;
    border-color: #d0d7de;
    color: #1f2328;
  }

  .quick-action-btn:hover {
    background-color: rgba(255, 255, 255, 0.07);
    border-color: rgba(255, 255, 255, 0.16);
    transform: translateY(-1px);
  }

  :global(html[data-theme='light']) .quick-action-btn:hover {
    background-color: #eaeef2;
  }

  .quick-action-btn.primary-action {
    padding: 13px;
    background: linear-gradient(135deg, rgba(20, 184, 166, 0.16) 0%, rgba(15, 118, 110, 0.08) 100%);
    border-color: rgba(45, 212, 191, 0.35);
  }

  .quick-action-btn.primary-action:hover {
    background: linear-gradient(135deg, rgba(20, 184, 166, 0.24) 0%, rgba(15, 118, 110, 0.14) 100%);
    border-color: rgba(45, 212, 191, 0.55);
  }

  .qa-left {
    display: flex;
    align-items: center;
    gap: 11px;
    min-width: 0;
  }

  .qa-icon-wrap {
    width: 30px;
    height: 30px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .qa-icon-wrap.primary-icon {
    background-color: rgba(45, 212, 191, 0.18);
    color: #2dd4bf;
  }

  .qa-icon-wrap.amber-icon {
    color: #f59e0b;
  }

  .qa-icon-wrap.blue-icon {
    color: #38bdf8;
  }

  .qa-icon-wrap.orange-icon {
    color: #fb923c;
  }

  .qa-texts {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .qa-title {
    font-size: 13px;
    font-weight: 600;
    color: inherit;
  }

  .qa-subtitle {
    font-size: 11.5px;
    color: #8b949e;
  }

  .qa-kbd {
    padding: 3px 7px;
    border-radius: 5px;
    background-color: rgba(0, 0, 0, 0.35);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #8b949e;
    font-size: 11px;
    font-family: var(--code-font, monospace);
    flex-shrink: 0;
  }

  :global(html[data-theme='light']) .qa-kbd {
    background-color: #ffffff;
    border-color: #d0d7de;
    color: #57606a;
  }

  :global(.qa-arrow) {
    color: #8b949e;
  }

  /* FOOTER DEL SIDEBAR */
  .sidebar-footer {
    border-top: 1px solid rgba(255, 255, 255, 0.07);
    padding-top: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 12px;
  }

  :global(html[data-theme='light']) .sidebar-footer {
    border-top-color: #d0d7de;
  }

  .tech-spec-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: #8b949e;
  }

  .spec-label {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  :global(.spec-icon.teal) {
    color: #2dd4bf;
  }

  :global(.spec-icon.amber) {
    color: #f59e0b;
  }

  .spec-value {
    font-family: var(--code-font, monospace);
    font-size: 11.5px;
    color: #c9d1d9;
  }

  :global(html[data-theme='light']) .spec-value {
    color: #24292f;
  }

  .sidebar-footer-links {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 6px;
    padding-top: 6px;
  }

  .footer-link-btn {
    background: none;
    border: none;
    padding: 0;
    color: #8b949e;
    font-size: 12px;
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }

  .footer-link-btn:hover {
    color: #e6edf3;
  }

  .footer-license {
    color: #6e7681;
    font-size: 11.5px;
  }

  /* PANEL PRINCIPAL DERECHO */
  .welcome-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 28px 34px 18px;
    min-width: 0;
    overflow: hidden;
  }

  .main-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 20px;
    margin-bottom: 20px;
  }

  .title-with-count {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 5px;
  }

  .title-with-count h2 {
    font-size: 22px;
    font-weight: 700;
    margin: 0;
    color: inherit;
  }

  .count-pill {
    padding: 2px 9px;
    border-radius: 999px;
    background-color: rgba(255, 255, 255, 0.09);
    color: #c9d1d9;
    font-size: 12px;
    font-weight: 600;
  }

  :global(html[data-theme='light']) .count-pill {
    background-color: #eaeef2;
    color: #24292f;
  }

  .main-subtitle {
    font-size: 13px;
    color: #8b949e;
  }

  .search-box {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 300px;
    padding: 8px 12px;
    border-radius: 10px;
    background-color: #161920;
    border: 1px solid rgba(255, 255, 255, 0.09);
    transition: border-color 0.15s ease;
  }

  :global(html[data-theme='light']) .search-box {
    background-color: #ffffff;
    border-color: #d0d7de;
  }

  .search-box:focus-within {
    border-color: #38bdf8;
  }

  :global(.search-icon) {
    color: #8b949e;
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: inherit;
    font-size: 13px;
    min-width: 0;
  }

  .search-input::placeholder {
    color: #6e7681;
  }

  .search-kbd {
    padding: 2px 6px;
    border-radius: 4px;
    background-color: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #8b949e;
    font-size: 10.5px;
    font-family: var(--code-font, monospace);
  }

  /* BARRA DE FILTROS */
  .filter-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: 14px;
    margin-bottom: 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  }

  :global(html[data-theme='light']) .filter-bar {
    border-bottom-color: #d0d7de;
  }

  .filter-tabs {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .filter-tab {
    padding: 6px 14px;
    border-radius: 8px;
    border: 1px solid transparent;
    background: transparent;
    color: #8b949e;
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .filter-tab:hover {
    color: #e6edf3;
    background-color: rgba(255, 255, 255, 0.04);
  }

  :global(html[data-theme='light']) .filter-tab:hover {
    color: #1f2328;
    background-color: rgba(0, 0, 0, 0.05);
  }

  .filter-tab.active {
    background-color: rgba(255, 255, 255, 0.09);
    border-color: rgba(255, 255, 255, 0.12);
    color: #f0f6fc;
    font-weight: 600;
  }

  :global(html[data-theme='light']) .filter-tab.active {
    background-color: #ffffff;
    border-color: #d0d7de;
    color: #1f2328;
  }

  .sort-control {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    color: #8b949e;
  }

  .sort-select {
    padding: 5px 10px;
    border-radius: 7px;
    background-color: #161920;
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #c9d1d9;
    font-size: 12.5px;
    outline: none;
    cursor: pointer;
  }

  :global(html[data-theme='light']) .sort-select {
    background-color: #ffffff;
    border-color: #d0d7de;
    color: #1f2328;
  }

  /* LISTA DE WORKSPACES */
  .workspaces-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-right: 4px;
  }

  .empty-list-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 10px;
    padding: 40px 24px;
    color: #8b949e;
  }

  :global(.empty-folder-icon) {
    color: #f59e0b;
    opacity: 0.8;
  }

  .empty-list-state h3 {
    font-size: 16px;
    font-weight: 600;
    color: #e6edf3;
    margin: 0;
  }

  :global(html[data-theme='light']) .empty-list-state h3 {
    color: #1f2328;
  }

  .empty-list-state p {
    font-size: 13px;
    max-width: 440px;
    line-height: 1.5;
  }

  .open-primary-cta {
    margin-top: 8px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 18px;
    border-radius: 8px;
    background: linear-gradient(135deg, #f59e0b 0%, #ea580c 100%);
    color: #0f1115;
    font-weight: 700;
    font-size: 13px;
    border: none;
    cursor: pointer;
  }

  /* TARJETA DE WORKSPACE */
  .workspace-card {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px 20px;
    border-radius: 14px;
    background-color: #15181e;
    border: 1px solid rgba(255, 255, 255, 0.08);
    cursor: pointer;
    transition: all 0.16s ease;
  }

  :global(html[data-theme='light']) .workspace-card {
    background-color: #ffffff;
    border-color: #d0d7de;
  }

  .workspace-card:hover,
  .workspace-card.selected {
    background-color: #1a1e26;
    border-color: rgba(245, 158, 11, 0.35);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.3);
  }

  :global(html[data-theme='light']) .workspace-card:hover,
  :global(html[data-theme='light']) .workspace-card.selected {
    background-color: #f0f6fc;
    border-color: #0969da;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.06);
  }

  .ws-icon-box {
    width: 46px;
    height: 46px;
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .ws-icon-box.pinned-box {
    background-color: rgba(245, 158, 11, 0.12);
    border: 1px solid rgba(245, 158, 11, 0.28);
    color: #f59e0b;
  }

  .ws-icon-box.teal-box {
    background-color: rgba(45, 212, 191, 0.12);
    border: 1px solid rgba(45, 212, 191, 0.25);
    color: #2dd4bf;
  }

  .ws-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .ws-title-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .ws-name {
    font-size: 15.5px;
    font-weight: 700;
    color: #f0f6fc;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  :global(html[data-theme='light']) .ws-name {
    color: #1f2328;
  }

  .pinned-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 6px;
    background-color: rgba(245, 158, 11, 0.15);
    border: 1px solid rgba(245, 158, 11, 0.35);
    color: #fbbf24;
    font-size: 11px;
    font-weight: 600;
  }

  .missing-badge {
    padding: 2px 8px;
    border-radius: 6px;
    background-color: rgba(248, 81, 73, 0.15);
    border: 1px solid rgba(248, 81, 73, 0.35);
    color: #f85149;
    font-size: 11px;
  }

  .ws-path-row {
    display: flex;
    align-items: center;
    gap: 6px;
    color: #8b949e;
    font-family: var(--code-font, monospace);
    font-size: 12px;
  }

  .ws-path {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ws-meta-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 2px;
    font-size: 12px;
    color: #8b949e;
  }

  .meta-item {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .meta-dot {
    color: #484f58;
  }

  .git-branch-item {
    color: #34d399;
    font-family: var(--code-font, monospace);
  }

  .git-branch-item.feature-branch {
    color: #fbbf24;
  }

  .status-ok-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 6px;
    background-color: rgba(56, 189, 248, 0.1);
    border: 1px solid rgba(56, 189, 248, 0.22);
    color: #38bdf8;
    font-size: 11px;
    font-family: var(--code-font, monospace);
  }

  /* BOTONES DE ACCIÓN EN CADA TARJETA */
  .ws-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .ws-action-icon-btn {
    width: 34px;
    height: 34px;
    border-radius: 8px;
    background-color: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.09);
    color: #8b949e;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  :global(html[data-theme='light']) .ws-action-icon-btn {
    background-color: #f6f8fa;
    border-color: #d0d7de;
  }

  .ws-action-icon-btn:hover {
    background-color: rgba(255, 255, 255, 0.09);
    color: #f0f6fc;
  }

  :global(html[data-theme='light']) .ws-action-icon-btn:hover {
    background-color: #eaeef2;
    color: #1f2328;
  }

  .ws-action-icon-btn.pinned {
    color: #fbbf24;
    border-color: rgba(245, 158, 11, 0.35);
    background-color: rgba(245, 158, 11, 0.1);
  }

  .ws-action-icon-btn.delete-btn:hover {
    color: #f85149;
    border-color: rgba(248, 81, 73, 0.4);
    background-color: rgba(248, 81, 73, 0.12);
  }

  .ws-open-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 8px 16px;
    border-radius: 9px;
    background: linear-gradient(135deg, #f59e0b 0%, #ea580c 100%);
    color: #0d1117;
    font-size: 13px;
    font-weight: 700;
    border: none;
    cursor: pointer;
    transition: filter 0.15s ease, transform 0.15s ease;
  }

  .ws-open-btn:hover {
    filter: brightness(1.08);
    transform: translateY(-1px);
  }

  /* FOOTER PRINCIPAL */
  .main-footer {
    margin-top: 16px;
    padding-top: 14px;
    border-top: 1px solid rgba(255, 255, 255, 0.07);
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
    color: #8b949e;
  }

  :global(html[data-theme='light']) .main-footer {
    border-top-color: #d0d7de;
  }

  .suggested-templates {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  :global(.sparkle-icon) {
    color: #f59e0b;
  }

  .template-pill {
    padding: 4px 10px;
    border-radius: 6px;
    background-color: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.09);
    color: #c9d1d9;
    font-size: 11.5px;
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  :global(html[data-theme='light']) .template-pill {
    background-color: #ffffff;
    border-color: #d0d7de;
    color: #24292f;
  }

  .template-pill:hover {
    background-color: rgba(255, 255, 255, 0.1);
  }

  .keyboard-hints {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .hint-pair {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .hint-pair kbd {
    padding: 2px 6px;
    border-radius: 4px;
    background-color: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    font-family: var(--code-font, monospace);
    font-size: 11px;
    color: #c9d1d9;
  }

  /* MODALES */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background-color: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(3px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2500;
  }

  .modal-dialog {
    width: 440px;
    max-width: 92vw;
    border-radius: 14px;
    background-color: #161920;
    border: 1px solid rgba(255, 255, 255, 0.12);
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6);
    padding: 20px;
    color: #e6edf3;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .modal-header h3 {
    font-size: 16px;
    font-weight: 700;
    margin: 0;
  }

  .modal-close-btn {
    background: transparent;
    border: none;
    color: #8b949e;
    cursor: pointer;
    padding: 4px;
    border-radius: 6px;
  }

  .modal-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .modal-label {
    font-size: 12.5px;
    color: #8b949e;
  }

  .modal-input {
    width: 100%;
    padding: 10px 12px;
    border-radius: 8px;
    background-color: #0d1117;
    border: 1px solid rgba(255, 255, 255, 0.14);
    color: #e6edf3;
    font-size: 13px;
    font-family: var(--code-font, monospace);
    outline: none;
  }

  .modal-input:focus {
    border-color: #38bdf8;
  }

  .modal-error {
    color: #f85149;
    font-size: 12px;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 6px;
  }

  .modal-btn {
    padding: 8px 14px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    border: none;
  }

  .modal-btn.secondary {
    background-color: rgba(255, 255, 255, 0.08);
    color: #c9d1d9;
  }

  .modal-btn.primary {
    background: linear-gradient(135deg, #f59e0b 0%, #ea580c 100%);
    color: #0d1117;
  }

  .modal-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .shortcuts-grid {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    border-radius: 8px;
    background-color: rgba(255, 255, 255, 0.03);
    font-size: 13px;
  }

  .shortcut-row kbd {
    padding: 3px 8px;
    border-radius: 5px;
    background-color: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.12);
    font-family: var(--code-font, monospace);
    font-size: 11.5px;
  }
</style>
