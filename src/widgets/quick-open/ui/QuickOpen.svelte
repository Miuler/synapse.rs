<script lang="ts">
  import { untrack } from 'svelte';
  import type { VaultItem, TagCountItem } from '@entities/vault-item';
  import { Command, Dialog } from 'bits-ui';
  import { Search, TextSearch, Tag, Folder, ChevronDown, ChevronUp } from 'lucide-svelte';
  import { FileIcon } from '@entities/file-type';
  import { SearchBox } from '@shared/ui/search-box';
  import { handleRelativePageJump } from '@shared/lib/navigation';
  import {
    vaultRepository,
    searchRepository,
    type SearchResult,
    type FullTextHit,
    type FolderMeta,
  } from '@shared/repositories';

  interface Props {
    isOpen?: boolean;
    searchQuery?: string;
    vaultItems?: VaultItem[];
    recentFiles?: string[];
    onSelectFile?: (path: string, matchedTerms?: string[]) => void;
    onRevealFolder?: (folderPath: string) => void;
    onClose?: () => void;
  }

  let {
    isOpen = $bindable(false),
    searchQuery = $bindable(''),
    vaultItems = [],
    recentFiles = [],
    onSelectFile,
    onRevealFolder,
    onClose
  }: Props = $props();

  // Constante de paginado para carga bajo demanda en scroll
  const PAGE_SIZE = 25;

  let nucleoResults = $state<SearchResult[]>([]);
  let nucleoQuery = $state('');
  let folderResults = $state<FolderMeta[]>([]);
  let isFoldersExpanded = $state(false);
  let selectedCommandValue = $state<string>('');
  let visibleCount = $state(PAGE_SIZE);
  let listContainerEl = $state<HTMLElement | null>(null);
  let isSearching = $state(false);

  // Modo carpeta si la consulta inicia con '/'
  const isFolderMode = $derived(searchQuery.trim().startsWith('/'));
  const folderQuery = $derived(isFolderMode ? searchQuery.trim().slice(1).trim() : '');

  // Carpetas visibles: en modo normal se muestran solo las 3 primeras a menos que se hayan expandido
  const visibleFolders = $derived.by<FolderMeta[]>(() => {
    if (isFolderMode || isFoldersExpanded || folderResults.length <= 3) {
      return folderResults;
    }
    return folderResults.slice(0, 3);
  });

  const remainingFoldersCount = $derived(
    !isFolderMode && !isFoldersExpanded && folderResults.length > 3
      ? folderResults.length - 3
      : 0
  );

  // Modo búsqueda full-text si la consulta inicia con '?'
  const isFtsMode = $derived(searchQuery.startsWith('?'));
  const ftsQuery = $derived(isFtsMode ? searchQuery.slice(1).trim() : '');
  let ftsResults = $state<FullTextHit[]>([]);
  let ftsTotalHits = $state(0);
  let ftsElapsedMs = $state(0);
  let isFtsLoading = $state(false);

  // Estadísticas de total de archivos y filtrados en memoria
  let totalFiles = $state<number>(0);
  let matchedFiles = $state<number>(0);

  // Dimensiones redimensionables del cuadro de diálogo con persistencia en localStorage
  function getInitialDialogWidth(): number {
    try {
      const saved = localStorage.getItem('synapse_quick_open_width');
      if (saved) {
        const p = parseInt(saved, 10);
        if (!isNaN(p) && p >= 400 && p <= 1600) return p;
      }
    } catch {}
    return 620;
  }

  function getInitialDialogHeight(): number {
    try {
      const saved = localStorage.getItem('synapse_quick_open_height');
      if (saved) {
        const p = parseInt(saved, 10);
        if (!isNaN(p) && p >= 280 && p <= 1200) return p;
      }
    } catch {}
    return 480;
  }

  let dialogWidth = $state<number>(getInitialDialogWidth());
  let dialogHeight = $state<number>(getInitialDialogHeight());
  let isResizing = $state(false);

  function startResize(e: PointerEvent, direction: 'nw' | 'ne' | 'sw' | 'se' | 'n' | 's' | 'e' | 'w') {
    e.preventDefault();
    e.stopPropagation();
    isResizing = true;
    document.body.style.userSelect = 'none';

    const startX = e.clientX;
    const startY = e.clientY;
    const startW = dialogWidth;
    const startH = dialogHeight;

    const onPointerMove = (ev: PointerEvent) => {
      const deltaX = ev.clientX - startX;
      const deltaY = ev.clientY - startY;
      const maxW = typeof window !== 'undefined' ? window.innerWidth * 0.95 : 1400;
      const maxH = typeof window !== 'undefined' ? window.innerHeight * 0.85 : 900;

      if (direction === 'se') {
        dialogWidth = Math.max(400, Math.min(startW + deltaX * 2, maxW));
        dialogHeight = Math.max(280, Math.min(startH + deltaY, maxH));
      } else if (direction === 'sw') {
        dialogWidth = Math.max(400, Math.min(startW - deltaX * 2, maxW));
        dialogHeight = Math.max(280, Math.min(startH + deltaY, maxH));
      } else if (direction === 'ne') {
        dialogWidth = Math.max(400, Math.min(startW + deltaX * 2, maxW));
        dialogHeight = Math.max(280, Math.min(startH - deltaY, maxH));
      } else if (direction === 'nw') {
        dialogWidth = Math.max(400, Math.min(startW - deltaX * 2, maxW));
        dialogHeight = Math.max(280, Math.min(startH - deltaY, maxH));
      } else if (direction === 'e') {
        dialogWidth = Math.max(400, Math.min(startW + deltaX * 2, maxW));
      } else if (direction === 'w') {
        dialogWidth = Math.max(400, Math.min(startW - deltaX * 2, maxW));
      } else if (direction === 's') {
        dialogHeight = Math.max(280, Math.min(startH + deltaY, maxH));
      } else if (direction === 'n') {
        dialogHeight = Math.max(280, Math.min(startH - deltaY, maxH));
      }
    };

    const onPointerUp = () => {
      isResizing = false;
      document.body.style.userSelect = '';
      window.removeEventListener('pointermove', onPointerMove);
      window.removeEventListener('pointerup', onPointerUp);
      window.removeEventListener('pointercancel', onPointerUp);
      try {
        localStorage.setItem('synapse_quick_open_width', String(Math.round(dialogWidth)));
        localStorage.setItem('synapse_quick_open_height', String(Math.round(dialogHeight)));
      } catch {}
    };

    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', onPointerUp);
    window.addEventListener('pointercancel', onPointerUp);
  }

  interface FileItemDisplay {
    id: string;
    title: string;
    path: string;
    fileName: string;
    isRecent?: boolean;
    tags?: string[];
  }

  // Lista de tags de la bóveda para autocompletado y sugerencias
  let vaultTags = $state<TagCountItem[]>([]);
  let isTagMode = $derived(searchQuery.trim().startsWith('#') && !searchQuery.trim().includes(' '));
  let tagSearchPrefix = $derived(isTagMode ? searchQuery.trim().slice(1).toLowerCase() : '');
  let filteredTagSuggestions = $derived.by<TagCountItem[]>(() => {
    if (!isTagMode) return [];
    if (!tagSearchPrefix) return vaultTags.slice(0, 30);
    return vaultTags.filter((t) => t.tag.toLowerCase().includes(tagSearchPrefix)).slice(0, 30);
  });

  // Mapa indexado de vaultItems para búsquedas O(1) en vez de find() O(N)
  let vaultItemMap = $derived.by<Map<string, VaultItem>>(() => {
    const map = new Map<string, VaultItem>();
    for (const item of vaultItems) {
      if (item.relative_path) {
        map.set(item.relative_path, item);
      }
    }
    return map;
  });

  // Prepara los candidatos completos para la lista priorizando el nombre del archivo
  let allCandidates = $derived.by<FileItemDisplay[]>(() => {
    const trimmed = searchQuery.trim();
    if (trimmed.length > 0) {
      if (nucleoResults.length > 0 && nucleoQuery === trimmed) {
        return nucleoResults.map((r) => {
          const path = r.note_path || r.text;
          const fileName = path.split('/').pop() || path;
          const isRecent = Boolean(r.is_recent || recentFiles.includes(path));
          return {
            id: path,
            title: r.text,
            path: path,
            fileName,
            isRecent,
            tags: r.tags,
          };
        });
      }

      // Fallback instantáneo en cliente (0ms): mientras nucleo responde o si no hubo resultados de nucleo
      const qLower = trimmed.toLowerCase();
      // Cada palabra de la consulta debe aparecer en el campo, en cualquier orden ("bcp ttib" ~ "BCP - TTIB ...")
      const qTokens = qLower.split(/\s+/).filter(Boolean);
      const matchesQuery = (text: string) => {
        const lower = text.toLowerCase();
        return qTokens.every((token) => lower.includes(token));
      };
      const filtered: FileItemDisplay[] = [];
      const added = new Set<string>();

      // Priorizar recientes
      for (const path of recentFiles) {
        if (!path || path.startsWith('empty:') || added.has(path)) continue;
        const fileName = path.split('/').pop() || path;
        const item = vaultItemMap.get(path);
        const title = item ? item.title : fileName.replace(/\.[^/.]+$/, '');
        if (matchesQuery(fileName) || matchesQuery(title) || matchesQuery(path)) {
          filtered.push({
            id: path,
            title,
            path,
            fileName,
            isRecent: true,
          });
          added.add(path);
        }
      }

      // Añadir el resto de archivos que coincidan
      for (const item of vaultItems) {
        if (!item.relative_path || item.relative_path.startsWith('empty:') || added.has(item.relative_path)) continue;
        const fileName = item.relative_path.split('/').pop() || item.relative_path;
        if (matchesQuery(fileName) || matchesQuery(item.title) || matchesQuery(item.relative_path)) {
          filtered.push({
            id: item.relative_path,
            title: item.title,
            path: item.relative_path,
            fileName,
            isRecent: false,
          });
          added.add(item.relative_path);
        }
      }

      // Ordenar con los mismos tiers estrictos de Rust:
      // 0: Stem/filename exacto
      // 1: Filename contiene query
      // 2: Título exacto
      // 3: Título contiene query
      // 4: Ruta contiene query
      const getTier = (item: FileItemDisplay): number => {
        const fLower = item.fileName.toLowerCase();
        const stem = fLower.includes('.') ? fLower.slice(0, fLower.lastIndexOf('.')) : fLower;
        const tLower = item.title.toLowerCase();
        if (stem === qLower || fLower === qLower) return 0;
        if (fLower.includes(qLower)) return 1;
        if (tLower === qLower) return 2;
        if (tLower.includes(qLower)) return 3;
        return 4;
      };

      filtered.sort((a, b) => {
        const tierA = getTier(a);
        const tierB = getTier(b);
        const isFileA = tierA <= 1;
        const isFileB = tierB <= 1;

        if (isFileA !== isFileB) return isFileA ? -1 : 1;
        if (!isFileA && tierA !== tierB) return tierA - tierB;
        if (a.isRecent !== b.isRecent) return a.isRecent ? -1 : 1;
        if (tierA !== tierB) return tierA - tierB;
        return a.fileName.localeCompare(b.fileName);
      });

      return filtered;
    }

    const list: FileItemDisplay[] = [];
    const addedPaths = new Set<string>();

    // 1. Si no hay búsqueda, PRIORIDAD a los últimos archivos que abrió el usuario
    for (const path of recentFiles) {
      if (!path || path.startsWith('empty:') || addedPaths.has(path)) continue;
      const item = vaultItemMap.get(path);
      const fileName = path.split('/').pop() || path;
      list.push({
        id: path,
        title: item ? item.title : fileName.replace(/\.[^/.]+$/, ''),
        path: path,
        fileName,
        isRecent: true,
      });
      addedPaths.add(path);
    }

    // 2. Añadir el resto de archivos de la bóveda para permitir explorarlos paginados
    for (const item of vaultItems) {
      if (!item.relative_path || item.relative_path.startsWith('empty:') || addedPaths.has(item.relative_path)) continue;
      const fileName = item.relative_path.split('/').pop() || item.relative_path;
      list.push({
        id: item.relative_path,
        title: item.title,
        path: item.relative_path,
        fileName,
        isRecent: false,
      });
      addedPaths.add(item.relative_path);
    }

    return list;
  });

  // Lista visible en el DOM recortada al tamaño de página (apertura instantánea sin retrasos)
  let fileList = $derived(allCandidates.slice(0, visibleCount));

  // Determinar si existen archivos recientes para condicionalmente renderizar líneas divisoras
  let hasRecents = $derived(fileList.some((f) => f.isRecent));

  let prevQuery = '';
  let prevIsOpen = false;

  // Reiniciar el límite de página visible solo cuando realmente cambia la búsqueda o se abre el diálogo
  $effect(() => {
    const currentQ = searchQuery;
    const currentOpen = isOpen;

    if (currentQ !== prevQuery || currentOpen !== prevIsOpen) {
      prevQuery = currentQ;
      prevIsOpen = currentOpen;

      untrack(() => {
        visibleCount = PAGE_SIZE;
        isFoldersExpanded = false;
        if (listContainerEl) {
          listContainerEl.scrollTop = 0;
        }
      });
    }
  });

  // Obtener el total de archivos y etiquetas en memoria solo al abrir el diálogo
  let wasOpenForCount = false;
  $effect(() => {
    const currentOpen = isOpen;
    if (currentOpen && !wasOpenForCount) {
      wasOpenForCount = true;
      untrack(() => {
        vaultRepository.getVaultFilesCount().then((count) => {
          if (count > 0) {
            totalFiles = count;
            if (!searchQuery.trim()) {
              matchedFiles = count;
            }
          } else if (vaultItems.length > 0) {
            totalFiles = vaultItems.length;
            if (!searchQuery.trim()) {
              matchedFiles = vaultItems.length;
            }
          }
        });
        vaultRepository.getVaultTags().then((tags) => {
          vaultTags = tags;
        }).catch(() => {
          vaultTags = [];
        });
      });
    } else if (!currentOpen) {
      wasOpenForCount = false;
    }
  });

  // Búsqueda interactiva ultrarrápida con Nucleo en Rust (solo cuando no es modo FTS)
  let prevNucleoQuery = '';
  let prevNucleoOpen = false;
  let prevNucleoFts = false;

  $effect(() => {
    const q = searchQuery.trim();
    const currentOpen = isOpen;
    const fts = isFtsMode;

    if (q !== prevNucleoQuery || currentOpen !== prevNucleoOpen || fts !== prevNucleoFts) {
      prevNucleoQuery = q;
      prevNucleoOpen = currentOpen;
      prevNucleoFts = fts;

      if (!q || !currentOpen || fts) {
        untrack(() => {
          nucleoResults = [];
          nucleoQuery = '';
          folderResults = [];
          matchedFiles = untrack(() => totalFiles);
          isSearching = false;
        });
        return;
      }

      untrack(() => {
        isSearching = true;
      });
      let active = true;

      // Pequeño debounce para no saturar llamadas IPC durante tecleo continuo rápido
      const timer = setTimeout(() => {
        if (isFolderMode) {
          vaultRepository.searchFolders(folderQuery, 30, 0).then((folders) => {
            if (active) {
              untrack(() => {
                folderResults = folders;
                nucleoResults = [];
                nucleoQuery = q;
                isSearching = false;
              });
            }
          }).catch(() => {
            if (active) {
              untrack(() => {
                folderResults = [];
                isSearching = false;
              });
            }
          });
        } else {
          vaultRepository.searchNotes(q).then((resp) => {
            if (active) {
              untrack(() => {
                nucleoResults = resp.results;
                folderResults = resp.folders || [];
                nucleoQuery = q;
                if (resp.total_files > 0) totalFiles = resp.total_files;
                matchedFiles = resp.matched_files;
                isSearching = false;
              });
            }
          }).catch(() => {
            if (active) {
              untrack(() => {
                isSearching = false;
              });
            }
          });
        }
      }, 40);

      return () => {
        active = false;
        clearTimeout(timer);
      };
    }
  });

  // Búsqueda interactiva Full-Text con Tantivy cuando inicia con '?'
  let prevFtsQuery = '';
  let prevFtsOpen = false;
  let prevFtsMode = false;

  $effect(() => {
    const currentOpen = isOpen;
    const fts = isFtsMode;
    const q = ftsQuery;

    if (currentOpen !== prevFtsOpen || fts !== prevFtsMode || q !== prevFtsQuery) {
      prevFtsOpen = currentOpen;
      prevFtsMode = fts;
      prevFtsQuery = q;

      if (!currentOpen || !fts || !q) {
        untrack(() => {
          ftsResults = [];
          ftsTotalHits = 0;
          ftsElapsedMs = 0;
          isFtsLoading = false;
        });
        return;
      }

      untrack(() => {
        isFtsLoading = true;
      });
      let active = true;

      const timer = setTimeout(() => {
        searchRepository
          .search(q, 50)
          .then((resp) => {
            if (active) {
              untrack(() => {
                ftsResults = resp.hits;
                ftsTotalHits = resp.total_hits;
                ftsElapsedMs = resp.elapsed_ms;
                isFtsLoading = false;
              });
            }
          })
          .catch((err) => {
            if (active) {
              console.error('Error en búsqueda FTS desde QuickOpen:', err);
              untrack(() => {
                isFtsLoading = false;
              });
            }
          });
      }, 120);

      return () => {
        active = false;
        clearTimeout(timer);
      };
    }
  });

  function handleScroll(e: Event) {
    const target = e.currentTarget as HTMLElement;
    if (!target) return;
    const distanceToBottom = target.scrollHeight - target.scrollTop - target.clientHeight;
    if (distanceToBottom < 100 && visibleCount < allCandidates.length) {
      visibleCount = Math.min(visibleCount + PAGE_SIZE, allCandidates.length);
    }
  }

  function handleKeydownRoot(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' && visibleCount < allCandidates.length) {
      const highlighted = listContainerEl?.querySelector('[data-highlighted], [data-selected]');
      if (highlighted && highlighted.nextElementSibling === null) {
        visibleCount = Math.min(visibleCount + PAGE_SIZE, allCandidates.length);
      }
    } else if (e.key === 'PageDown' || e.key === 'PageUp') {
      e.preventDefault();
      e.stopPropagation();

      const allItems = listContainerEl?.querySelectorAll<HTMLElement>('.palette-item');
      if (!allItems || allItems.length === 0) return;

      handleRelativePageJump({
        containerEl: listContainerEl,
        direction: e.key === 'PageDown' ? 'pagedown' : 'pageup',
        items: allItems,
        fallbackItemHeight: 40,
        onNearEnd: () => {
          if (visibleCount < allCandidates.length) {
            visibleCount = Math.min(visibleCount + PAGE_SIZE, allCandidates.length);
          }
        },
        onSelect: (targetItem) => {
          const val = targetItem.getAttribute('data-value');
          if (val) {
            selectedCommandValue = val;
          }
        },
      });
    }
  }

  function closeDialog() {
    isOpen = false;
    searchQuery = '';
    visibleCount = PAGE_SIZE;
    if (onClose) onClose();
  }

  function selectFile(path: string, matchedTerms?: string[]) {
    if (onSelectFile) onSelectFile(path, matchedTerms);
    closeDialog();
  }
