<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import {
    vaultRepository,
    isTauriEnvironment,
    type RecentWorkspaceEntry,
    type SelectVaultFolderResult,
  } from '@shared/repositories';
  import { systemApi } from '@shared/api';
  import { appSettings, shortcutsConfig, matchesShortcut } from '@entities/settings';
  import { t } from '@shared/lib/i18n';
  import { ThemeToggle } from '@features/theme-toggle';
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
    HelpCircle,
    Settings,
    Minus,
    Square,
    X,
    Cpu,
    Zap,
    Languages,
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
  let appVersion = $state('1.0.0-alpha.1');
  let isLoading = $state(true);
  let activeFilter = $state<FilterTab>('all');
  let sortOrder = $state<SortOrder>('recent');
  let searchQuery = $state('');
  let selectedIndex = $state(0);
  let isKeyboardNav = $state(false);
  let hoveredIndex: number | null = null;
  let searchInputRef = $state<HTMLInputElement | null>(null);

  const numberFormatters = {
    es: new Intl.NumberFormat('es-ES'),
    en: new Intl.NumberFormat('en-US'),
  };
  const dateFormatters = {
    es: new Intl.DateTimeFormat('es-ES', { day: '2-digit', month: 'short' }),
    en: new Intl.DateTimeFormat('en-US', { day: '2-digit', month: 'short' }),
  };

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
    const len = filteredWorkspaces.length;
    if (len === 0) {
      selectedIndex = 0;
    } else if (selectedIndex >= len) {
      selectedIndex = len - 1;
    } else if (selectedIndex < 0) {
      selectedIndex = 0;
    }
  });

  function formatRelativeTime(timestampMs: number): string {
    if (!timestampMs) return t('welcome.timeRecent');
    const diffSec = Math.max(0, Math.floor((Date.now() - timestampMs) / 1000));
    if (diffSec < 60) return t('welcome.timeJustNow');
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return t('welcome.timeMinAgo', { min: diffMin });
    const diffHours = Math.floor(diffMin / 60);
    if (diffHours < 24) return diffHours === 1 ? t('welcome.timeHourAgo') : t('welcome.timeHoursAgo', { hours: diffHours });
    const diffDays = Math.floor(diffHours / 24);
    if (diffDays === 1) {
      const d = new Date(timestampMs);
      const hh = String(d.getHours()).padStart(2, '0');
      const mm = String(d.getMinutes()).padStart(2, '0');
      return t('welcome.timeYesterday', { time: `${hh}:${mm}` });
    }
    if (diffDays < 30) return t('welcome.timeDaysAgo', { days: diffDays });
    const d = new Date(timestampMs);
    const loc = appSettings.locale === 'es' ? 'es' : 'en';
    return dateFormatters[loc].format(d);
  }

  function formatNotesCount(count: number): string {
    const loc = appSettings.locale === 'es' ? 'es' : 'en';
    const formatted = numberFormatters[loc].format(count);
    return count === 1 ? t('welcome.notesCountSingle', { count: formatted }) : t('welcome.notesCountPlural', { count: formatted });
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
        const base = !isKeyboardNav && hoveredIndex !== null ? hoveredIndex : selectedIndex;
        isKeyboardNav = true;
        selectedIndex = (base + 1) % filteredWorkspaces.length;
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (filteredWorkspaces.length > 0) {
        const base = !isKeyboardNav && hoveredIndex !== null ? hoveredIndex : selectedIndex;
        isKeyboardNav = true;
        selectedIndex =
          (base - 1 + filteredWorkspaces.length) % filteredWorkspaces.length;
      }
    } else if (e.key === 'Enter') {
      const activeEl = document.activeElement;
      const isButtonFocused =
        activeEl instanceof HTMLButtonElement || activeEl instanceof HTMLSelectElement;
      const targetIdx = !isKeyboardNav && hoveredIndex !== null ? hoveredIndex : selectedIndex;
      if (!isButtonFocused && filteredWorkspaces[targetIdx]) {
        e.preventDefault();
        handleOpenWorkspace(filteredWorkspaces[targetIdx]);
      }
    }
  }

  onMount(() => {
    shortcutsConfig.loadFromBackend();
    systemApi.getAppVersion().then((v) => {
      if (v) appVersion = v;
    }).catch(() => {});
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
        <span class="brand-version">v{appVersion}</span>
      </div>
    </div>

    <div class="titlebar-right" data-tauri-drag-region="false">
      {#if canCloseWelcome && onCloseWelcome}
        <button
          type="button"
          class="back-to-editor-btn"
          onclick={onCloseWelcome}
          title={t('welcome.backToWorkspaceTitle')}
        >
          <span>{t('welcome.backToWorkspace')}</span>
        </button>
      {/if}

      <div class="engine-status-pill" title={t('welcome.tantivyTitle')}>
        <span class="status-dot"></span>
        <span>{t('welcome.tantivyCoreOnline')}</span>
      </div>

      <button
        type="button"
        class="top-icon-btn"
        onclick={() => appSettings.toggleLocale()}
        title={t('ribbon.languageTooltip', { lang: appSettings.locale === 'es' ? 'Español' : 'English' })}
        aria-label={t('ribbon.toggleLanguageAria', { lang: appSettings.locale === 'es' ? 'English' : 'Español' })}
      >
        <Languages size={15} />
      </button>

      <button
        type="button"
        class="top-icon-btn"
        onclick={() => (isShortcutsOpen = true)}
        title={t('welcome.helpTitle')}
        aria-label={t('welcome.helpAria')}
      >
        <HelpCircle size={15} />
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
          <span class="rust-engine-badge">{t('welcome.heroEngineBadge')}</span>
          <span class="local-vault-label">{t('welcome.heroVaultLabel')}</span>
        </div>

        <p class="hero-description">
          {t('welcome.heroDescription')}
        </p>
      </div>

      <!-- INICIO RÁPIDO -->
      <div class="quick-start-section">
        <span class="section-heading">{t('welcome.quickStartSection')}</span>

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
              <span class="qa-title">{t('welcome.newWorkspaceTitle')}</span>
              <span class="qa-subtitle">{t('welcome.newWorkspaceSubtitle')}</span>
            </div>
          </div>
          <kbd class="qa-kbd">{shortcutsConfig.welcome.new_workspace}</kbd>
        </button>

        <button type="button" class="quick-action-btn" onclick={handleSelectLocalFolder}>
          <div class="qa-left">
            <div class="qa-icon-wrap amber-icon">
              <FolderOpen size={17} />
            </div>
            <span class="qa-title">{t('welcome.openLocalFolder')}</span>
          </div>
          <kbd class="qa-kbd">{shortcutsConfig.welcome.open_folder}</kbd>
        </button>

        <button type="button" class="quick-action-btn" onclick={handleOpenStandaloneFile}>
          <div class="qa-left">
            <div class="qa-icon-wrap blue-icon">
              <FileText size={17} />
            </div>
            <div class="qa-texts">
              <span class="qa-title">{t('welcome.openStandaloneFileTitle')}</span>
              <span class="qa-subtitle">{t('welcome.openStandaloneFileSubtitle')}</span>
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
            <span class="qa-title">{t('welcome.cloneGit')}</span>
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
            <span>{t('welcome.tantivyStatus')}</span>
          </div>
          <span class="spec-value">0ms Latencia</span>
        </div>

        <div class="sidebar-footer-links">
          <button type="button" class="footer-link-btn" onclick={() => (isShortcutsOpen = true)}>
            {t('welcome.shortcutsLink')}
          </button>
          <span class="footer-license">{t('welcome.mitLicense')}</span>
        </div>
      </div>
    </aside>

    <!-- PANEL PRINCIPAL DERECHO: WORKSPACES RECIENTES -->
    <main class="welcome-main">
      <!-- CABECERA Y BUSCADOR -->
      <div class="main-header">
        <div class="main-header-titles">
          <div class="title-with-count">
            <h2>{t('welcome.recentWorkspacesHeading')}</h2>
            <span class="count-pill">{workspaces.length}</span>
          </div>
          <p class="main-subtitle">
            {t('welcome.recentWorkspacesSubtitle')}
          </p>
        </div>

        <div class="search-box">
          <Search size={15} class="search-icon" />
          <input
            type="text"
            bind:this={searchInputRef}
            bind:value={searchQuery}
            placeholder={t('welcome.searchPlaceholder')}
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
            {t('welcome.filterAll')}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeFilter === 'pinned'}
            class="filter-tab"
            class:active={activeFilter === 'pinned'}
            onclick={() => (activeFilter = 'pinned')}
          >
            {t('welcome.filterPinned')}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeFilter === 'git'}
            class="filter-tab"
            class:active={activeFilter === 'git'}
            onclick={() => (activeFilter = 'git')}
          >
            {t('welcome.filterGit')}
          </button>
        </div>

        <div class="sort-control">
          <span class="sort-label">{t('welcome.sortLabel')}</span>
          <select bind:value={sortOrder} class="sort-select" aria-label={t('welcome.sortLabel')}>
            <option value="recent">{t('welcome.sortRecent')}</option>
            <option value="name">{t('welcome.sortName')}</option>
            <option value="notes">{t('welcome.sortNotes')}</option>
          </select>
        </div>
      </div>

      <!-- LISTA DE TARJETAS DE WORKSPACE -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="workspaces-list"
        class:keyboard-nav={isKeyboardNav}
        onpointermove={() => {
          if (isKeyboardNav) isKeyboardNav = false;
        }}
        onpointerleave={() => {
          hoveredIndex = null;
        }}
      >
        {#if isLoading}
          <div class="empty-list-state">
            <span>{t('welcome.loadingWorkspaces')}</span>
          </div>
        {:else if filteredWorkspaces.length === 0}
          <div class="empty-list-state">
            <FolderOpen size={42} strokeWidth={1.4} class="empty-folder-icon" />
            {#if workspaces.length === 0}
              <h3>{t('welcome.emptyWorkspacesTitle')}</h3>
              <p>
                {t('welcome.emptyWorkspacesDescription')}
              </p>
              <button type="button" class="open-primary-cta" onclick={handleSelectLocalFolder}>
                <FolderOpen size={16} />
                <span>{t('welcome.openFolderCta')}</span>
              </button>
            {:else}
              <h3>{t('welcome.emptyFilterTitle')}</h3>
              <p>{t('welcome.emptyFilterDescription')}</p>
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
              onpointerenter={() => {
                hoveredIndex = idx;
              }}
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
                      <span>{t('welcome.pinnedBadge')}</span>
                    </span>
                  {/if}
                  {#if !ws.exists}
                    <span class="missing-badge">{t('welcome.missingDiskBadge')}</span>
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
                    <span>{ws.exists ? t('welcome.statusOk') : t('welcome.statusOffline')}</span>
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
                  title={ws.pinned ? t('welcome.unpinWorkspace') : t('welcome.pinWorkspace')}
                  aria-label={t('welcome.pinWorkspace')}
                >
                  <Star size={15} fill={ws.pinned ? 'currentColor' : 'none'} />
                </button>

                <button
                  type="button"
                  class="ws-action-icon-btn"
                  onclick={(e) => handleOpenInNewWindow(ws, e)}
                  title={t('welcome.openInNewWindow')}
                  aria-label={t('welcome.openInNewWindow')}
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
                  <span>{t('welcome.openWorkspaceBtn')}</span>
                  <ArrowRight size={15} />
                </button>

                <button
                  type="button"
                  class="ws-action-icon-btn delete-btn"
                  onclick={(e) => handleRemoveWorkspace(ws, e)}
                  title={t('welcome.removeRecentWorkspace')}
                  aria-label={t('welcome.removeRecentWorkspace')}
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
        <!-- Botón de Alternar Tema (Claro / Oscuro / Sistema) -->
        <ThemeToggle />

        <div class="keyboard-hints">
          <span class="hint-pair"><kbd>Enter</kbd> {t('welcome.hintOpen')}</span>
          <span class="hint-pair"><kbd>Supr</kbd> {t('welcome.hintRemove')}</span>
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
        <h3>{t('welcome.cloneModalTitle')}</h3>
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
          {t('welcome.cloneRemoteRepoUrlLabel')}
        </label>
        <input
          id="git-url-input"
          type="text"
          class="modal-input"
          placeholder={t('welcome.cloneRepoUrlPlaceholder')}
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
            {t('common.cancel')}
          </button>
          <button
            type="submit"
            class="modal-btn primary"
            disabled={!cloneRepoUrl.trim() || isCloning}
          >
            {isCloning ? t('common.loading') : t('welcome.cloneSelectDestAndClone')}
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
        <h3>{t('welcome.shortcutsModalTitle')}</h3>
        <button type="button" class="modal-close-btn" onclick={() => (isShortcutsOpen = false)}>
          <X size={16} />
        </button>
      </div>
      <div class="shortcuts-grid">
        <div class="shortcut-row">
          <span>{t('welcome.newWorkspaceTitle')}</span>
          <kbd>{shortcutsConfig.welcome.new_workspace}</kbd>
        </div>
        <div class="shortcut-row">
          <span>{t('welcome.openLocalFolder')}</span>
          <kbd>{shortcutsConfig.welcome.open_folder}</kbd>
        </div>
        <div class="shortcut-row">
          <span>{t('welcome.cloneGit')}</span>
          <kbd>{shortcutsConfig.welcome.clone_git}</kbd>
        </div>
        <div class="shortcut-row">
          <span>{t('welcome.recentWorkspacesHeading')}</span>
          <kbd>{shortcutsConfig.welcome.search_workspaces}</kbd>
        </div>
        <div class="shortcut-row">
          <span>{t('welcome.shortcutNavList')}</span>
          <kbd>↑ / ↓ / Enter</kbd>
        </div>
        <div class="shortcut-row">
          <span>{t('welcome.shortcutFindFiles')}</span>
          <kbd>{shortcutsConfig.workspace.quick_open}</kbd>
        </div>
        <div class="shortcut-row">
          <span>{t('welcome.shortcutNewNote')}</span>
          <kbd>{shortcutsConfig.workspace.new_file}</kbd>
        </div>
        <div class="shortcut-row">
          <span>{t('welcome.shortcutCommandPalette')}</span>
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
    background-color: var(--bg-primary);
    color: var(--text-primary);
    font-family: var(--main-font, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif);
    user-select: none;
    overflow: hidden;
    z-index: 2000;
    contain: strict;
  }

  /* BARRA SUPERIOR */
  .welcome-titlebar {
    height: 40px;
    min-height: 40px;
    padding: 0 16px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    background-color: var(--bg-tertiary);
    border-bottom: 1px solid var(--border-primary);
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
    color: var(--text-secondary);
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
    color: var(--accent);
  }

  .brand-title {
    color: var(--text-primary);
    font-weight: 500;
  }

  .brand-sep,
  .brand-version {
    color: var(--text-secondary);
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
    background: var(--accent-bg);
    color: var(--accent);
    border: 1px solid var(--accent-border);
    cursor: pointer;
    transition: background-color 0.08s ease;
  }

  .back-to-editor-btn:hover {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .engine-status-pill {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 4px 10px;
    border-radius: 999px;
    background-color: var(--success-bg);
    border: 1px solid var(--success-border);
    color: var(--success);
    font-size: 11.5px;
    font-family: var(--code-font, monospace);
  }

  .status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background-color: var(--success);
  }

  .top-icon-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background-color 0.08s ease, color 0.08s ease;
  }

  .top-icon-btn:hover {
    background-color: var(--hover-bg);
    color: var(--text-primary);
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
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-primary);
    padding: 28px 22px 20px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
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
    background: radial-gradient(circle at 30% 25%, color-mix(in srgb, var(--success) 14%, transparent), var(--bg-secondary));
    border: 1px solid var(--border-primary);
    box-shadow: var(--popover-shadow);
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
  }

  .hero-title {
    font-size: 24px;
    font-weight: 800;
    letter-spacing: 0.08em;
    margin: 0 0 10px;
    color: var(--text-primary);
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
    background-color: var(--warning-bg);
    border: 1px solid var(--warning-border);
    color: var(--warning);
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0.05em;
    font-family: var(--code-font, monospace);
  }

  .local-vault-label {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .hero-description {
    font-size: 12.5px;
    line-height: 1.55;
    color: var(--text-secondary);
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
    color: var(--text-secondary);
    margin-bottom: 2px;
    padding-left: 2px;
  }

  .quick-action-btn {
    width: 100%;
    padding: 11px 13px;
    border-radius: 10px;
    background-color: var(--hover-bg-subtle);
    border: 1px solid var(--border-primary);
    color: var(--text-primary);
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
    transition: background-color 0.08s ease, border-color 0.08s ease;
    text-align: left;
  }

  .quick-action-btn:hover {
    background-color: var(--hover-bg);
    border-color: var(--border-primary);
  }

  .quick-action-btn.primary-action {
    padding: 13px;
    background-color: var(--success-bg);
    border-color: var(--success-border);
  }

  .quick-action-btn.primary-action:hover {
    background-color: color-mix(in srgb, var(--success) 22%, transparent);
    border-color: color-mix(in srgb, var(--success) 55%, transparent);
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
    background-color: color-mix(in srgb, var(--success) 18%, transparent);
    color: var(--success);
  }

  .qa-icon-wrap.amber-icon {
    color: var(--warning);
  }

  .qa-icon-wrap.blue-icon {
    color: var(--accent);
  }

  .qa-icon-wrap.orange-icon {
    color: var(--orange);
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
    color: var(--text-secondary);
  }

  .qa-kbd {
    padding: 3px 7px;
    border-radius: 5px;
    background-color: var(--bg-tertiary);
    border: 1px solid var(--border-primary);
    color: var(--text-secondary);
    font-size: 11px;
    font-family: var(--code-font, monospace);
    flex-shrink: 0;
  }

  :global(.qa-arrow) {
    color: var(--text-secondary);
  }

  /* FOOTER DEL SIDEBAR */
  .sidebar-footer {
    border-top: 1px solid var(--border-primary);
    padding-top: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 12px;
  }

  .tech-spec-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: var(--text-secondary);
  }

  .spec-label {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  :global(.spec-icon.teal) {
    color: var(--success);
  }

  :global(.spec-icon.amber) {
    color: var(--warning);
  }

  .spec-value {
    font-family: var(--code-font, monospace);
    font-size: 11.5px;
    color: var(--text-primary);
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
    color: var(--text-secondary);
    font-size: 12px;
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }

  .footer-link-btn:hover {
    color: var(--text-primary);
  }

  .footer-license {
    color: var(--text-secondary);
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
    background-color: var(--hover-bg);
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 600;
  }

  .main-subtitle {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .search-box {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 300px;
    padding: 8px 12px;
    border-radius: 10px;
    background-color: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    transition: border-color 0.15s ease;
  }

  .search-box:focus-within {
    border-color: var(--accent);
  }

  :global(.search-icon) {
    color: var(--text-secondary);
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
    color: var(--text-secondary);
  }

  .search-kbd {
    padding: 2px 6px;
    border-radius: 4px;
    background-color: var(--hover-bg);
    border: 1px solid var(--border-primary);
    color: var(--text-secondary);
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
    border-bottom: 1px solid var(--border-primary);
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
    color: var(--text-secondary);
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.08s ease, color 0.08s ease, border-color 0.08s ease;
  }

  .filter-tab:hover {
    color: var(--text-primary);
    background-color: var(--hover-bg-subtle);
  }

  .filter-tab.active {
    background-color: var(--hover-bg);
    border-color: var(--border-primary);
    color: var(--text-primary);
    font-weight: 600;
  }

  .sort-control {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    color: var(--text-secondary);
  }

  .sort-select {
    padding: 5px 10px;
    border-radius: 7px;
    background-color: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    color: var(--text-primary);
    font-size: 12.5px;
    outline: none;
    cursor: pointer;
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
    color: var(--text-secondary);
  }

  :global(.empty-folder-icon) {
    color: var(--warning);
    opacity: 0.8;
  }

  .empty-list-state h3 {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
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
    background: linear-gradient(135deg, #f59e0b 0%, #ea580c 100%); /* fijo: gradiente de marca del botón */
    color: #0f1115; /* fijo: texto sobre el gradiente de marca */
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
    background-color: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    cursor: pointer;
    contain: layout style paint;
    transition: background-color 0.08s ease, border-color 0.08s ease;
  }

  .workspaces-list:not(:hover) .workspace-card.selected,
  .workspaces-list.keyboard-nav .workspace-card.selected,
  .workspaces-list:not(.keyboard-nav) .workspace-card:hover {
    background-color: var(--bg-tertiary);
    border-color: var(--warning-border);
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
    background-color: var(--warning-bg);
    border: 1px solid var(--warning-border);
    color: var(--warning);
  }

  .ws-icon-box.teal-box {
    background-color: var(--success-bg);
    border: 1px solid var(--success-border);
    color: var(--success);
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
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .pinned-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 6px;
    background-color: var(--warning-bg);
    border: 1px solid var(--warning-border);
    color: var(--warning);
    font-size: 11px;
    font-weight: 600;
  }

  .missing-badge {
    padding: 2px 8px;
    border-radius: 6px;
    background-color: var(--danger-bg);
    border: 1px solid var(--danger-border);
    color: var(--danger);
    font-size: 11px;
  }

  .ws-path-row {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
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
    color: var(--text-secondary);
  }

  .meta-item {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .meta-dot {
    color: color-mix(in srgb, var(--text-secondary) 55%, transparent);
  }

  .git-branch-item {
    color: var(--success);
    font-family: var(--code-font, monospace);
  }

  .git-branch-item.feature-branch {
    color: var(--warning);
  }

  .status-ok-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 6px;
    background-color: var(--accent-bg);
    border: 1px solid var(--accent-border);
    color: var(--accent);
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
    background-color: var(--hover-bg-subtle);
    border: 1px solid var(--border-primary);
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background-color 0.08s ease, color 0.08s ease, border-color 0.08s ease;
  }

  .ws-action-icon-btn:hover {
    background-color: var(--hover-bg);
    color: var(--text-primary);
  }

  .ws-action-icon-btn.pinned {
    color: var(--warning);
    border-color: var(--warning-border);
    background-color: var(--warning-bg);
  }

  .ws-action-icon-btn.delete-btn:hover {
    color: var(--danger);
    border-color: var(--danger-border);
    background-color: var(--danger-bg);
  }

  .ws-open-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 8px 16px;
    border-radius: 9px;
    background-color: #f59e0b; /* fijo: color de marca del botón */
    color: #0d1117; /* fijo: texto sobre el botón de marca */
    font-size: 13px;
    font-weight: 700;
    border: none;
    cursor: pointer;
    transition: background-color 0.08s ease;
  }

  .ws-open-btn:hover {
    background-color: #fbbf24; /* fijo: color de marca del botón */
  }

  /* FOOTER PRINCIPAL */
  .main-footer {
    margin-top: 16px;
    padding-top: 14px;
    border-top: 1px solid var(--border-primary);
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
    color: var(--text-secondary);
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
    background-color: var(--hover-bg);
    border: 1px solid var(--border-primary);
    font-family: var(--code-font, monospace);
    font-size: 11px;
    color: var(--text-primary);
  }

  /* MODALES */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background-color: var(--overlay-bg);
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
    background-color: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    box-shadow: var(--modal-shadow);
    padding: 20px;
    color: var(--text-primary);
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
    color: var(--text-secondary);
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
    color: var(--text-secondary);
  }

  .modal-input {
    width: 100%;
    padding: 10px 12px;
    border-radius: 8px;
    background-color: var(--bg-primary);
    border: 1px solid var(--border-primary);
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--code-font, monospace);
    outline: none;
  }

  .modal-input:focus {
    border-color: var(--accent);
  }

  .modal-error {
    color: var(--danger);
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
    background-color: var(--hover-bg);
    color: var(--text-primary);
  }

  .modal-btn.primary {
    background: linear-gradient(135deg, #f59e0b 0%, #ea580c 100%); /* fijo: gradiente de marca del botón */
    color: #0d1117; /* fijo: texto sobre el gradiente de marca */
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
    background-color: var(--hover-bg-subtle);
    font-size: 13px;
  }

  .shortcut-row kbd {
    padding: 3px 8px;
    border-radius: 5px;
    background-color: var(--hover-bg);
    border: 1px solid var(--border-primary);
    font-family: var(--code-font, monospace);
    font-size: 11.5px;
  }
</style>