<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { isTauriEnvironment } from '@shared/api';
  import type { TabItem } from '@entities/vault-item';
  import type { MarkdownViewMode } from '@widgets/status-bar';

  export type { MarkdownViewMode };

  interface Props {
    title?: string;
    tabs?: TabItem[];
    activeTabPath?: string | null;
    isEditing?: boolean;
    isMarkdownFile?: boolean;
    markdownViewMode?: MarkdownViewMode;
    showSaveButton?: boolean;
    onSelectTab?: (path: string) => void;
    onCloseTab?: (path: string) => void;
    onCloseAllTabs?: () => void;
    onNewTab?: () => void;
    onNewFile?: () => void;
    onToggleView?: () => void;
    onChangeMarkdownView?: (mode: MarkdownViewMode) => void;
    onSplitView?: () => void;
    onOpenCommandPalette?: () => void;
    onOpenQuickOpen?: () => void;
    onSave?: () => void;
    onAction?: (actionId: string) => void;
  }

  let {
    title = 'Nota de bienvenida.md',
    tabs = [],
    activeTabPath = null,
    isEditing = $bindable(true),
    isMarkdownFile = false,
    markdownViewMode = 'live',
    showSaveButton = false,
    onSelectTab,
    onCloseTab,
    onCloseAllTabs,
    onNewTab,
    onNewFile,
    onToggleView,
    onChangeMarkdownView,
    onSplitView,
    onOpenCommandPalette,
    onOpenQuickOpen,
    onSave,
    onAction
  }: Props = $props();

  let tabsScrollRef = $state<HTMLDivElement | null>(null);
  let menuContainerRef = $state<HTMLDivElement | null>(null);
  let searchInputRef = $state<HTMLInputElement | null>(null);
  let tabsListRef = $state<HTMLDivElement | null>(null);

  let canScrollLeft = $state(false);
  let canScrollRight = $state(false);
  let isTabsMenuOpen = $state(false);

  let searchQuery = $state('');
  let selectedIndex = $state(0);

  // Estados para el selector de vistas Markdown
  let isViewMenuOpen = $state(false);
  let selectedViewIndex = $state(0);
  let viewContainerRef = $state<HTMLDivElement | null>(null);

  const MARKDOWN_VIEW_MODES: {
    id: MarkdownViewMode;
    label: string;
    badge: string;
    description: string;
  }[] = [
    {
      id: 'live',
      label: 'En vivo',
      badge: 'Live Preview',
      description: 'Editor con diagramas interactivos y Markdown rico',
    },
    {
      id: 'source',
      label: 'Fuente',
      badge: 'Source Mode',
      description: 'Código Markdown puro con resaltado de sintaxis',
    },
    {
      id: 'reading',
      label: 'Lectura',
      badge: 'Reading View',
      description: 'Documento renderizado final de solo lectura',
    },
  ];

  function toggleViewMenu(e: MouseEvent) {
    e.stopPropagation();
    isTabsMenuOpen = false;
    isViewMenuOpen = !isViewMenuOpen;
    if (isViewMenuOpen) {
      const currentIdx = MARKDOWN_VIEW_MODES.findIndex(m => m.id === markdownViewMode);
      selectedViewIndex = currentIdx !== -1 ? currentIdx : 0;
    }
  }

  function selectMarkdownView(mode: MarkdownViewMode) {
    isViewMenuOpen = false;
    if (onChangeMarkdownView) {
      onChangeMarkdownView(mode);
    } else if (onToggleView) {
      onToggleView();
    }
  }

  function handleViewKeydown(e: KeyboardEvent) {
    if (!isViewMenuOpen) return;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedViewIndex = (selectedViewIndex + 1) % MARKDOWN_VIEW_MODES.length;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedViewIndex = (selectedViewIndex - 1 + MARKDOWN_VIEW_MODES.length) % MARKDOWN_VIEW_MODES.length;
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const selected = MARKDOWN_VIEW_MODES[selectedViewIndex];
      if (selected) {
        selectMarkdownView(selected.id);
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      isViewMenuOpen = false;
    }
  }

  let filteredTabs = $derived(
    tabs.filter((tab) => {
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase().trim();
      return (
        (tab.title && tab.title.toLowerCase().includes(q)) ||
        (tab.path && tab.path.toLowerCase().includes(q))
      );
    })
  );

  $effect(() => {
    if (filteredTabs.length > 0) {
      if (selectedIndex >= filteredTabs.length || selectedIndex < 0) {
        selectedIndex = 0;
      }
    } else {
      selectedIndex = 0;
    }
  });

  function triggerAction(actionId: string) {
    if (onAction) onAction(actionId);
  }

  function checkScroll() {
    if (!tabsScrollRef) return;
    canScrollLeft = tabsScrollRef.scrollLeft > 2;
    canScrollRight =
      tabsScrollRef.scrollLeft <
      tabsScrollRef.scrollWidth - tabsScrollRef.clientWidth - 2;
  }

  function scrollLeft() {
    if (tabsScrollRef) {
      tabsScrollRef.scrollBy({ left: -200, behavior: 'smooth' });
    }
  }

  function scrollRight() {
    if (tabsScrollRef) {
      tabsScrollRef.scrollBy({ left: 200, behavior: 'smooth' });
    }
  }

  function handleWheel(e: WheelEvent) {
    if (!tabsScrollRef) return;
    if (e.deltaY !== 0 && e.deltaX === 0) {
      e.preventDefault();
      tabsScrollRef.scrollLeft += e.deltaY;
      checkScroll();
    }
  }

  function getTabsPageSize(): number {
    if (tabsListRef) {
      const firstItem = tabsListRef.querySelector('.dropdown-tab-item') as HTMLElement | null;
      if (firstItem && firstItem.offsetHeight > 0) {
        return Math.max(1, Math.floor(tabsListRef.clientHeight / firstItem.offsetHeight));
      }
    }
    return 6;
  }

  function scrollSelectedIntoView(blockMode: ScrollLogicalPosition = 'nearest') {
    if (!tabsListRef) return;
    const selectedEl = tabsListRef.querySelector('.dropdown-tab-item.selected') as HTMLElement | null;
    if (selectedEl) {
      selectedEl.scrollIntoView({ block: blockMode });
    }
  }

  function selectTabAndClose(path: string) {
    if (onSelectTab) onSelectTab(path);
    isTabsMenuOpen = false;
  }

  function handleTabAuxClick(e: MouseEvent, path: string) {
    if (e.button === 1) {
      e.preventDefault();
      e.stopPropagation();
      if (onCloseTab) onCloseTab(path);
    }
  }

  function handleTabMouseDown(e: MouseEvent) {
    if (e.button === 1) {
      e.preventDefault();
    }
  }

  function openTabsMenu() {
    if (!tabs || tabs.length === 0) return;
    isTabsMenuOpen = true;
    searchQuery = '';
    const activeIdx = tabs.findIndex((t) => t.path === activeTabPath);
    selectedIndex = activeIdx >= 0 ? activeIdx : 0;
    tick().then(() => {
      searchInputRef?.focus();
      scrollSelectedIntoView();
    });
  }

  function toggleTabsMenu() {
    isViewMenuOpen = false;
    if (isTabsMenuOpen) {
      isTabsMenuOpen = false;
    } else {
      openTabsMenu();
    }
  }

  function handleSearchKeyDown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (filteredTabs.length > 0) {
        selectedIndex = (selectedIndex + 1) % filteredTabs.length;
        tick().then(() => scrollSelectedIntoView('nearest'));
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (filteredTabs.length > 0) {
        selectedIndex = (selectedIndex - 1 + filteredTabs.length) % filteredTabs.length;
        tick().then(() => scrollSelectedIntoView('nearest'));
      }
    } else if (e.key === 'Home') {
      e.preventDefault();
      if (filteredTabs.length > 0) {
        selectedIndex = 0;
        tick().then(() => {
          if (tabsListRef) tabsListRef.scrollTop = 0;
          scrollSelectedIntoView('start');
        });
      }
    } else if (e.key === 'End') {
      e.preventDefault();
      if (filteredTabs.length > 0) {
        selectedIndex = filteredTabs.length - 1;
        tick().then(() => {
          if (tabsListRef) tabsListRef.scrollTop = tabsListRef.scrollHeight;
          scrollSelectedIntoView('end');
        });
      }
    } else if (e.key === 'PageDown') {
      e.preventDefault();
      if (filteredTabs.length > 0) {
        const pageSize = getTabsPageSize();
        selectedIndex = Math.min(filteredTabs.length - 1, selectedIndex + pageSize);
        tick().then(() => scrollSelectedIntoView('nearest'));
      }
    } else if (e.key === 'PageUp') {
      e.preventDefault();
      if (filteredTabs.length > 0) {
        const pageSize = getTabsPageSize();
        selectedIndex = Math.max(0, selectedIndex - pageSize);
        tick().then(() => scrollSelectedIntoView('nearest'));
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (filteredTabs.length > 0 && filteredTabs[selectedIndex]) {
        selectTabAndClose(filteredTabs[selectedIndex].path);
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      isTabsMenuOpen = false;
    }
  }

  function handleCloseAll() {
    isTabsMenuOpen = false;
    if (onCloseAllTabs) {
      onCloseAllTabs();
    } else if (onCloseTab && tabs) {
      for (const t of [...tabs]) {
        onCloseTab(t.path);
      }
    }
  }

  function handleClickOutside(e: MouseEvent) {
    if (
      isTabsMenuOpen &&
      menuContainerRef &&
      !menuContainerRef.contains(e.target as Node)
    ) {
      isTabsMenuOpen = false;
    }
    if (
      isViewMenuOpen &&
      viewContainerRef &&
      !viewContainerRef.contains(e.target as Node)
    ) {
      isViewMenuOpen = false;
    }
  }

  function handleGlobalKeyDown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === 'A' || e.key === 'a')) {
      e.preventDefault();
      e.stopPropagation();
      toggleTabsMenu();
    } else if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && (e.key === 't' || e.key === 'T')) {
      e.preventDefault();
      e.stopPropagation();
      if (onNewTab) onNewTab();
    } else if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && (e.key === 'n' || e.key === 'N')) {
      e.preventDefault();
      e.stopPropagation();
      if (onNewFile) {
        onNewFile();
      } else if (onAction) {
        onAction('new-file');
      }
    } else if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && (e.key === 'w' || e.key === 'W')) {
      e.preventDefault();
      e.stopPropagation();
      const targetPath = activeTabPath || (tabs && tabs.length > 0 ? tabs[tabs.length - 1].path : '');
      if (targetPath && onCloseTab) {
        onCloseTab(targetPath);
      }
    } else if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && (e.key === 'o' || e.key === 'O')) {
      e.preventDefault();
      e.stopPropagation();
      if (onOpenQuickOpen) {
        onOpenQuickOpen();
      } else if (onAction) {
        onAction('quick-open');
      }
    }
  }

  $effect(() => {
    const currentPath = activeTabPath;
    const currentTabs = tabs;
    tick().then(() => {
      checkScroll();
      if (tabsScrollRef && currentPath) {
        const tabsList = tabsScrollRef.querySelectorAll('.tab-title-container');
        for (const el of tabsList) {
          if ((el as HTMLElement).dataset.path === currentPath) {
            (el as HTMLElement).scrollIntoView({
              behavior: 'smooth',
              block: 'nearest',
              inline: 'nearest',
            });
            break;
          }
        }
      }
    });
  });

  onMount(() => {
    window.addEventListener('click', handleClickOutside);
    window.addEventListener('keydown', handleGlobalKeyDown);
    window.addEventListener('resize', checkScroll);

    let resizeObserver: ResizeObserver | null = null;
    if (tabsScrollRef && typeof ResizeObserver !== 'undefined') {
      resizeObserver = new ResizeObserver(() => {
        checkScroll();
      });
      resizeObserver.observe(tabsScrollRef);
    }

    checkScroll();

    return () => {
      window.removeEventListener('click', handleClickOutside);
      window.removeEventListener('keydown', handleGlobalKeyDown);
      window.removeEventListener('resize', checkScroll);
      if (resizeObserver) {
        resizeObserver.disconnect();
      }
    };
  });

  async function handleWindowControl(action: 'minimize' | 'maximize' | 'close') {
    if (!isTauriEnvironment()) return;
    const appWindow = getCurrentWindow();
    if (action === 'minimize') {
      await appWindow.minimize();
    } else if (action === 'maximize') {
      await appWindow.toggleMaximize();
    } else if (action === 'close') {
      await appWindow.close();
    }
  }

  export { openTabsMenu, toggleTabsMenu };