</script>

<Dialog.Root
  open={isOpen}
  onOpenChange={(open) => {
    isOpen = open;
    if (!open) {
      closeDialog();
    }
  }}
>
  <Dialog.Portal>
    <Dialog.Overlay class="quick-open-backdrop" />
    <Dialog.Content
      class={`quick-open-container ${isResizing ? 'is-resizing' : ''}`}
      style="width: {dialogWidth}px; height: {dialogHeight}px;"
    >
      <!-- Handles de redimensionamiento desde las 4 esquinas y 4 bordes -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-handle corner nw" onpointerdown={(e) => startResize(e, 'nw')}></div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-handle corner ne" onpointerdown={(e) => startResize(e, 'ne')}></div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-handle corner sw" onpointerdown={(e) => startResize(e, 'sw')}></div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-handle corner se" onpointerdown={(e) => startResize(e, 'se')}>
        <svg class="corner-grip-icon" viewBox="0 0 10 10" width="10" height="10">
          <path d="M9 1L1 9 M9 5L5 9 M9 9L9 9" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
        </svg>
      </div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-handle edge n" onpointerdown={(e) => startResize(e, 'n')}></div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-handle edge s" onpointerdown={(e) => startResize(e, 's')}></div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-handle edge e" onpointerdown={(e) => startResize(e, 'e')}></div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-handle edge w" onpointerdown={(e) => startResize(e, 'w')}></div>

      <Dialog.Title class="sr-only">Buscador Rápido de Archivos</Dialog.Title>
      <Command.Root
        bind:value={selectedCommandValue}
        class="quick-open-command-root"
        loop
        shouldFilter={false}
        onkeydown={handleKeydownRoot}
      >
        <SearchBox
          class={isFtsMode ? 'is-fts' : isTagMode ? 'is-tag-mode' : isFolderMode ? 'is-folder-mode' : ''}
          bind:value={searchQuery}
          placeholder={isFtsMode ? "Buscar en el contenido de las notas... (Tantivy)" : isTagMode ? "Filtrar por etiqueta... (#nombre)" : isFolderMode ? "Explorar carpetas... (/nombre)" : "Buscar archivo o carpeta... (Ctrl+O, '/' carpetas, '?' contenido, '#' tags)"}
          loading={isFtsLoading}
        >
          {#snippet icon()}
            {#if isFtsMode}
              <TextSearch size={16} class="search-box-icon fts-active-icon" />
            {:else if isTagMode}
              <Tag size={16} class="search-box-icon tag-active-icon" />
            {:else if isFolderMode}
              <Folder size={16} class="search-box-icon folder-active-icon" />
            {:else}
              <Search size={16} class="search-box-icon" />
            {/if}
          {/snippet}

          {#snippet badge()}
            {#if isFtsMode}
              <span class="fts-indicator-badge">FTS</span>
            {:else if isTagMode}
              <span class="tag-indicator-badge">TAG</span>
            {:else if isFolderMode}
              <span class="folder-indicator-badge">DIR</span>
            {/if}
          {/snippet}
        </SearchBox>

        <Command.List
          bind:ref={listContainerEl}
          class="quick-open-results-container"
          onscroll={handleScroll}
        >
          {#if isFtsMode}
            {#if ftsQuery.length === 0}
              <div class="fts-mode-hint">
                <div class="fts-hint-title">
                  <TextSearch size={15} />
                  <span>Búsqueda Full-Text activada</span>
                </div>
                <p class="fts-hint-desc">
                  Escribe cualquier término tras el signo <code>?</code> para buscar en el interior de tus notas, diagramas y código.
                </p>
                <div class="fts-hint-tags">
                  <span>Ej: <code>?servidor</code></span>
                  <span><code>?"frase exacta"</code></span>
                  <span><code>?kind:markdown</code></span>
                </div>
              </div>
            {:else if ftsResults.length === 0 && !isFtsLoading}
              <Command.Empty class="empty-state">
                No se encontraron coincidencias en el contenido para <strong>"{ftsQuery}"</strong>
              </Command.Empty>
            {:else}
              {#each ftsResults as hit (hit.note_path)}
                <Command.Item
                  class="palette-item quick-fts-item"
                  value={hit.note_path}
                  forceMount={true}
                  onSelect={() => selectFile(hit.note_path, hit.matched_terms)}
                >
                  <div class="quick-fts-main">
                    <div class="quick-fts-header">
                      <FileIcon path={hit.note_path} size={15} class="file-icon" />
                      <span class="item-name">
                        {#each hit.title as seg}
                          {#if seg.highlighted}
                            <mark class="quick-fts-mark">{seg.text}</mark>
                          {:else}
                            {seg.text}
                          {/if}
                        {/each}
                      </span>
                      <span class="quick-fts-kind {hit.kind}">{hit.kind}</span>
                      <span class="item-path">{hit.note_path}</span>
                    </div>
                    {#if hit.snippet && hit.snippet.length > 0}
                      <div class="quick-fts-snippet">
                        {#each hit.snippet as seg}
                          {#if seg.highlighted}
                            <mark class="quick-fts-mark">{seg.text}</mark>
                          {:else}
                            {seg.text}
                          {/if}
                        {/each}
                      </div>
                    {/if}
                  </div>
                </Command.Item>
              {/each}
            {/if}
          {:else}
            {#if isTagMode && filteredTagSuggestions.length > 0 && fileList.length === 0}
              <div class="tag-suggestions-header">
                <Tag size={13} />
                <span>Etiquetas disponibles ({filteredTagSuggestions.length})</span>
              </div>
              {#each filteredTagSuggestions as t}
                <Command.Item
                  class="palette-item tag-suggestion-item"
                  value={`#${t.tag}`}
                  forceMount={true}
                  onSelect={() => {
                    searchQuery = `#${t.tag} `;
                  }}
                >
                  <Tag size={14} class="tag-icon" />
                  <span class="tag-name">#{t.tag}</span>
                  <span class="tag-count">{t.count} {t.count === 1 ? 'nota' : 'notas'}</span>
                </Command.Item>
              {/each}
            {:else if isSearching}
              <div class="empty-state">Buscando...</div>
            {:else if fileList.length === 0 && folderResults.length === 0}
              <div class="empty-state">{isFolderMode ? 'No se encontraron carpetas' : 'No se encontraron archivos'}</div>
            {/if}

            <!-- SECCIÓN DE CARPETAS COINCIDENTES -->
            {#if folderResults.length > 0}
              <div class="section-divider">
                <span class="section-divider-label">
                  Carpetas ({folderResults.length})
                </span>
              </div>
              {#each visibleFolders as folder (folder.relative_path)}
                <Command.Item
                  class="palette-item folder-palette-item"
                  value={`folder:${folder.relative_path} ${folder.name} ${(folder.dominant_tags || []).join(' ')}`}
                  forceMount={true}
                  onSelect={() => {
                    // Enter en carpeta: autocompleta la ruta con '/' para drill-down inmediato
                    searchQuery = `${folder.relative_path}/`;
                  }}
                >
                  <Folder size={15} class="folder-item-icon" />
                  <div class="folder-name-container">
                    <span class="item-name folder-title">{folder.name}</span>
                    {#if folder.parent}
                      <span class="folder-parent-path">{folder.parent}/</span>
                    {/if}
                  </div>

                  {#if folder.dominant_tags && folder.dominant_tags.length > 0}
                    <div class="item-tags-container">
                      {#each folder.dominant_tags.slice(0, 3) as tag}
                        <span class="folder-tag-pill">{tag}</span>
                      {/each}
                    </div>
                  {/if}

                  <div class="folder-meta-badge">
                    <span class="folder-notes-count">{folder.notes_count} {folder.notes_count === 1 ? 'nota' : 'notas'}</span>
                    {#if onRevealFolder}
                      <button
                        type="button"
                        class="folder-reveal-btn"
                        title="Revelar carpeta en el explorador"
                        onclick={(e) => {
                          e.stopPropagation();
                          if (onRevealFolder) onRevealFolder(folder.relative_path);
                          closeDialog();
                        }}
                      >
                        Revelar
                      </button>
                    {/if}
                  </div>
                </Command.Item>
              {/each}

              <!-- Elemento interactivo para colapsar / expandir más de 3 carpetas -->
              {#if !isFolderMode && folderResults.length > 3}
                <Command.Item
                  class="palette-item toggle-folders-item"
                  value="toggle_folders_expand"
                  forceMount={true}
                  onSelect={() => {
                    isFoldersExpanded = !isFoldersExpanded;
                  }}
                >
                  {#if !isFoldersExpanded}
                    <ChevronDown size={14} class="toggle-folders-icon" />
                    <span class="toggle-folders-label">Ver {remainingFoldersCount} carpetas más...</span>
                  {:else}
                    <ChevronUp size={14} class="toggle-folders-icon" />
                    <span class="toggle-folders-label">Mostrar menos carpetas</span>
                  {/if}
                </Command.Item>
              {/if}
            {/if}

            {#each fileList as file, i (file.id)}
              <!-- Líneas de sección solo si existen archivos recientes -->
              {#if hasRecents && i === 0 && file.isRecent}
                <div class="section-divider">
                  <span class="section-divider-label">Recientes</span>
                </div>
              {:else if hasRecents && !file.isRecent && (i === 0 || fileList[i - 1]?.isRecent)}
                <div class="section-divider">
                  <span class="section-divider-label">Archivos</span>
                </div>
              {/if}

              <Command.Item
                class="palette-item"
                value={`${file.fileName} ${file.title} ${file.path} ${(file.tags || []).join(' ')}`}
                forceMount={true}
                onSelect={() => selectFile(file.path)}
              >
                <FileIcon path={file.path} name={file.fileName} size={15} class="file-icon" />
                <span class="item-name">{file.fileName}</span>
                {#if file.tags && file.tags.length > 0}
                  <div class="item-tags-container">
                    {#each file.tags as tag}
                      <button
                        type="button"
                        class="item-tag-pill"
                        onclick={(e) => {
                          e.stopPropagation();
                          searchQuery = `#${tag} `;
                        }}
                      >
                        #{tag}
                      </button>
                    {/each}
                  </div>
                {/if}
                <span class="item-path">
                  {file.path}{file.title && file.title !== file.fileName && file.title !== file.fileName.replace(/\.[^/.]+$/, '') ? ` · ${file.title}` : ''}
                </span>
              </Command.Item>
            {/each}

            {#if visibleCount < allCandidates.length}
              <div class="scroll-more-indicator">
                Mostrando {visibleCount} de {allCandidates.length} archivos (desplázate para cargar más)
              </div>
            {/if}
          {/if}
        </Command.List>

        <footer class="quick-open-footer">
          <div class="footer-shortcuts">
            <span><kbd>↑</kbd> <kbd>↓</kbd> Navegar</span>
            <span><kbd>↵</kbd> Abrir</span>
            <span><kbd>esc</kbd> Cerrar</span>
          </div>
          <div class="footer-stats">
            {#if isFtsMode}
              {#if ftsQuery.length > 0}
                <span>{ftsTotalHits.toLocaleString()} {ftsTotalHits === 1 ? 'coincidencia' : 'coincidencias'} ({ftsElapsedMs.toFixed(1)} ms) en contenido</span>
              {:else}
                <span>Modo Full-Text (?...)</span>
              {/if}
            {:else if isTagMode}
              <span>Filtro de etiquetas (#tag) · {matchedFiles.toLocaleString()} notas</span>
            {:else if isFolderMode}
              <span>Modo Carpetas (/...) · {folderResults.length} carpetas</span>
            {:else if searchQuery.trim().length > 0}
              <span>{matchedFiles.toLocaleString()} de {totalFiles.toLocaleString()} archivos</span>
            {:else if totalFiles > 0}
              <span>{totalFiles.toLocaleString()} archivos</span>
            {/if}
          </div>
        </footer>
      </Command.Root>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.sr-only) {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border-width: 0;
  }

  :global(.quick-open-backdrop) {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background-color: var(--dialog-backdrop, rgba(10, 12, 16, 0.7));
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 14vh;
    z-index: 1000;
    animation: fadeIn 0.15s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  :global(.quick-open-container) {
    position: fixed;
    top: 14vh;
    left: 50%;
    transform: translateX(-50%);
    min-width: 400px;
    max-width: 95vw;
    min-height: 280px;
    max-height: 85vh;
    background-color: var(--bg-primary, #ffffff);
    border-radius: 12px;
    border: 1px solid var(--border-primary, #d0d7de);
    box-shadow: var(--modal-shadow, 0 24px 48px rgba(0, 0, 0, 0.1));
    overflow: hidden;
    display: flex;
    flex-direction: column;
    z-index: 1001;
    outline: none;
    animation: slideDown 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  :global(.quick-open-container.is-resizing) {
    animation: none !important;
    user-select: none !important;
  }

  @keyframes slideDown {
    from { transform: translateX(-50%) translateY(-12px) scale(0.98); opacity: 0; }
    to { transform: translateX(-50%) translateY(0) scale(1); opacity: 1; }
  }

  /* Handles de redimensionamiento */
  :global(.quick-open-container .resize-handle) {
    position: absolute;
    z-index: 100;
  }

  :global(.quick-open-container .resize-handle.corner) {
    width: 18px;
    height: 18px;
  }

  :global(.quick-open-container .resize-handle.corner.nw) {
    top: 0;
    left: 0;
    cursor: nwse-resize;
  }

  :global(.quick-open-container .resize-handle.corner.ne) {
    top: 0;
    right: 0;
    cursor: nesw-resize;
  }

  :global(.quick-open-container .resize-handle.corner.sw) {
    bottom: 0;
    left: 0;
    cursor: nesw-resize;
  }

  :global(.quick-open-container .resize-handle.corner.se) {
    bottom: 0;
    right: 0;
    cursor: nwse-resize;
    display: flex;
    align-items: flex-end;
    justify-content: flex-end;
    padding: 3px 4px;
  }

  :global(.quick-open-container .corner-grip-icon) {
    color: var(--text-secondary, #656d76);
    opacity: 0.4;
    pointer-events: none;
    transition: opacity 0.15s ease, color 0.15s ease;
  }

  :global(.quick-open-container .resize-handle.corner.se:hover .corner-grip-icon) {
    opacity: 1;
    color: var(--accent, #0969da);
  }

  :global(.quick-open-container .resize-handle.edge.n) {
    top: 0;
    left: 18px;
    right: 18px;
    height: 6px;
    cursor: ns-resize;
  }

  :global(.quick-open-container .resize-handle.edge.s) {
    bottom: 0;
    left: 18px;
    right: 18px;
    height: 6px;
    cursor: ns-resize;
  }

  :global(.quick-open-container .resize-handle.edge.w) {
    left: 0;
    top: 18px;
    bottom: 18px;
    width: 6px;
    cursor: ew-resize;
  }

  :global(.quick-open-container .resize-handle.edge.e) {
    right: 0;
    top: 18px;
    bottom: 18px;
    width: 6px;
    cursor: ew-resize;
  }

  :global(.quick-open-container .quick-open-command-root) {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    flex: 1 1 100%;
    min-height: 0;
    outline: none;
  }

  :global(.quick-open-container .quick-open-results-container) {
    flex: 1 1 0%;
    min-height: 0;
    max-height: none !important;
    height: auto !important;
    overflow-y: auto;
    padding: 6px 0;
    outline: none;
  }

  :global(.quick-open-container .empty-state) {
    padding: 24px;
    text-align: center;
    color: var(--text-secondary, #656d76);
    font-size: 14px;
  }

  :global(.quick-open-container .section-divider) {
    display: flex;
    align-items: center;
    padding: 10px 16px 4px;
    user-select: none;
    pointer-events: none;
  }

  :global(.quick-open-container .section-divider-label) {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary, #656d76);
    opacity: 0.75;
  }

  :global(.quick-open-container .section-divider::after) {
    content: '';
    flex: 1;
    margin-left: 10px;
    height: 1px;
    background-color: var(--border-primary, #d0d7de);
    opacity: 0.5;
  }

  :global(.quick-open-container .palette-item) {
    display: flex;
    align-items: center;
    padding: 10px 16px;
    cursor: pointer;
    font-size: 14px;
    color: var(--text-secondary, #656d76);
    user-select: none;
    outline: none;
    contain: layout style paint;
  }

  :global(.quick-open-container .palette-item:hover),
  :global(.quick-open-container .palette-item[data-selected]),
  :global(.quick-open-container .palette-item[data-highlighted]) {
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.1));
    color: var(--text-primary, #1f2328);
  }

  :global(.quick-open-container .file-icon) {
    width: 16px;
    height: 16px;
    margin-right: 10px;
    color: var(--text-secondary, #656d76);
    flex-shrink: 0;
  }

  :global(.quick-open-container .item-name) {
    font-weight: 600;
    color: var(--text-primary, #1f2328);
    margin-right: 8px;
    white-space: nowrap;
  }

  :global(.quick-open-container .item-path) {
    font-size: 12px;
    color: var(--text-secondary, #656d76);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.75;
  }

  :global(.quick-open-container .quick-open-footer) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 8px 16px;
    background-color: var(--footer-bg, rgba(0, 0, 0, 0.03));
    border-top: 1px solid var(--border-primary, #d0d7de);
    font-size: 11px;
    color: var(--text-secondary, #656d76);
    flex-shrink: 0;
    margin-top: auto;
  }

  :global(.quick-open-container .footer-shortcuts) {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  :global(.quick-open-container .footer-stats) {
    display: flex;
    align-items: center;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary, #656d76);
    opacity: 0.85;
    user-select: none;
    white-space: nowrap;
  }

  :global(.quick-open-container .quick-open-footer kbd) {
    font-family: var(--mono, monospace);
    background: var(--bg-secondary, #f6f8fa);
    padding: 1px 4px;
    border-radius: 3px;
    color: var(--text-primary, #1f2328);
    border: 1px solid var(--border-primary, #d0d7de);
  }

  :global(.quick-open-container .scroll-more-indicator) {
    padding: 10px 16px;
    text-align: center;
    font-size: 11px;
    color: var(--text-secondary, #656d76);
    border-top: 1px dashed var(--border-primary, #d0d7de);
    background: var(--bg-secondary, #f6f8fa);
    user-select: none;
  }

  :global(.quick-open-container .search-box-wrapper.is-fts) {
    border-bottom: 2px solid var(--accent, #0969da);
  }

  :global(.quick-open-container .fts-active-icon) {
    color: var(--accent, #0969da) !important;
  }

  :global(.quick-open-container .fts-indicator-badge) {
    font-size: 10px;
    font-weight: 700;
    font-family: var(--mono, monospace);
    color: var(--accent, #0969da);
    background: rgba(9, 105, 218, 0.12);
    padding: 2px 5px;
    border-radius: 4px;
    border: 1px solid rgba(9, 105, 218, 0.25);
    flex-shrink: 0;
  }

  :global(.quick-open-container .quick-fts-spinner) {
    width: 13px;
    height: 13px;
    border: 2px solid var(--border-primary, #d0d7de);
    border-top-color: var(--accent, #0969da);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
    flex-shrink: 0;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .fts-mode-hint {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    color: var(--text-secondary, #656d76);
  }

  .fts-hint-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 600;
    color: var(--accent, #0969da);
  }

  .fts-hint-desc {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
  }

  .fts-hint-desc code,
  .fts-hint-tags code {
    font-family: var(--mono, monospace);
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    padding: 1px 4px;
    border-radius: 3px;
    color: var(--text-primary, #1f2328);
  }

  .fts-hint-tags {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    flex-wrap: wrap;
    margin-top: 4px;
  }

  .quick-fts-main {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
    min-width: 0;
  }

  .quick-fts-header {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .quick-fts-kind {
    font-size: 10px;
    text-transform: uppercase;
    font-weight: 600;
    padding: 1px 5px;
    border-radius: 3px;
    letter-spacing: 0.03em;
    flex-shrink: 0;
  }

  .quick-fts-kind.markdown {
    background: rgba(9, 105, 218, 0.12);
    color: var(--accent, #0969da);
  }

  .quick-fts-kind.mermaid {
    background: rgba(227, 98, 9, 0.12);
    color: #e36209;
  }

  .quick-fts-kind.code {
    background: rgba(130, 80, 223, 0.12);
    color: #8250df;
  }

  .quick-fts-kind.excalidraw {
    background: rgba(26, 127, 55, 0.12);
    color: #1a7f37;
  }

  .quick-fts-snippet {
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-secondary, #656d76);
    padding-left: 24px;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    text-overflow: ellipsis;
    word-break: break-word;
  }

  :global(.quick-open-container mark.quick-fts-mark) {
    background-color: var(--search-match-bg, rgba(234, 179, 8, 0.35));
    color: inherit;
    font-weight: 600;
    border-radius: 2px;
    padding: 0 1px;
  }

  :global(.quick-open-container .folder-active-icon) {
    color: var(--accent, #0969da) !important;
  }

  :global(.quick-open-container .folder-indicator-badge) {
    font-size: 10px;
    font-weight: 700;
    font-family: var(--mono, monospace);
    color: var(--accent, #0969da);
    background: rgba(9, 105, 218, 0.12);
    padding: 2px 5px;
    border-radius: 4px;
    border: 1px solid rgba(9, 105, 218, 0.25);
    flex-shrink: 0;
  }

  :global(.quick-open-container .folder-palette-item) {
    gap: 8px;
  }

  :global(.quick-open-container .folder-item-icon) {
    color: var(--accent, #0969da);
    flex-shrink: 0;
    margin-right: 2px;
  }

  :global(.quick-open-container .folder-name-container) {
    display: flex;
    align-items: baseline;
    gap: 6px;
    overflow: hidden;
  }

  :global(.quick-open-container .folder-title) {
    font-weight: 600;
    color: var(--text-primary, #1f2328);
  }

  :global(.quick-open-container .folder-parent-path) {
    font-size: 11px;
    color: var(--text-secondary, #656d76);
    opacity: 0.7;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  :global(.quick-open-container .folder-tag-pill) {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 9999px;
    background: var(--bg-tertiary, #eaeef2);
    color: var(--text-secondary, #656d76);
    font-weight: 500;
  }

  :global(.quick-open-container .folder-meta-badge) {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--text-secondary, #656d76);
  }

  :global(.quick-open-container .folder-notes-count) {
    font-size: 11px;
    opacity: 0.8;
    white-space: nowrap;
  }

  :global(.quick-open-container .folder-reveal-btn) {
    border: 1px solid var(--border-primary, #d0d7de);
    background: var(--bg-primary, #ffffff);
    color: var(--text-secondary, #656d76);
    border-radius: 3px;
    font-size: 10px;
    padding: 1px 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  :global(.quick-open-container .folder-reveal-btn:hover) {
    background: var(--accent-bg, rgba(9, 105, 218, 0.1));
    color: var(--accent, #0969da);
    border-color: var(--accent, #0969da);
  }

  :global(.quick-open-container .toggle-folders-item) {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 16px 6px 36px;
    font-size: 12px;
    font-weight: 500;
    color: var(--accent, #0969da);
    cursor: pointer;
    user-select: none;
    border-radius: 4px;
    margin: 2px 8px;
    transition: background-color 0.12s ease;
  }

  :global(.quick-open-container .toggle-folders-item:hover),
  :global(.quick-open-container .toggle-folders-item[data-selected]),
  :global(.quick-open-container .toggle-folders-item[data-highlighted]) {
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.1));
  }

  :global(.quick-open-container .toggle-folders-icon) {
    color: var(--accent, #0969da);
    flex-shrink: 0;
  }

  :global(.quick-open-container .toggle-folders-label) {
    color: var(--accent, #0969da);
  }

  /* Estilos para badges e indicadores de etiquetas */
  :global(.quick-open-container .tag-indicator-badge) {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.05em;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(130, 80, 223, 0.15);
    color: var(--accent-purple, #8250df);
    border: 1px solid rgba(130, 80, 223, 0.3);
  }

  :global(.quick-open-container .tag-active-icon) {
    color: var(--accent-purple, #8250df);
  }

  .tag-suggestions-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 16px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted, #8c959f);
  }

  :global(.quick-open-container .tag-suggestion-item) {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
  }

  :global(.quick-open-container .tag-suggestion-item .tag-name) {
    font-weight: 600;
    color: var(--text-primary, #1f2328);
    font-family: var(--mono, monospace);
    font-size: 13px;
  }

  :global(.quick-open-container .tag-suggestion-item .tag-count) {
    margin-left: auto;
    font-size: 11px;
    color: var(--text-muted, #8c959f);
  }

  .item-tags-container {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-left: 6px;
    flex-shrink: 0;
  }

  .item-tag-pill {
    background: var(--bg-tertiary, #eaeef2);
    color: var(--text-secondary, #656d76);
    border: 1px solid var(--border-secondary, #d8dee4);
    font-size: 10.5px;
    font-family: var(--mono, monospace);
    padding: 1px 6px;
    border-radius: 10px;
    cursor: pointer;
    line-height: 1.2;
    transition: background-color 0.12s ease, border-color 0.12s ease, color 0.12s ease;
  }

  .item-tag-pill:hover {
    background: var(--accent, #0969da);
    color: #ffffff;
    border-color: var(--accent, #0969da);
  }
</style>
