<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { isTauriEnvironment } from '@shared/repositories';
  import type { TabItem, MarkdownViewMode } from '@entities/vault-item';
  import { Popover, ContextMenu } from 'bits-ui';
  import { FileIcon } from '@entities/file-type';
  import { WindowControls } from '@shared/ui/window-controls';
  import {
    ChevronLeft,
    ChevronRight,
    ChevronDown,
    X,
    Plus,
    Search,
    Save,
    PenLine,
    Eye,
    Columns2,
    MoreHorizontal,
    Minus,
    Square,
    GitCompare,
    GitCompareArrows,
    ExternalLink,
    FolderSearch,
    Copy,
    History,
    Lock
  } from 'lucide-svelte';

  interface Props {
    title?: string;
    tabs?: TabItem[];
    activeTabPath?: string | null;
    isEditing?: boolean;
    isMarkdownFile?: boolean;
    markdownViewMode?: MarkdownViewMode;
    showSaveButton?: boolean;
    showViewToggle?: boolean;
    canGoBack?: boolean;
    canGoForward?: boolean;
    showWindowControls?: boolean;
    isSplitActive?: boolean;
    onNavigateBack?: () => void;
    onNavigateForward?: () => void;
    onSelectTab?: (path: string) => void;
    onCloseTab?: (path: string) => void;
    onCloseAllTabs?: () => void;
    onCloseOtherTabs?: (path: string) => void;
    onCloseTabsToRight?: (path: string) => void;
    onOpenFileWithDefaultApp?: (path: string) => void;
    onShowItemInFolder?: (path: string) => void;
    onOpenInNewWindow?: (path: string, moveTab?: boolean) => void;
    onReorderTab?: (draggedPath: string, targetIndex: number) => void;
    onReceiveTabFromOtherWindow?: (path: string, targetIndex: number, sourceWindow: string) => void;
    onNewTab?: () => void;
    onNewFile?: () => void;
    onToggleView?: () => void;
    onChangeMarkdownView?: (mode: MarkdownViewMode) => void;
    onSplitView?: (path?: string) => void;
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
    showViewToggle = false,
    canGoBack = false,
    canGoForward = false,
    showWindowControls = true,
    isSplitActive = false,
    onNavigateBack,
    onNavigateForward,
    onSelectTab,
    onCloseTab,
    onCloseAllTabs,
    onCloseOtherTabs,
    onCloseTabsToRight,
    onOpenFileWithDefaultApp,
    onShowItemInFolder,
    onOpenInNewWindow,
    onReorderTab,
    onReceiveTabFromOtherWindow,
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

  function isSyntheticTab(p?: string | null): boolean {
    if (!p) return false;
    return (
      p.startsWith('empty:') ||
      p.startsWith('diff:') ||
      p.startsWith('diff-staged:') ||
      p.startsWith('diff-rev:') ||
      p.startsWith('rev:') ||
      p.startsWith('synapse://')
    );
  }

  let tabsScrollRef = $state<HTMLDivElement | null>(null);
  let menuContainerRef = $state<HTMLDivElement | null>(null);
  let searchInputRef = $state<HTMLInputElement | null>(null);
  let tabsListRef = $state<HTMLDivElement | null>(null);

  let canScrollLeft = $state(false);
  let canScrollRight = $state(false);
  let isTabsMenuOpen = $state(false);

  let searchQuery = $state('');
  let selectedIndex = $state(0);

  // Estado y DND de pestañas
  interface SynapseTabDragData {
    path: string;
    title: string;
    windowLabel: string;
  }

  let myWindowLabel = $state('main');
  let draggingTabPath = $state<string | null>(null);
  let dropIndicator = $state<{ path: string; position: 'left' | 'right' } | null>(null);
  let isContainerDropActive = $state(false);

  function parseTabDragData(e: DragEvent): SynapseTabDragData | null {
    try {
      const rawCustom = e.dataTransfer?.getData('application/x-synapse-tab');
      if (rawCustom) return JSON.parse(rawCustom);
      const rawText = e.dataTransfer?.getData('text/plain');
      if (rawText && rawText.startsWith('synapse-tab:')) {
        return JSON.parse(rawText.slice('synapse-tab:'.length));
      }
    } catch {}
    return null;
  }

  function handleTabDragStart(e: DragEvent, tab: TabItem) {
    if (isSyntheticTab(tab.path)) return;
    draggingTabPath = tab.path;
    const payload: SynapseTabDragData = {
      path: tab.path,
      title: tab.title,
      windowLabel: myWindowLabel,
    };
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('application/x-synapse-tab', JSON.stringify(payload));
      e.dataTransfer.setData('text/plain', `synapse-tab:${JSON.stringify(payload)}`);
    }
  }

  function handleTabDragOver(e: DragEvent, tab: TabItem, index: number) {
    const isSynapseTab = e.dataTransfer?.types.includes('application/x-synapse-tab') ||
      e.dataTransfer?.types.includes('text/plain');
    if (!isSynapseTab) return;

    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'move';
    }

    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const mouseX = e.clientX - rect.left;
    const position: 'left' | 'right' = mouseX < rect.width / 2 ? 'left' : 'right';

    if (dropIndicator?.path !== tab.path || dropIndicator?.position !== position) {
      dropIndicator = { path: tab.path, position };
    }
  }

  function handleTabDragLeave(e: DragEvent, tab: TabItem) {
    if (dropIndicator?.path === tab.path) {
      dropIndicator = null;
    }
  }

  function handleTabDrop(e: DragEvent, targetTab: TabItem, targetIndex: number) {
    e.preventDefault();
    e.stopPropagation();
    const data = parseTabDragData(e);
    const currentIndicator = dropIndicator;
    dropIndicator = null;
    draggingTabPath = null;
    isContainerDropActive = false;

    if (!data || !data.path) return;

    let finalTargetIndex = targetIndex;
    if (currentIndicator && currentIndicator.position === 'right') {
      finalTargetIndex = targetIndex + 1;
    }

    const isExistingTab = tabs.some((t) => t.path === data.path);
    if (data.windowLabel === myWindowLabel && isExistingTab) {
      if (onReorderTab) {
        onReorderTab(data.path, finalTargetIndex);
      }
    } else {
      if (onReceiveTabFromOtherWindow) {
        onReceiveTabFromOtherWindow(data.path, finalTargetIndex, data.windowLabel);
      }
    }
  }

  function handleTabDragEnd() {
    draggingTabPath = null;
    dropIndicator = null;
    isContainerDropActive = false;
  }

  function handleContainerDragOver(e: DragEvent) {
    const isSynapseTab = e.dataTransfer?.types.includes('application/x-synapse-tab') ||
      e.dataTransfer?.types.includes('text/plain');
    if (!isSynapseTab) return;
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'move';
    }
    isContainerDropActive = true;
  }

  function handleContainerDragLeave(e: DragEvent) {
    if (e.currentTarget === e.target) {
      isContainerDropActive = false;
    }
  }

  function handleContainerDrop(e: DragEvent) {
    e.preventDefault();
    const data = parseTabDragData(e);
    draggingTabPath = null;
    dropIndicator = null;
    isContainerDropActive = false;

    if (!data || !data.path) return;

    const targetIndex = tabs.length;
    const isExistingTab = tabs.some((t) => t.path === data.path);
    if (data.windowLabel === myWindowLabel && isExistingTab) {
      if (onReorderTab) {
        onReorderTab(data.path, targetIndex);
      }
    } else {
      if (onReceiveTabFromOtherWindow) {
        onReceiveTabFromOtherWindow(data.path, targetIndex, data.windowLabel);
      }
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

  function handleGlobalKeyDown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === 'A' || e.key === 'a')) {
      e.preventDefault();
      e.stopPropagation();
      toggleTabsMenu();
    }
  }

  $effect(() => {
    const currentPath = activeTabPath;
    const _currentTabs = tabs;
    tick().then(() => {
      checkScroll();
      if (tabsScrollRef && currentPath) {
        try {
          const activeTabEl = tabsScrollRef.querySelector<HTMLElement>(`.tab-title-container[data-path="${CSS.escape(currentPath)}"]`);
          if (activeTabEl) {
            const containerRect = tabsScrollRef.getBoundingClientRect();
            const tabRect = activeTabEl.getBoundingClientRect();
            if (tabRect.left < containerRect.left) {
              tabsScrollRef.scrollLeft -= (containerRect.left - tabRect.left + 12);
            } else if (tabRect.right > containerRect.right) {
              tabsScrollRef.scrollLeft += (tabRect.right - containerRect.right + 12);
            }
          }
        } catch {
          // Si el selector falla, no romper nada
        }
      }
    });
  });

  onMount(() => {
    if (isTauriEnvironment()) {
      try {
        myWindowLabel = getCurrentWebviewWindow().label;
      } catch {}
    }

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
      window.removeEventListener('keydown', handleGlobalKeyDown);
      window.removeEventListener('resize', checkScroll);
      if (resizeObserver) {
        resizeObserver.disconnect();
      }
    };
  });

  async function handleHeaderDblClick(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    if (!target) return;
    // No alternar si el doble clic ocurrió sobre un botón, input o pestaña
    if (target.closest('button, input, [data-tauri-drag-region="false"], .tab-title-container')) {
      return;
    }
    if (isTauriEnvironment()) {
      try {
        const appWindow = getCurrentWebviewWindow();
        await appWindow.toggleMaximize();
      } catch (err) {
        console.error('Error al alternar maximizar ventana:', err);
      }
    }
  }

  export { openTabsMenu, toggleTabsMenu };
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<header class="editor-header" data-tauri-drag-region ondblclick={handleHeaderDblClick}>
  <!-- BOTONES DE NAVEGACIÓN ATRÁS / ADELANTE -->
  <div class="nav-buttons" data-tauri-drag-region="false">
    <button
      type="button"
      class="icon-btn"
      data-tauri-drag-region="false"
      disabled={!canGoBack}
      onclick={() => {
        if (onNavigateBack) onNavigateBack();
        else triggerAction('nav-back');
      }}
      title={canGoBack ? "Navegar atrás entre pestañas (Alt+Flecha izquierda o Botón 3 del ratón)" : "Navegar atrás"}
    >
      <ChevronLeft size={16} class="icon" />
    </button>
    <button
      type="button"
      class="icon-btn"
      data-tauri-drag-region="false"
      disabled={!canGoForward}
      onclick={() => {
        if (onNavigateForward) onNavigateForward();
        else triggerAction('nav-forward');
      }}
      title={canGoForward ? "Navegar adelante entre pestañas (Alt+Flecha derecha o Botón 4 del ratón)" : "Navegar adelante"}
    >
      <ChevronRight size={16} class="icon" />
    </button>
  </div>

  <!-- ÁREA CENTRAL DE PESTAÑAS: OCUPA SOLO EL ESPACIO DISPONIBLE Y SOPORTA SCROLL CON FLECHAS -->
  <div class="tabs-area-wrapper" data-tauri-drag-region>
    {#if canScrollLeft}
      <button
        type="button"
        class="tab-scroll-btn left"
        data-tauri-drag-region="false"
        onclick={scrollLeft}
        title="Desplazar pestañas a la izquierda"
      >
        <ChevronLeft size={14} />
      </button>
    {/if}

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="tabs-scroll-container"
      class:drop-active={isContainerDropActive}
      data-tauri-drag-region
      bind:this={tabsScrollRef}
      onscroll={checkScroll}
      onwheel={handleWheel}
      ondragover={handleContainerDragOver}
      ondragleave={handleContainerDragLeave}
      ondrop={handleContainerDrop}
    >
      {#if tabs && tabs.length > 0}
        {#each tabs as tab, index (tab.path)}
          <ContextMenu.Root>
            <ContextMenu.Trigger>
              {#snippet child({ props })}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                  {...props}
                  class="tab-title-container"
                  class:active={tab.path === activeTabPath}
                  class:is-dirty={tab.isDirty}
                  class:is-dragging={draggingTabPath === tab.path}
                  class:drag-target-left={dropIndicator?.path === tab.path && dropIndicator?.position === 'left'}
                  class:drag-target-right={dropIndicator?.path === tab.path && dropIndicator?.position === 'right'}
                  data-path={tab.path}
                  data-tauri-drag-region="false"
                  draggable={!isSyntheticTab(tab.path)}
                  title={tab.abs_path || tab.path}
                  onclick={() => { if (onSelectTab) onSelectTab(tab.path); }}
                  onauxclick={(e) => handleTabAuxClick(e, tab.path)}
                  onmousedown={handleTabMouseDown}
                  ondragstart={(e) => handleTabDragStart(e, tab)}
                  ondragover={(e) => handleTabDragOver(e, tab, index)}
                  ondragleave={(e) => handleTabDragLeave(e, tab)}
                  ondrop={(e) => handleTabDrop(e, tab, index)}
                  ondragend={handleTabDragEnd}
                >
                  {#if tab.path.startsWith('diff-staged:')}
                    <GitCompareArrows size={14} class="file-icon" style="color: #22c55e;" />
                  {:else if tab.path.startsWith('diff-rev:')}
                    <GitCompare size={14} class="file-icon" style="color: #a855f7;" />
                  {:else if tab.path.startsWith('diff:')}
                    <GitCompare size={14} class="file-icon" style="color: #3b82f6;" />
                  {:else if tab.path.startsWith('rev:')}
                    <History size={14} class="file-icon" style="color: #0969da;" />
                  {:else}
                    <FileIcon path={tab.path} name={tab.title} kind={tab.kind} size={14} class="file-icon" />
                  {/if}
                  <span class="tab-title">{tab.title}{tab.isDirty ? ' *' : ''}</span>
                  {#if tab.isLocked}
                    <span class="tab-lock-badge" title="Archivo bloqueado (auto-guardado desactivado)">
                      <Lock size={11} class="tab-lock-icon" />
                    </span>
                  {/if}
                  <button
                    type="button"
                    class="close-tab-btn"
                    onclick={(e) => {
                      e.stopPropagation();
                      if (onCloseTab) onCloseTab(tab.path);
                    }}
                    title="Cerrar pestaña (Ctrl+W, Ctrl+F4 o click central)"
                  >
                    <X size={12} />
                  </button>
                </div>
              {/snippet}
            </ContextMenu.Trigger>
            <ContextMenu.Portal>
              <ContextMenu.Content class="tab-context-menu">
                {#if !isSyntheticTab(tab.path)}
                  <ContextMenu.Item
                    class="context-menu-item"
                    onSelect={() => {
                      if (onOpenInNewWindow) onOpenInNewWindow(tab.path, false);
                    }}
                  >
                    <ExternalLink size={14} class="context-menu-item-icon" />
                    <span>Abrir en nueva ventana</span>
                  </ContextMenu.Item>
                  <ContextMenu.Item
                    class="context-menu-item"
                    onSelect={() => {
                      if (onOpenInNewWindow) onOpenInNewWindow(tab.path, true);
                    }}
                  >
                    <ExternalLink size={14} class="context-menu-item-icon" />
                    <span>Mover a nueva ventana</span>
                  </ContextMenu.Item>
                  <ContextMenu.Separator class="context-menu-divider" />
                {/if}
                <ContextMenu.Item
                  class="context-menu-item"
                  onSelect={() => {
                    if (onCloseTab) onCloseTab(tab.path);
                  }}
                >
                  <X size={14} class="context-menu-item-icon" />
                  <span>Cerrar pestaña</span>
                </ContextMenu.Item>
                <ContextMenu.Item
                  class="context-menu-item"
                  onSelect={() => {
                    if (onCloseOtherTabs) onCloseOtherTabs(tab.path);
                  }}
                >
                  <span>Cerrar otras pestañas</span>
                </ContextMenu.Item>
                <ContextMenu.Item
                  class="context-menu-item"
                  onSelect={() => {
                    if (onCloseTabsToRight) onCloseTabsToRight(tab.path);
                  }}
                >
                  <span>Cerrar pestañas a la derecha</span>
                </ContextMenu.Item>
                {#if !isSyntheticTab(tab.path)}
                  <ContextMenu.Separator class="context-menu-divider" />
                  <ContextMenu.Item
                    class="context-menu-item"
                    onSelect={() => {
                      if (onOpenFileWithDefaultApp) onOpenFileWithDefaultApp(tab.path);
                    }}
                  >
                    <ExternalLink size={14} class="context-menu-item-icon" />
                    <span>Abrir con la app por defecto</span>
                  </ContextMenu.Item>
                  <ContextMenu.Item
                    class="context-menu-item"
                    onSelect={() => {
                      if (onShowItemInFolder) onShowItemInFolder(tab.path);
                    }}
                  >
                    <FolderSearch size={14} class="context-menu-item-icon" />
                    <span>Mostrar en carpeta</span>
                  </ContextMenu.Item>
                  <ContextMenu.Separator class="context-menu-divider" />
                  <ContextMenu.Item
                    class="context-menu-item"
                    onSelect={() => {
                      navigator.clipboard.writeText(tab.path);
                    }}
                  >
                    <Copy size={14} class="context-menu-item-icon" />
                    <span>Copiar ruta</span>
                  </ContextMenu.Item>
                {/if}
              </ContextMenu.Content>
            </ContextMenu.Portal>
          </ContextMenu.Root>
        {/each}
      {:else if title}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="tab-title-container active"
          onauxclick={(e) => handleTabAuxClick(e, '')}
          onmousedown={handleTabMouseDown}
        >
          <FileIcon path="" name={title} size={14} class="file-icon" />
          <span class="tab-title">{title}</span>
          <button
            type="button"
            class="close-tab-btn"
            onclick={() => { if (onCloseTab) onCloseTab(''); }}
            title="Cerrar pestaña (Ctrl+W, Ctrl+F4 o click central)"
          >
            <X size={12} />
          </button>
        </div>
      {/if}

      <!-- BOTÓN NUEVA PESTAÑA: SIEMPRE PEGADO A LA IZQUIERDA, JUNTO AL ÚLTIMO TAB -->
      <button
        type="button"
        class="new-tab-btn"
        data-tauri-drag-region="false"
        onclick={() => { if (onNewTab) onNewTab(); }}
        title="Nueva pestaña (Ctrl+T)"
      >
        <Plus size={14} />
      </button>
    </div>

    {#if canScrollRight}
      <button
        type="button"
        class="tab-scroll-btn right"
        data-tauri-drag-region="false"
        onclick={scrollRight}
        title="Desplazar pestañas a la derecha"
      >
        <ChevronRight size={14} />
      </button>
      <!-- BOTÓN NUEVA PESTAÑA: SIEMPRE PEGADO A LA IZQUIERDA, JUNTO AL ÚLTIMO TAB -->
      <button
        type="button"
        class="new-tab-btn"
        data-tauri-drag-region="false"
        onclick={() => { if (onNewTab) onNewTab(); }}
        title="Nueva pestaña (Ctrl+T)"
      >
        <Plus size={14} />
      </button>
    {/if}
  </div>

  <!-- SECCIÓN DERECHA -->
  <div class="right-section">
    <!-- MENÚ DESPLEGABLE PARA LISTAR Y BUSCAR TODAS LAS PESTAÑAS Y BOTÓN DE CERRAR TODAS -->
    <!-- AL COSTADO IZQUIERDO DE LOS BOTONES DE GRABAR Y EDITAR -->
    {#if tabs && tabs.length > 0}
      <div class="tabs-actions-group">
        <Popover.Root
          bind:open={isTabsMenuOpen}
          onOpenChange={(open) => {
            if (open) {
              openTabsMenu();
            }
          }}
        >
          <Popover.Trigger>
            {#snippet child({ props })}
              <button
                type="button"
                class="icon-btn tab-dropdown-trigger"
                class:active={isTabsMenuOpen}
                title="Listar y buscar pestañas abiertas (Ctrl+Shift+A)"
                {...props}
              >
                <ChevronDown size={14} class="icon" />
              </button>
            {/snippet}
          </Popover.Trigger>

          <Popover.Portal>
            <Popover.Content class="tabs-dropdown-menu" align="end" side="bottom" sideOffset={6}>
              <div class="dropdown-header">
                <span>Pestañas ({filteredTabs.length}{filteredTabs.length !== tabs.length ? `/${tabs.length}` : ''})</span>
                <button type="button" class="dropdown-close-all-btn" onclick={handleCloseAll}>
                  Cerrar todas
                </button>
              </div>

              <!-- CAJA DE TEXTO PARA BUSCAR RÁPIDAMENTE ENTRE LAS PESTAÑAS ABIERTAS -->
              <div class="dropdown-search-container">
                <Search size={14} class="dropdown-search-icon" />
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
                    <X size={12} />
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
                      {#if tab.path.startsWith('diff-staged:')}
                        <GitCompareArrows size={14} class="dropdown-file-icon" style="color: #22c55e;" />
                      {:else if tab.path.startsWith('diff-rev:')}
                        <GitCompare size={14} class="dropdown-file-icon" style="color: #a855f7;" />
                      {:else if tab.path.startsWith('diff:')}
                        <GitCompare size={14} class="dropdown-file-icon" style="color: #3b82f6;" />
                      {:else if tab.path.startsWith('rev:')}
                        <History size={14} class="dropdown-file-icon" style="color: #0969da;" />
                      {:else}
                        <FileIcon path={tab.path} name={tab.title} kind={tab.kind} size={14} class="dropdown-file-icon" />
                      {/if}
                      <span class="dropdown-tab-title">{tab.title}{tab.isDirty ? ' *' : ''}</span>
                      {#if tab.isLocked}
                        <span class="tab-lock-badge" title="Archivo bloqueado (auto-guardado desactivado)">
                          <Lock size={11} class="tab-lock-icon" />
                        </span>
                      {/if}
                      <button
                        type="button"
                        class="dropdown-tab-close"
                        onclick={(e) => {
                          e.stopPropagation();
                          if (onCloseTab) onCloseTab(tab.path);
                        }}
                        title="Cerrar pestaña (Ctrl+W, Ctrl+F4 o click central)"
                      >
                        <X size={12} />
                      </button>
                    </div>
                  {/each}
                {:else}
                  <div class="dropdown-no-results">
                    No se encontraron pestañas abiertas
                  </div>
                {/if}
              </div>
            </Popover.Content>
          </Popover.Portal>
        </Popover.Root>
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
        <Save size={14} class="icon" />
        <span>Grabar</span>
      </button>
    {/if}

    <!-- BOTÓN PRINCIPAL EDICIÓN / LECTURA (SOLO CUANDO HAY TAB ABIERTO CON CONTENIDO) -->
    {#if showViewToggle}
      <button
        type="button"
        class="view-toggle-btn"
        onclick={() => {
          if (onToggleView) {
            onToggleView();
          } else {
            isEditing = !isEditing;
          }
        }}
        title={isEditing ? "Cambiar a modo Lectura" : "Cambiar a modo Edición"}
      >
        {#if isEditing}
          <PenLine size={14} class="icon" />
        {:else}
          <Eye size={14} class="icon" />
        {/if}
        <span>{isEditing ? "Edición" : "Lectura"}</span>
      </button>
    {/if}

    <button
      type="button"
      class="icon-btn"
      class:active-btn={isSplitActive}
      onclick={() => { if (onSplitView) onSplitView(); }}
      ondragover={(e) => {
        if (e.dataTransfer?.types.includes('application/x-synapse-tab') || e.dataTransfer?.types.includes('text/plain')) {
          e.preventDefault();
          if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
        }
      }}
      ondrop={(e) => {
        e.preventDefault();
        e.stopPropagation();
        const data = parseTabDragData(e);
        if (data?.path && onSplitView) {
          onSplitView(data.path);
        }
      }}
      title={isSplitActive ? "Cerrar división de paneles" : "Dividir panel verticalmente"}
    >
      <Columns2 size={14} class="icon" />
    </button>

    {#if activeTabPath && !isSyntheticTab(activeTabPath)}
      <button
        type="button"
        class="icon-btn"
        onclick={() => { if (onOpenInNewWindow) onOpenInNewWindow(activeTabPath, false); }}
        title="Abrir pestaña en nueva ventana"
      >
        <ExternalLink size={14} class="icon" />
      </button>
    {/if}

    <button type="button" class="icon-btn" onclick={() => triggerAction('more-options')} title="Más opciones del archivo">
      <MoreHorizontal size={14} class="icon" />
    </button>

    <!-- Espaciador dedicado para arrastrar la ventana incluso con múltiples pestañas abiertas -->
    <div class="window-drag-spacer" data-tauri-drag-region title="Arrastrar ventana"></div>

    {#if showWindowControls}
      <WindowControls />
    {/if}
  </div>
</header>

<style>
  .editor-header {
    height: 42px;
    min-height: 42px;
    max-height: 42px;
    flex-shrink: 0;
    background-color: var(--bg-primary, #ffffff);
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    display: flex;
    align-items: center;
    padding: 0 10px;
    user-select: none;
    gap: 8px;
    position: relative;
    box-sizing: border-box;
    overflow: hidden;
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
    flex-wrap: nowrap;
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

  .tab-scroll-btn :global(svg) {
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

  .new-tab-btn :global(svg) {
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

  .tab-title-container.is-dragging {
    opacity: 0.35;
    transform: scale(0.98);
  }

  .tab-title-container.drag-target-left {
    position: relative;
    border-left: 2px solid var(--accent, #0969da);
  }

  .tab-title-container.drag-target-right {
    position: relative;
    border-right: 2px solid var(--accent, #0969da);
  }

  .tabs-scroll-container.drop-active {
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.04));
    border-radius: 6px;
  }

  :global(.file-icon) {
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

  :global(.tab-lock-icon) {
    color: var(--text-muted, #8c959f);
    flex-shrink: 0;
    margin-left: 2px;
  }

  .tab-title-container.active :global(.tab-lock-icon) {
    color: var(--accent, #0969da);
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
    background: var(--hover-bg, rgba(0, 0, 0, 0.1));
    color: var(--text-primary, #1f2328);
  }

  .close-tab-btn :global(svg) {
    width: 12px;
    height: 12px;
  }

  .right-section {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .window-drag-spacer {
    flex: 0 0 24px;
    height: 100%;
    cursor: default;
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

  :global(.tabs-dropdown-menu) {
    width: 300px;
    max-height: 420px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    box-shadow: var(--popover-shadow, 0 8px 24px rgba(0, 0, 0, 0.14));
    z-index: 1000;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    outline: none;
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

  :global(.tabs-dropdown-menu .dropdown-header) {
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

  :global(.tabs-dropdown-menu .dropdown-close-all-btn) {
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

  :global(.tabs-dropdown-menu .dropdown-close-all-btn:hover) {
    background: rgba(207, 34, 46, 0.1);
  }

  :global(.tabs-dropdown-menu .dropdown-search-container) {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    background: var(--bg-primary, #ffffff);
  }

  :global(.tabs-dropdown-menu .dropdown-search-icon) {
    width: 14px;
    height: 14px;
    color: var(--text-secondary, #656d76);
    flex-shrink: 0;
  }

  :global(.tabs-dropdown-menu .dropdown-search-input) {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    font-size: 12px;
    color: var(--text-primary, #1f2328);
    outline: none;
    padding: 0;
  }

  :global(.tabs-dropdown-menu .dropdown-search-input::placeholder) {
    color: var(--text-secondary, #656d76);
    font-size: 11px;
  }

  :global(.tabs-dropdown-menu .dropdown-search-clear) {
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

  :global(.tabs-dropdown-menu .dropdown-search-clear:hover) {
    background: rgba(0, 0, 0, 0.08);
    color: var(--text-primary, #1f2328);
  }

  :global(.tabs-dropdown-menu .dropdown-search-clear svg) {
    width: 12px;
    height: 12px;
  }

  :global(.tabs-dropdown-menu .dropdown-tabs-list) {
    overflow-y: auto;
    max-height: 300px;
    padding: 4px 0;
  }

  :global(.tabs-dropdown-menu .dropdown-tab-item) {
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

  :global(.tabs-dropdown-menu .dropdown-tab-item:hover),
  :global(.tabs-dropdown-menu .dropdown-tab-item.selected),
  :global(.tabs-dropdown-menu .dropdown-tab-item[data-highlighted]) {
    background: rgba(9, 105, 218, 0.08);
  }

  :global(.tabs-dropdown-menu .dropdown-tab-item.selected),
  :global(.tabs-dropdown-menu .dropdown-tab-item[data-highlighted]) {
    border-left-color: var(--accent, #0969da);
  }

  :global(.tabs-dropdown-menu .dropdown-tab-item.active) {
    color: var(--accent, #0969da);
    font-weight: 500;
  }

  :global(.tabs-dropdown-menu .dropdown-file-icon) {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    color: var(--accent, #0969da);
  }

  :global(.tabs-dropdown-menu .dropdown-tab-title) {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.tabs-dropdown-menu .dropdown-tab-close) {
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

  :global(.tabs-dropdown-menu .dropdown-tab-close:hover) {
    opacity: 1;
    background: var(--hover-bg, rgba(0, 0, 0, 0.08));
    color: var(--text-primary, #1f2328);
  }

  :global(.tabs-dropdown-menu .dropdown-tab-close svg) {
    width: 12px;
    height: 12px;
  }

  :global(.tabs-dropdown-menu .dropdown-no-results) {
    padding: 16px 12px;
    text-align: center;
    color: var(--text-secondary, #656d76);
    font-size: 13px;
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
    background-color: var(--hover-bg, rgba(0, 0, 0, 0.05));
  }

  .icon-btn.active-btn {
    background-color: var(--bg-tertiary, #eaeef2);
    color: var(--accent, #0969da);
  }

  .icon-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
    pointer-events: none;
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
    background: var(--hover-bg, rgba(0, 0, 0, 0.05));
    color: var(--accent, #0969da);
    border-color: var(--accent, #0969da);
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

  :global(.tab-context-menu) {
    min-width: 190px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    padding: 4px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12), 0 2px 6px rgba(0, 0, 0, 0.08);
    z-index: 9999;
    display: flex;
    flex-direction: column;
    user-select: none;
    outline: none;
    animation: context-menu-fade 0.1s ease-out;
  }

  @keyframes context-menu-fade {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  :global(.tab-context-menu .context-menu-divider) {
    height: 1px;
    background: var(--border-primary, #d0d7de);
    margin: 4px 0;
  }

  :global(.tab-context-menu .context-menu-item) {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border: none;
    background: transparent;
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-primary, #1f2328);
    cursor: pointer;
    text-align: left;
    width: 100%;
    transition: background 0.1s ease, color 0.1s ease;
    font-family: inherit;
    outline: none;
    box-sizing: border-box;
  }

  :global(.tab-context-menu .context-menu-item:hover),
  :global(.tab-context-menu .context-menu-item[data-highlighted]) {
    background: var(--accent-bg, rgba(9, 105, 218, 0.08));
    color: var(--accent, #0969da);
  }

  :global(.tab-context-menu .context-menu-item-icon) {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
  }
</style>