</script>

<header class="editor-header" data-tauri-drag-region>
  <!-- BOTONES DE NAVEGACIÓN ATRÁS / ADELANTE -->
  <div class="nav-buttons">
    <button type="button" class="icon-btn" onclick={() => triggerAction('nav-back')} title="Navegar atrás">
      <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="15 18 9 12 15 6"/>
      </svg>
    </button>
    <button type="button" class="icon-btn" onclick={() => triggerAction('nav-forward')} title="Navegar adelante">
      <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="9 18 15 12 9 6"/>
      </svg>
    </button>
  </div>

  <!-- ÁREA CENTRAL DE PESTAÑAS: OCUPA SOLO EL ESPACIO DISPONIBLE Y SOPORTA SCROLL CON FLECHAS -->
  <div class="tabs-area-wrapper">
    {#if canScrollLeft}
      <button
        type="button"
        class="tab-scroll-btn left"
        onclick={scrollLeft}
        title="Desplazar pestañas a la izquierda"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="15 18 9 12 15 6"/>
        </svg>
      </button>
    {/if}

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="tabs-scroll-container"
      bind:this={tabsScrollRef}
      onscroll={checkScroll}
      onwheel={handleWheel}
    >
      {#if tabs && tabs.length > 0}
        {#each tabs as tab (tab.path)}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="tab-title-container"
            class:active={tab.path === activeTabPath}
            class:is-dirty={tab.isDirty}
            data-path={tab.path}
            title={tab.abs_path || tab.path}
            onclick={() => { if (onSelectTab) onSelectTab(tab.path); }}
            onauxclick={(e) => handleTabAuxClick(e, tab.path)}
            onmousedown={handleTabMouseDown}
          >
            <svg class="file-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
              <polyline points="14 2 14 8 20 8"/>
            </svg>
            <span class="tab-title">{tab.title}{tab.isDirty ? ' *' : ''}</span>
            <button
              type="button"
              class="close-tab-btn"
              onclick={(e) => {
                e.stopPropagation();
                if (onCloseTab) onCloseTab(tab.path);
              }}
              title="Cerrar pestaña (Ctrl+W o click central)"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18"/>
                <line x1="6" y1="6" x2="18" y2="18"/>
              </svg>
            </button>
          </div>
        {/each}
      {:else if title}
        <div
          class="tab-title-container active"
          onauxclick={(e) => handleTabAuxClick(e, '')}
          onmousedown={handleTabMouseDown}
        >
          <svg class="file-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
            <polyline points="14 2 14 8 20 8"/>
          </svg>
          <span class="tab-title">{title}</span>
          <button
            type="button"
            class="close-tab-btn"
            onclick={() => { if (onCloseTab) onCloseTab(''); }}
            title="Cerrar pestaña (Ctrl+W o click central)"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>
      {/if}

      <!-- BOTÓN NUEVA PESTAÑA: SIEMPRE PEGADO A LA IZQUIERDA, JUNTO AL ÚLTIMO TAB -->
      <button
        type="button"
        class="new-tab-btn"
        onclick={() => { if (onNewTab) onNewTab(); }}
        title="Nueva pestaña (Ctrl+T)"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="12" y1="5" x2="12" y2="19"/>
          <line x1="5" y1="12" x2="19" y2="12"/>
        </svg>
      </button>
    </div>

    {#if canScrollRight}
      <button
        type="button"
        class="tab-scroll-btn right"
        onclick={scrollRight}
        title="Desplazar pestañas a la derecha"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="9 18 15 12 9 6"/>
        </svg>
      </button>
      <!-- BOTÓN NUEVA PESTAÑA: SIEMPRE PEGADO A LA IZQUIERDA, JUNTO AL ÚLTIMO TAB -->
      <button
        type="button"
        class="new-tab-btn"
        onclick={() => { if (onNewTab) onNewTab(); }}
        title="Nueva pestaña (Ctrl+T)"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="12" y1="5" x2="12" y2="19"/>
          <line x1="5" y1="12" x2="19" y2="12"/>
        </svg>
      </button>
    {/if}
  </div>

  <!-- SECCIÓN DERECHA -->
  <div class="right-section">
    <!-- MENÚ DESPLEGABLE PARA LISTAR Y BUSCAR TODAS LAS PESTAÑAS Y BOTÓN DE CERRAR TODAS -->
    <!-- AL COSTADO IZQUIERDO DE LOS BOTONES DE GRABAR Y EDITAR -->
    {#if tabs && tabs.length > 0}
      <div class="tabs-actions-group" bind:this={menuContainerRef}>
        <button
          type="button"
          class="icon-btn tab-dropdown-trigger"
          class:active={isTabsMenuOpen}
          onclick={(e) => {
            e.stopPropagation();
            toggleTabsMenu();
          }}
          title="Listar y buscar pestañas abiertas (Ctrl+Shift+A)"
        >
          <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="6 9 12 15 18 9"/>
          </svg>
        </button>

        {#if isTabsMenuOpen}
          <div class="tabs-dropdown-menu">
            <div class="dropdown-header">
              <span>Pestañas ({filteredTabs.length}{filteredTabs.length !== tabs.length ? `/${tabs.length}` : ''})</span>
              <button type="button" class="dropdown-close-all-btn" onclick={handleCloseAll}>
                Cerrar todas
              </button>
            </div>

            <!-- CAJA DE TEXTO PARA BUSCAR RÁPIDAMENTE ENTRE LAS PESTAÑAS ABIERTAS -->
            <div class="dropdown-search-container">
              <svg class="dropdown-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="11" cy="11" r="8"/>
                <line x1="21" y1="21" x2="16.65" y2="16.65"/>
              </svg>
              <input
                type="text"
                class="dropdown-search-input"
                placeholder="Buscar pestaña... (Enter para abrir)"
                bind:this={searchInputRef}
                bind:value={searchQuery}
                onkeydown={handleSearchKeyDown}
              />
              {#if searchQuery}
                <button
                  type="button"
                  class="dropdown-search-clear"
                  onclick={() => {
                    searchQuery = '';
                    searchInputRef?.focus();
                  }}
                  title="Limpiar búsqueda"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <line x1="18" y1="6" x2="6" y2="18"/>
                    <line x1="6" y1="6" x2="18" y2="18"/>
                  </svg>
                </button>
              {/if}
            </div>

            <div class="dropdown-tabs-list" bind:this={tabsListRef}>
              {#if filteredTabs.length > 0}
                {#each filteredTabs as tab, index (tab.path)}
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    class="dropdown-tab-item"
                    class:active={tab.path === activeTabPath}
                    class:selected={index === selectedIndex}
                    title={tab.abs_path || tab.path}
                    onmouseenter={() => { selectedIndex = index; }}
                    onclick={() => selectTabAndClose(tab.path)}
                    onauxclick={(e) => handleTabAuxClick(e, tab.path)}
                    onmousedown={handleTabMouseDown}
                  >
                    <svg class="dropdown-file-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                      <polyline points="14 2 14 8 20 8"/>
                    </svg>
                    <span class="dropdown-tab-title">{tab.title}{tab.isDirty ? ' *' : ''}</span>
                    <button
                      type="button"
                      class="dropdown-tab-close"
                      onclick={(e) => {
                        e.stopPropagation();
                        if (onCloseTab) onCloseTab(tab.path);
                      }}
                      title="Cerrar pestaña (Ctrl+W o click central)"
                    >
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <line x1="18" y1="6" x2="6" y2="18"/>
                        <line x1="6" y1="6" x2="18" y2="18"/>
                      </svg>
                    </button>
                  </div>
                {/each}
              {:else}
                <div class="dropdown-no-results">
                  No se encontraron pestañas abiertas
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {/if}

    {#if showSaveButton}
      <button
        type="button"
        class="save-btn"
        onclick={() => {
          if (onSave) onSave();
        }}
        title="Guardar / Grabar cambios (Ctrl+S)"
      >
        <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/>
          <polyline points="17 21 17 13 7 13 7 21"/>
          <polyline points="7 3 7 8 15 8"/>
        </svg>
        <span>Grabar</span>
      </button>
    {/if}

    <!-- SELECTOR DE MODO DE VISTA: LISTA DESPLEGABLE EN CABECERA SI ES MARKDOWN -->
    {#if isMarkdownFile}
      <div class="md-view-dropdown-container" bind:this={viewContainerRef}>
        <button
          type="button"
          class="view-toggle-btn md-mode-btn"
          class:live-mode={markdownViewMode === 'live'}
          class:source-mode={markdownViewMode === 'source'}
          class:reading-mode={markdownViewMode === 'reading'}
          class:active={isViewMenuOpen}
          onclick={toggleViewMenu}
          title="Cambiar vista de Markdown (En vivo / Fuente / Lectura)"
        >
          {#if markdownViewMode === 'live'}
            <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
            </svg>
            <span>En vivo</span>
          {:else if markdownViewMode === 'source'}
            <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="16 18 22 12 16 6"/>
              <polyline points="8 6 2 12 8 18"/>
            </svg>
            <span>Fuente</span>
          {:else}
            <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/>
              <path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/>
            </svg>
            <span>Lectura</span>
          {/if}
          <svg class="chevron-icon" class:open={isViewMenuOpen} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="6 9 12 15 18 9"/>
          </svg>
        </button>

        {#if isViewMenuOpen}
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <div class="view-dropdown-menu" role="dialog" aria-modal="true" tabindex="-1" onkeydown={handleViewKeydown}>
            <div class="view-dropdown-header">
              <span>Vista Markdown</span>
              <span class="header-badge">{markdownViewMode === 'live' ? 'En vivo' : markdownViewMode === 'source' ? 'Fuente' : 'Lectura'}</span>
            </div>

            <div class="view-options-list">
              {#each MARKDOWN_VIEW_MODES as item, idx}
                {@const isCurrent = markdownViewMode === item.id}
                <button
                  type="button"
                  class="view-dropdown-item"
                  class:selected={idx === selectedViewIndex}
                  class:current={isCurrent}
                  class:mode-live={item.id === 'live'}
                  class:mode-source={item.id === 'source'}
                  class:mode-reading={item.id === 'reading'}
                  onmouseenter={() => { selectedViewIndex = idx; }}
                  onclick={() => selectMarkdownView(item.id)}
                >
                  <div class="item-icon-box">
                    {#if item.id === 'live'}
                      <svg class="item-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
                      </svg>
                    {:else if item.id === 'source'}
                      <svg class="item-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="16 18 22 12 16 6"/>
                        <polyline points="8 6 2 12 8 18"/>
                      </svg>
                    {:else}
                      <svg class="item-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/>
                        <path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/>
                      </svg>
                    {/if}
                  </div>

                  <div class="item-text">
                    <div class="item-title-row">
                      <span class="item-label">{item.label}</span>
                      <span class="item-badge">{item.badge}</span>
                    </div>
                    <span class="item-desc">{item.description}</span>
                  </div>

                  {#if isCurrent}
                    <svg class="check-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                      <polyline points="20 6 9 17 4 12"/>
                    </svg>
                  {/if}
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {:else}
      <button
        type="button"
        class="view-toggle-btn"
        onclick={() => {
          isEditing = !isEditing;
          if (onToggleView) onToggleView();
        }}
        title={isEditing ? 'Cambiar a modo Lectura' : 'Cambiar a modo Edición'}
      >
        <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          {#if isEditing}
            <path d="M12 20h9"/>
            <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/>
          {:else}
            <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
            <circle cx="12" cy="12" r="3"/>
          {/if}
        </svg>
        <span>{isEditing ? 'Edición' : 'Lectura'}</span>
      </button>
    {/if}

    <button type="button" class="icon-btn" onclick={() => { if (onSplitView) onSplitView(); }} title="Dividir panel verticalmente">
      <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
        <line x1="12" y1="3" x2="12" y2="21"/>
      </svg>
    </button>

    <button type="button" class="icon-btn" onclick={() => triggerAction('more-options')} title="Más opciones del archivo">
      <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="1"/>
        <circle cx="19" cy="12" r="1"/>
        <circle cx="5" cy="12" r="1"/>
      </svg>
    </button>

    <!-- CONTROLES DE VENTANA PERSONALIZADOS -->
    <div class="window-controls">
      <button type="button" class="win-btn minimize" onclick={() => handleWindowControl('minimize')} title="Minimizar">
        <svg class="win-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="5" y1="12" x2="19" y2="12"/>
        </svg>
      </button>
      <button type="button" class="win-btn maximize" onclick={() => handleWindowControl('maximize')} title="Maximizar / Restaurar">
        <svg class="win-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="5" y="5" width="14" height="14" rx="1"/>
        </svg>
      </button>
      <button type="button" class="win-btn close" onclick={() => handleWindowControl('close')} title="Cerrar">
        <svg class="win-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="18" y1="6" x2="6" y2="18"/>
          <line x1="6" y1="6" x2="18" y2="18"/>
        </svg>
      </button>
    </div>
  </div>
</header>

<style>
  .editor-header {
    height: 42px;
    flex-shrink: 0;
    background-color: var(--bg-primary, #ffffff);
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    display: flex;
    align-items: center;
    padding: 0 10px;
    user-select: none;
    gap: 8px;
    position: relative;
  }

  .nav-buttons {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }

  .tabs-area-wrapper {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    height: 100%;
    overflow: hidden;
    position: relative;
  }

  .tabs-scroll-container {
    display: flex;
    align-items: center;
    gap: 4px;
    overflow-x: auto;
    overflow-y: hidden;
    width: 100%;
    height: 100%;
    scrollbar-width: none;
    -ms-overflow-style: none;
    scroll-behavior: smooth;
    padding: 2px 0;
  }

  .tabs-scroll-container::-webkit-scrollbar {
    display: none;
  }

  .tab-scroll-btn {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 26px;
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    color: var(--text-secondary, #656d76);
    border-radius: 4px;
    cursor: pointer;
    z-index: 2;
    transition: all 0.15s ease;
    padding: 0;
  }

  .tab-scroll-btn:hover {
    background: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
    border-color: var(--accent, #0969da);
  }

  .tab-scroll-btn.left {
    margin-right: 4px;
  }

  .tab-scroll-btn.right {
    margin-left: 4px;
  }

  .tab-scroll-btn svg {
    width: 12px;
    height: 12px;
  }

  .new-tab-btn {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-secondary, #656d76);
    border-radius: 6px;
    cursor: pointer;
    z-index: 2;
    transition: all 0.15s ease;
    padding: 0;
    margin-left: 0;
  }

  .new-tab-btn:hover {
    background: var(--bg-secondary, #f6f8fa);
    border-color: var(--border-primary, #d0d7de);
    color: var(--text-primary, #1f2328);
  }

  .new-tab-btn svg {
    width: 14px;
    height: 14px;
  }

  .tab-title-container {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    background: var(--bg-secondary, #f6f8fa);
    border-radius: 6px;
    border: 1px solid var(--border-primary, #d0d7de);
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
    opacity: 0.75;
    flex-shrink: 0;
  }

  .tab-title-container:hover {
    opacity: 1;
    background: var(--bg-primary, #ffffff);
  }

  .tab-title-container.active {
    opacity: 1;
    background: var(--bg-primary, #ffffff);
    border-color: var(--accent, #0969da);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
  }

  .file-icon {
    width: 14px;
    height: 14px;
    color: var(--accent, #0969da);
    flex-shrink: 0;
  }

  .tab-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary, #1f2328);
  }

  .close-tab-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    background: transparent;
    border-radius: 4px;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    margin-left: 4px;
    transition: all 0.15s ease;
    flex-shrink: 0;
  }

  .close-tab-btn:hover {
    background: rgba(0, 0, 0, 0.1);
    color: var(--text-primary, #1f2328);
  }

  .close-tab-btn svg {
    width: 12px;
    height: 12px;
  }

  .right-section {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .tabs-actions-group {
    position: relative;
    display: flex;
    align-items: center;
    gap: 2px;
    padding-right: 6px;
    margin-right: 4px;
    border-right: 1px solid var(--border-primary, #d0d7de);
  }

  .tab-dropdown-trigger.active {
    background-color: var(--bg-secondary, #f6f8fa);
    color: var(--accent, #0969da);
  }

  .tabs-dropdown-menu {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    width: 300px;
    max-height: 420px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.14);
    z-index: 1000;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: dropdown-fade 0.15s ease;
  }

  @keyframes dropdown-fade {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .dropdown-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    background: var(--bg-secondary, #f6f8fa);
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary, #656d76);
  }

  .dropdown-close-all-btn {
    background: transparent;
    border: none;
    color: #cf222e;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    transition: all 0.15s ease;
  }

  .dropdown-close-all-btn:hover {
    background: rgba(207, 34, 46, 0.1);
  }

  .dropdown-search-container {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    background: var(--bg-primary, #ffffff);
  }

  .dropdown-search-icon {
    width: 14px;
    height: 14px;
    color: var(--text-secondary, #656d76);
    flex-shrink: 0;
  }

  .dropdown-search-input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    font-size: 12px;
    color: var(--text-primary, #1f2328);
    outline: none;
    padding: 0;
  }

  .dropdown-search-input::placeholder {
    color: var(--text-secondary, #656d76);
    font-size: 11px;
  }

  .dropdown-search-clear {
    border: none;
    background: transparent;
    padding: 0;
    cursor: pointer;
    color: var(--text-secondary, #656d76);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 4px;
    transition: all 0.1s ease;
  }

  .dropdown-search-clear:hover {
    background: rgba(0, 0, 0, 0.08);
    color: var(--text-primary, #1f2328);
  }

  .dropdown-search-clear svg {
    width: 12px;
    height: 12px;
  }

  .dropdown-tabs-list {
    overflow-y: auto;
    max-height: 300px;
    padding: 4px 0;
  }

  .dropdown-tab-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    cursor: pointer;
    transition: background 0.1s ease;
    font-size: 13px;
    color: var(--text-primary, #1f2328);
    border-left: 2px solid transparent;
  }

  .dropdown-tab-item:hover,
  .dropdown-tab-item.selected {
    background: rgba(9, 105, 218, 0.08);
  }

  .dropdown-tab-item.selected {
    border-left-color: var(--accent, #0969da);
  }

  .dropdown-tab-item.active {
    color: var(--accent, #0969da);
    font-weight: 500;
  }

  .dropdown-file-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    color: var(--accent, #0969da);
  }

  .dropdown-tab-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dropdown-tab-close {
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    border-radius: 4px;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    opacity: 0.6;
    transition: all 0.15s ease;
  }

  .dropdown-tab-close:hover {
    opacity: 1;
    background: rgba(0, 0, 0, 0.08);
    color: var(--text-primary, #1f2328);
  }

  .dropdown-tab-close svg {
    width: 12px;
    height: 12px;
  }

  .dropdown-no-results {
    padding: 16px 12px;
    text-align: center;
    color: var(--text-secondary, #656d76);
    font-size: 12px;
    font-style: italic;
  }

  .icon-btn {
    width: 30px;
    height: 30px;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-secondary, #656d76);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .icon-btn:hover {
    color: var(--text-primary, #1f2328);
    background-color: rgba(0, 0, 0, 0.05);
  }

  .view-toggle-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    height: 30px;
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    color: var(--text-primary, #1f2328);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .view-toggle-btn:hover {
    background: rgba(0, 0, 0, 0.05);
    color: var(--accent, #0969da);
    border-color: var(--accent, #0969da);
  }

  /* Selector desplegable de Modo de Vista Markdown en la cabecera */
  .md-view-dropdown-container {
    position: relative;
    display: flex;
    align-items: center;
  }

  .view-toggle-btn.md-mode-btn {
    font-weight: 500;
    gap: 6px;
  }

  .view-toggle-btn.live-mode {
    color: #1a7f37;
    background: rgba(46, 160, 67, 0.08);
    border-color: rgba(46, 160, 67, 0.3);
    font-weight: 600;
  }

  .view-toggle-btn.live-mode:hover,
  .view-toggle-btn.live-mode.active {
    background: rgba(46, 160, 67, 0.16);
    border-color: #1a7f37;
  }

  .view-toggle-btn.source-mode {
    color: var(--accent, #0969da);
    background: var(--accent-bg, rgba(9, 105, 218, 0.08));
    border-color: rgba(9, 105, 218, 0.3);
    font-weight: 600;
  }

  .view-toggle-btn.source-mode:hover,
  .view-toggle-btn.source-mode.active {
    background: var(--accent-bg, rgba(9, 105, 218, 0.16));
    border-color: var(--accent, #0969da);
  }

  .view-toggle-btn.reading-mode {
    color: #8250df;
    background: rgba(130, 80, 223, 0.08);
    border-color: rgba(130, 80, 223, 0.3);
    font-weight: 600;
  }

  .view-toggle-btn.reading-mode:hover,
  .view-toggle-btn.reading-mode.active {
    background: rgba(130, 80, 223, 0.16);
    border-color: #8250df;
  }

  .chevron-icon {
    width: 10px;
    height: 10px;
    opacity: 0.7;
    transition: transform 0.15s ease;
  }

  .chevron-icon.open {
    transform: rotate(180deg);
  }

  .view-dropdown-menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    width: 290px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.14);
    z-index: 1000;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: dropdown-fade 0.15s ease;
    outline: none;
  }

  .view-dropdown-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--bg-secondary, #f6f8fa);
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary, #656d76);
  }

  .header-badge {
    font-size: 10px;
    color: var(--text-secondary, #656d76);
  }

  .view-options-list {
    padding: 4px 0;
    display: flex;
    flex-direction: column;
  }

  .view-dropdown-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    transition: background 0.1s ease;
    outline: none;
  }

  .view-dropdown-item:hover,
  .view-dropdown-item.selected {
    background: rgba(0, 0, 0, 0.04);
  }

  .view-dropdown-item.current {
    background: rgba(9, 105, 218, 0.05);
  }

  .view-dropdown-item.current.selected {
    background: rgba(9, 105, 218, 0.1);
  }

  .item-icon-box {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-secondary, #656d76);
  }

  .view-dropdown-item.mode-live .item-icon-box {
    color: #1a7f37;
    background: rgba(46, 160, 67, 0.1);
  }

  .view-dropdown-item.mode-source .item-icon-box {
    color: var(--accent, #0969da);
    background: rgba(9, 105, 218, 0.1);
  }

  .view-dropdown-item.mode-reading .item-icon-box {
    color: #8250df;
    background: rgba(130, 80, 223, 0.1);
  }

  .item-icon {
    width: 14px;
    height: 14px;
  }

  .item-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  .item-title-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .item-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
  }

  .item-badge {
    font-size: 9px;
    padding: 1px 5px;
    border-radius: 4px;
    font-weight: 500;
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-secondary, #656d76);
    border: 1px solid var(--border-primary, #d0d7de);
  }

  .view-dropdown-item.current.mode-live .item-label {
    color: #1a7f37;
  }

  .view-dropdown-item.current.mode-source .item-label {
    color: var(--accent, #0969da);
  }

  .view-dropdown-item.current.mode-reading .item-label {
    color: #8250df;
  }

  .item-desc {
    font-size: 10.5px;
    color: var(--text-secondary, #656d76);
    line-height: 1.3;
  }

  .check-icon {
    width: 14px;
    height: 14px;
    color: var(--accent, #0969da);
    flex-shrink: 0;
  }

  .save-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    background: var(--accent-bg, rgba(9, 105, 218, 0.1));
    border: 1px solid var(--accent-border, rgba(9, 105, 218, 0.3));
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--accent, #0969da);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .save-btn:hover {
    background: var(--accent, #0969da);
    color: #ffffff;
  }

  .icon {
    width: 16px;
    height: 16px;
  }

  .window-controls {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: 6px;
    padding-left: 6px;
    border-left: 1px solid var(--border-primary, #d0d7de);
  }

  .win-btn {
    width: 28px;
    height: 28px;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-secondary, #656d76);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .win-btn:hover {
    color: var(--text-primary, #1f2328);
    background-color: rgba(0, 0, 0, 0.05);
  }

  .win-btn.close:hover {
    color: #ffffff;
    background-color: #e5534b;
  }

  .win-icon {
    width: 14px;
    height: 14px;
  }
</style>
