<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { Marked } from 'marked';
  import { Search, ChevronUp, ChevronDown, X } from 'lucide-svelte';
  import { resolveIncludes } from '../lib/include-resolver';
  import { appSettings } from '@entities/settings';
  import { renderUnifiedDiagramSvg, renderExcalidrawToSvg } from '@shared/lib/diagrams';
  import { vaultRepository } from '@shared/repositories';
  import { resolveVaultImageUrl, parseImageDimensions } from '../lib/image-resolver';
  import { resolveMarkdownWikilinks } from '../lib/link-resolver';

  interface Props {
    content: string;
    filePath?: string | null;
    scrollToTerms?: string[];
    onNavigate?: (path: string) => void;
  }

  let { content = '', filePath = null, scrollToTerms = [], onNavigate }: Props = $props();

  let containerRef = $state<HTMLDivElement | null>(null);
  let renderedHtml = $state('');

  // Estado del panel de búsqueda en modo lectura
  let showSearchBar = $state(false);
  let searchQuery = $state('');
  let caseSensitive = $state(false);
  let matchCount = $state(0);
  let currentMatchIndex = $state(0);
  let searchInputRef = $state<HTMLInputElement | null>(null);
  let matchesNodes = $state<HTMLElement[]>([]);

  export function triggerSearch() {
    showSearchBar = true;
    tick().then(() => {
      searchInputRef?.focus();
      searchInputRef?.select();
    });
  }

  export function closeSearch() {
    showSearchBar = false;
    searchQuery = '';
    clearHighlights();
  }

  function clearHighlights() {
    if (!containerRef) return;
    const marks = containerRef.querySelectorAll('mark.reading-search-highlight');
    marks.forEach((mark) => {
      const parent = mark.parentNode;
      if (!parent) return;
      while (mark.firstChild) {
        parent.insertBefore(mark.firstChild, mark);
      }
      parent.removeChild(mark);
      parent.normalize();
    });
    matchesNodes = [];
    matchCount = 0;
    currentMatchIndex = 0;
  }

  function performSearch() {
    clearHighlights();
    const query = searchQuery.trim();
    if (!containerRef || !query) return;

    const article = containerRef.querySelector('.markdown-body');
    if (!article) return;

    // Buscar en los nodos de texto de .markdown-body ignorando scripts/estilos
    const walker = document.createTreeWalker(
      article,
      NodeFilter.SHOW_TEXT,
      {
        acceptNode(node) {
          const parent = node.parentElement;
          if (!parent) return NodeFilter.FILTER_REJECT;
          const tagName = parent.tagName.toUpperCase();
          if (tagName === 'SCRIPT' || tagName === 'STYLE') {
            return NodeFilter.FILTER_REJECT;
          }
          if (parent.closest('.reading-search-bar')) {
            return NodeFilter.FILTER_REJECT;
          }
          return NodeFilter.FILTER_ACCEPT;
        }
      }
    );

    const nodesToReplace: { node: Text; matches: { start: number; end: number }[] }[] = [];
    let textNode: Node | null = walker.nextNode();

    while (textNode) {
      const text = textNode.textContent || '';
      const flags = caseSensitive ? 'g' : 'gi';
      const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      const regex = new RegExp(escaped, flags);
      const matches: { start: number; end: number }[] = [];
      let m: RegExpExecArray | null;

      while ((m = regex.exec(text)) !== null) {
        matches.push({ start: m.index, end: m.index + m[0].length });
      }

      if (matches.length > 0) {
        nodesToReplace.push({ node: textNode as Text, matches });
      }

      textNode = walker.nextNode();
    }

    const createdMarks: HTMLElement[] = [];

    for (const { node, matches } of nodesToReplace) {
      const parent = node.parentNode;
      if (!parent) continue;

      const fullText = node.textContent || '';
      const fragment = document.createDocumentFragment();
      let lastIdx = 0;

      for (const { start, end } of matches) {
        if (start > lastIdx) {
          fragment.appendChild(document.createTextNode(fullText.substring(lastIdx, start)));
        }
        const mark = document.createElement('mark');
        mark.className = 'reading-search-highlight';
        mark.textContent = fullText.substring(start, end);
        fragment.appendChild(mark);
        createdMarks.push(mark);
        lastIdx = end;
      }

      if (lastIdx < fullText.length) {
        fragment.appendChild(document.createTextNode(fullText.substring(lastIdx)));
      }

      parent.replaceChild(fragment, node);
    }

    matchesNodes = createdMarks;
    matchCount = createdMarks.length;
    currentMatchIndex = createdMarks.length > 0 ? 1 : 0;

    if (createdMarks.length > 0) {
      updateActiveHighlight();
    }
  }

  function updateActiveHighlight() {
    matchesNodes.forEach((mark, idx) => {
      if (idx === currentMatchIndex - 1) {
        mark.classList.add('reading-search-active');
        mark.scrollIntoView({ behavior: 'smooth', block: 'center' });
      } else {
        mark.classList.remove('reading-search-active');
      }
    });
  }

  function findNext() {
    if (matchCount === 0) return;
    currentMatchIndex = currentMatchIndex >= matchCount ? 1 : currentMatchIndex + 1;
    updateActiveHighlight();
  }

  function findPrevious() {
    if (matchCount === 0) return;
    currentMatchIndex = currentMatchIndex <= 1 ? matchCount : currentMatchIndex - 1;
    updateActiveHighlight();
  }

  function handleSearchKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      closeSearch();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      if (e.shiftKey) {
        findPrevious();
      } else {
        findNext();
      }
    }
  }

  function handleWrapperKeyDown(e: KeyboardEvent) {
    const isFind = (e.ctrlKey || e.metaKey) && (e.key === 'f' || e.key === 'F');
    if (isFind) {
      e.preventDefault();
      e.stopPropagation();
      triggerSearch();
    }
  }

  const marked = new Marked();

  marked.use({
    renderer: {
      link({ href, title, text }: { href: string; title?: string | null; text: string }) {
        const safeHref = href || '';
        const isExternal = /^(?:https?:\/\/|mailto:|tel:)/i.test(safeHref);
        return `<a href="${escapeHtml(safeHref)}" class="${isExternal ? 'external-link' : 'internal-link'}" ${isExternal ? 'target="_blank" rel="noopener noreferrer"' : ''} ${title ? `title="${escapeHtml(title)}"` : ''} data-internal-path="${escapeHtml(safeHref)}">${text}</a>`;
      },
      image({ href, title, text }: { href: string; title?: string | null; text: string }) {
        const { cleanAlt, width, height } = parseImageDimensions(text || '');
        const style = [
          width ? `width: ${width};` : '',
          height ? `height: ${height};` : '',
        ].filter(Boolean).join(' ');

        let cleanHref = href || '';
        try {
          cleanHref = decodeURI(cleanHref);
        } catch {
          // Mantener original si falla
        }

        const isReady = /^(?:https?:\/\/|data:|blob:|asset:\/\/)/i.test(cleanHref);

        return `<figure class="reading-image-figure" data-raw-src="${escapeHtml(cleanHref)}" data-alt="${escapeHtml(cleanAlt)}" data-title="${escapeHtml(title || '')}">
          <div class="reading-image-container">
            <img class="reading-image-el" ${isReady ? `src="${escapeHtml(cleanHref)}"` : ''} alt="${escapeHtml(cleanAlt)}" ${title ? `title="${escapeHtml(title)}"` : ''} ${style ? `style="${style}"` : ''} loading="lazy" />
          </div>
          ${cleanAlt ? `<figcaption class="reading-image-caption">${escapeHtml(cleanAlt)}</figcaption>` : ''}
        </figure>`;
      },
      code({ text, lang }: { text: string; lang?: string }) {
        const cleanLang = (lang || '').trim().toLowerCase();
        if (/^(?:mermaid|mermair|mermai|merman)$/i.test(cleanLang)) {
          return `<div class="reading-mermaid-card" data-code="${encodeURIComponent(text)}" data-engine="mermaid">
            <div class="reading-mermaid-header">
              <span class="reading-mermaid-badge">Diagrama Mermaid</span>
            </div>
            <div class="reading-mermaid-body">
              <span class="reading-mermaid-loading">Cargando diagrama...</span>
            </div>
          </div>`;
        }
        if (cleanLang === 'd2') {
          return `<div class="reading-mermaid-card" data-code="${encodeURIComponent(text)}" data-engine="d2">
            <div class="reading-mermaid-header">
              <span class="reading-mermaid-badge">Diagrama D2</span>
            </div>
            <div class="reading-mermaid-body">
              <span class="reading-mermaid-loading">Cargando diagrama...</span>
            </div>
          </div>`;
        }
        if (cleanLang === 'excalidraw') {
          return `<div class="reading-excalidraw-card" data-code="${encodeURIComponent(text)}">
            <div class="reading-mermaid-header">
              <span class="reading-mermaid-badge">Dibujo Excalidraw</span>
            </div>
            <div class="reading-excalidraw-body">
              <span class="reading-mermaid-loading">Cargando dibujo Excalidraw...</span>
            </div>
          </div>`;
        }
        return false;
      },
    },
  });

  async function renderImages() {
    if (!containerRef) return;
    const isDark = typeof document !== 'undefined' && document.documentElement.getAttribute('data-theme') === 'dark';
    const figures = containerRef.querySelectorAll<HTMLElement>('.reading-image-figure');
    for (const fig of figures) {
      const rawSrc = fig.getAttribute('data-raw-src') || '';
      if (!rawSrc) continue;

      const cleanSrcLower = rawSrc.toLowerCase().split('?')[0].split('#')[0];
      const isExcalidraw = /\.(?:excalidraw|excalidraw\.json|excalidraw\.md)$/i.test(cleanSrcLower);

      if (isExcalidraw) {
        // Renderizado especial de Excalidraw embebido con caché
        const container = fig.querySelector<HTMLElement>('.reading-image-container');
        if (!container) continue;

        try {
          let targetPath = rawSrc.trim().replace(/^['"]|['"]$/g, '').split('#')[0].trim();
          if (filePath && !targetPath.startsWith('/')) {
            const lastSlash = filePath.lastIndexOf('/');
            const baseDir = lastSlash >= 0 ? filePath.slice(0, lastSlash) : '';
            targetPath = baseDir ? `${baseDir}/${targetPath}` : targetPath;
          }
          targetPath = targetPath.replace(/^\/+/, '');

          const note = await vaultRepository.readNote(targetPath);
          if (note && note.content) {
            const svgString = await renderExcalidrawToSvg(note.content, targetPath, isDark);
            if (svgString) {
              container.innerHTML = `<div class="reading-excalidraw-svg-wrapper">${svgString}</div>`;
              continue;
            }
          }
        } catch (err) {
          console.warn('Error renderizando Excalidraw en modo lectura:', err);
        }
      }

      const img = fig.querySelector<HTMLImageElement>('.reading-image-el');
      if (!img) continue;

      try {
        const resolvedUrl = await resolveVaultImageUrl(rawSrc, filePath);
        if (resolvedUrl && img.src !== resolvedUrl) {
          img.src = resolvedUrl;
        }
      } catch (err) {
        console.warn('Error resolviendo imagen en modo lectura:', err);
      }
    }
  }

  async function renderDiagrams() {
    if (!containerRef) return;
    const isDark = typeof document !== 'undefined' && document.documentElement.getAttribute('data-theme') === 'dark';

    // 1. Renderizar tarjetas de bloques ```excalidraw
    const excalidrawCards = containerRef.querySelectorAll<HTMLDivElement>('.reading-excalidraw-card');
    for (const card of excalidrawCards) {
      const rawCode = card.getAttribute('data-code');
      if (!rawCode) continue;
      const code = decodeURIComponent(rawCode);
      const body = card.querySelector('.reading-excalidraw-body');
      if (!body) continue;

      try {
        const resolvedCode = await resolveIncludes(code, filePath);
        const svgString = await renderExcalidrawToSvg(resolvedCode, filePath || undefined, isDark);
        if (svgString) {
          body.innerHTML = `<div class="reading-excalidraw-svg-wrapper">${svgString}</div>`;
        } else {
          body.innerHTML = `<div class="reading-mermaid-empty">Dibujo Excalidraw vacío</div>`;
        }
      } catch (err: any) {
        body.innerHTML = `
          <div class="reading-mermaid-error">
            <div class="error-badge">Error al renderizar Excalidraw</div>
            <pre>${escapeHtml(String(err?.message || err))}</pre>
          </div>
        `;
      }
    }

    // 2. Renderizar diagramas Mermaid y D2
    const cards = containerRef.querySelectorAll<HTMLDivElement>('.reading-mermaid-card');
    for (const card of cards) {
      const rawCode = card.getAttribute('data-code');
      if (!rawCode) continue;
      const engine = card.getAttribute('data-engine') === 'd2' ? 'd2' : appSettings.mermaidRenderer;
      const code = decodeURIComponent(rawCode);
      const body = card.querySelector('.reading-mermaid-body');
      if (!body) continue;

      try {
        const resolvedCode = await resolveIncludes(code, filePath);
        const { svg, error } = await renderUnifiedDiagramSvg(resolvedCode, 'cm-diagram', engine);
        if (error) {
          body.innerHTML = `
            <div class="reading-mermaid-error">
              <div class="error-badge">Error de sintaxis Mermaid</div>
              <pre>${escapeHtml(error)}</pre>
            </div>
          `;
        } else if (svg) {
          body.innerHTML = `<div class="reading-mermaid-svg">${svg}</div>`;
        } else {
          body.innerHTML = `<div class="reading-mermaid-empty">Diagrama vacío</div>`;
        }
      } catch (e) {
        body.innerHTML = `
          <div class="reading-mermaid-error">
            <div class="error-badge">Error al procesar diagrama</div>
            <pre>${escapeHtml(String(e instanceof Error ? e.message : e))}</pre>
          </div>
        `;
      }
    }
  }

  function escapeHtml(str: unknown): string {
    if (typeof str !== 'string') return '';
    return str
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;');
  }

  onMount(() => {
    renderDiagrams();
    renderImages();
  });

  export function scrollToMatch(terms: string[]) {
    if (!containerRef || !terms || terms.length === 0) return;
    for (const term of terms) {
      const cleanTerm = term.trim().toLowerCase();
      if (!cleanTerm) continue;
      const walker = document.createTreeWalker(containerRef, NodeFilter.SHOW_TEXT);
      let node: Node | null = walker.nextNode();
      while (node) {
        if (node.textContent && node.textContent.toLowerCase().includes(cleanTerm)) {
          const parent = node.parentElement;
          if (parent) {
            parent.scrollIntoView({ behavior: 'smooth', block: 'center' });
            return;
          }
        }
        node = walker.nextNode();
      }
    }
  }

  function handleContainerClick(event: MouseEvent) {
    const target = (event.target as HTMLElement).closest('a');
    if (!target) return;

    const href = target.getAttribute('href');
    if (!href) return;

    const isExternal = /^(?:https?:\/\/|mailto:|tel:)/i.test(href);
    if (!isExternal) {
      event.preventDefault();
      event.stopPropagation();
      const internalPath = target.getAttribute('data-internal-path') || href;
      const decoded = decodeURI(internalPath);
      if (onNavigate) {
        onNavigate(decoded);
      }
    }
  }

  $effect(() => {
    const raw = content;
    let isCurrent = true;

    resolveMarkdownWikilinks(raw, filePath).then(async (transformed) => {
      if (!isCurrent) return;
      try {
        const parsed = await marked.parse(transformed);
        renderedHtml = typeof parsed === 'string' ? parsed : '';
      } catch (err) {
        console.error('Error parseando markdown en modo lectura:', err);
        renderedHtml = escapeHtml(transformed);
      }

      await tick();
      if (!isCurrent) return;
      renderDiagrams();
      renderImages();
      if (scrollToTerms && scrollToTerms.length > 0) {
        scrollToMatch(scrollToTerms);
      }
    }).catch((err) => {
      console.error('Error resolviendo wikilinks en modo lectura:', err);
      if (!isCurrent) return;
      try {
        const parsed = marked.parse(raw);
        renderedHtml = typeof parsed === 'string' ? (parsed as string) : '';
      } catch {
        renderedHtml = escapeHtml(raw);
      }
    });

    return () => {
      isCurrent = false;
    };
  });

  $effect(() => {
    if (scrollToTerms && scrollToTerms.length > 0 && renderedHtml) {
      tick().then(() => {
        scrollToMatch(scrollToTerms);
      });
    }
  });
  $effect(() => {
    if (showSearchBar && searchQuery) {
      performSearch();
    }
  });

  $effect(() => {
    // Si se re-renderiza el HTML y la barra de búsqueda está abierta, refrescar
    if (renderedHtml && showSearchBar && searchQuery) {
      tick().then(() => {
        performSearch();
      });
    }
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="markdown-reading-wrapper"
  bind:this={containerRef}
  onkeydown={handleWrapperKeyDown}
  tabindex="-1"
>
  {#if showSearchBar}
    <div class="reading-search-bar" role="search">
      <div class="reading-search-input-wrapper">
        <Search size={14} class="reading-search-icon" />
        <input
          bind:this={searchInputRef}
          type="text"
          class="reading-search-input"
          placeholder="Buscar en el documento..."
          bind:value={searchQuery}
          oninput={() => {
            performSearch();
          }}
          onkeydown={handleSearchKeyDown}
        />
        {#if searchQuery}
          <button
            type="button"
            class="reading-search-icon-btn"
            title="Borrar búsqueda"
            onclick={() => {
              searchQuery = '';
              clearHighlights();
              searchInputRef?.focus();
            }}
          >
            <X size={12} />
          </button>
        {/if}
      </div>

      <div class="reading-search-info">
        {#if searchQuery}
          <span class="reading-search-count">
            {matchCount > 0 ? `${currentMatchIndex} de ${matchCount}` : 'Sin coincidencias'}
          </span>
        {/if}
      </div>

      <div class="reading-search-actions">
        <button
          type="button"
          class="reading-search-btn"
          title="Anterior (Shift+Enter)"
          disabled={matchCount === 0}
          onclick={findPrevious}
        >
          <ChevronUp size={14} />
        </button>
        <button
          type="button"
          class="reading-search-btn"
          title="Siguiente (Enter)"
          disabled={matchCount === 0}
          onclick={findNext}
        >
          <ChevronDown size={14} />
        </button>
        <button
          type="button"
          class="reading-search-btn reading-search-case-btn"
          class:active={caseSensitive}
          title="Coincidir mayúsculas y minúsculas"
          onclick={() => {
            caseSensitive = !caseSensitive;
            performSearch();
          }}
        >
          Aa
        </button>
        <button
          type="button"
          class="reading-search-btn reading-search-close-btn"
          title="Cerrar búsqueda (Esc)"
          onclick={closeSearch}
        >
          <X size={14} />
        </button>
      </div>
    </div>
  {/if}

  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <article class="markdown-body" onclick={handleContainerClick}>
    {#if renderedHtml}
      {@html renderedHtml}
    {:else}
      <p class="empty-doc">Documento vacío</p>
    {/if}
  </article>
</div>

<style>
  .markdown-reading-wrapper {
    width: 100%;
    height: 100%;
    overflow-y: auto;
    padding: 24px 32px;
    box-sizing: border-box;
    background-color: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
    font-family: var(--main-font, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif);
    line-height: 1.7;
    position: relative;
    outline: none;
  }

  /* Barra de búsqueda flotante en vista de lectura */
  .reading-search-bar {
    position: sticky;
    top: 0;
    z-index: 20;
    margin: -16px -24px 16px -24px;
    padding: 6px 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    background-color: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
  }

  .reading-search-input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 160px;
    max-width: 320px;
  }

  :global(.reading-search-icon) {
    position: absolute;
    left: 8px;
    color: var(--text-secondary, #656d76);
    pointer-events: none;
  }

  .reading-search-input {
    width: 100%;
    height: 28px;
    padding: 0 26px 0 28px;
    font-size: 13px;
    font-family: inherit;
    color: var(--text-primary, #1f2328);
    background-color: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 4px;
    outline: none;
    box-sizing: border-box;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .reading-search-input:focus {
    border-color: var(--accent, #0969da);
    box-shadow: 0 0 0 2px rgba(9, 105, 218, 0.2);
  }

  .reading-search-icon-btn {
    position: absolute;
    right: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2px;
    border: none;
    background: transparent;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    border-radius: 3px;
  }

  .reading-search-icon-btn:hover {
    color: var(--text-primary, #1f2328);
    background-color: var(--bg-secondary, #f6f8fa);
  }

  .reading-search-info {
    font-size: 12px;
    color: var(--text-secondary, #656d76);
    min-width: 70px;
    user-select: none;
  }

  .reading-search-count {
    white-space: nowrap;
  }

  .reading-search-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .reading-search-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 26px;
    min-width: 26px;
    padding: 0 6px;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 4px;
    background-color: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
    cursor: pointer;
    font-size: 11px;
    transition: background-color 0.15s ease, border-color 0.15s ease;
  }

  .reading-search-btn:hover:not(:disabled) {
    background-color: var(--bg-secondary, #f6f8fa);
    border-color: var(--text-secondary, #656d76);
  }

  .reading-search-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .reading-search-case-btn {
    font-weight: 600;
  }

  .reading-search-case-btn.active {
    background-color: rgba(9, 105, 218, 0.15);
    border-color: var(--accent, #0969da);
    color: var(--accent, #0969da);
  }

  .reading-search-close-btn:hover {
    color: var(--error-text, #cf222e);
  }

  :global(mark.reading-search-highlight) {
    background-color: rgba(255, 223, 93, 0.45);
    color: inherit;
    border-radius: 2px;
    padding: 1px 2px;
  }

  :global(mark.reading-search-highlight.reading-search-active) {
    background-color: #ff9632 !important;
    color: #000000 !important;
    outline: 2px solid #ff9632;
    border-radius: 2px;
  }

  .markdown-body {
    max-width: 100%;
    margin: 0;
    font-size: 15px;
  }

  .empty-doc {
    color: var(--text-secondary, #656d76);
    font-style: italic;
  }

  :global(.markdown-body h1) {
    font-size: 2em;
    font-weight: 700;
    padding-bottom: 0.3em;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    margin-top: 24px;
    margin-bottom: 16px;
  }

  :global(.markdown-body h2) {
    font-size: 1.5em;
    font-weight: 600;
    padding-bottom: 0.3em;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    margin-top: 24px;
    margin-bottom: 16px;
  }

  :global(.markdown-body h3) {
    font-size: 1.25em;
    font-weight: 600;
    margin-top: 24px;
    margin-bottom: 12px;
  }

  :global(.markdown-body p) {
    margin-top: 0;
    margin-bottom: 16px;
  }

  :global(.markdown-body ul),
  :global(.markdown-body ol) {
    padding-left: 2em;
    margin-top: 0;
    margin-bottom: 16px;
  }

  :global(.markdown-body li) {
    margin-top: 0.25em;
  }

  :global(.markdown-body blockquote) {
    padding: 0 1em;
    color: var(--text-secondary, #656d76);
    border-left: 0.25em solid var(--border-primary, #d0d7de);
    margin: 0 0 16px 0;
  }

  :global(.markdown-body table) {
    border-collapse: collapse;
    width: 100%;
    margin-bottom: 16px;
  }

  :global(.markdown-body th),
  :global(.markdown-body td) {
    padding: 6px 13px;
    border: 1px solid var(--border-primary, #d0d7de);
  }

  :global(.markdown-body th) {
    font-weight: 600;
    background-color: var(--bg-secondary, #f6f8fa);
  }

  :global(.markdown-body pre) {
    background-color: var(--bg-secondary, #f6f8fa);
    border-radius: 6px;
    padding: 16px;
    overflow: auto;
    font-size: 85%;
    line-height: 1.45;
  }

  :global(.markdown-body code) {
    font-family: var(--code-font, ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace);
    font-size: 85%;
    padding: 0.2em 0.4em;
    margin: 0;
    background-color: var(--code-inline-bg, rgba(175, 184, 193, 0.2));
    border-radius: 4px;
  }

  :global(.markdown-body pre code) {
    background-color: transparent;
    padding: 0;
  }

  :global(.markdown-body a) {
    color: var(--accent, #0969da);
    text-decoration: underline;
  }

  :global(.markdown-body a.internal-link) {
    color: var(--accent, #0969da);
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
    font-weight: 500;
    transition: opacity 0.15s ease, text-decoration-color 0.15s ease;
  }

  :global(.markdown-body a.internal-link:hover) {
    opacity: 0.8;
    text-decoration-thickness: 2px;
  }

  /* Figuras e imágenes en modo lectura */
  :global(.reading-image-figure) {
    margin: 20px 0;
    padding: 0;
    text-align: center;
  }

  :global(.reading-image-container) {
    display: inline-block;
    max-width: 100%;
    border-radius: 8px;
    overflow: hidden;
    border: 1px solid var(--border-primary, #d0d7de);
    box-shadow: var(--card-shadow, 0 2px 8px rgba(0, 0, 0, 0.04));
    background-color: var(--bg-primary, #ffffff);
  }

  :global(.reading-image-el) {
    display: block;
    max-width: 100%;
    height: auto;
  }

  :global(.reading-image-caption) {
    margin-top: 8px;
    font-size: 13px;
    color: var(--text-secondary, #656d76);
    font-style: italic;
  }

  :global(.reading-mermaid-card) {
    margin: 20px 0;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    background-color: var(--bg-primary, #ffffff);
    box-shadow: var(--card-shadow, 0 2px 8px rgba(0, 0, 0, 0.04));
    overflow: hidden;
  }

  :global(.reading-mermaid-header) {
    padding: 6px 12px;
    background-color: var(--bg-secondary, #f6f8fa);
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    display: flex;
    align-items: center;
  }

  :global(.reading-mermaid-badge) {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary, #656d76);
  }

  :global(.reading-mermaid-body) {
    padding: 16px;
    display: flex;
    justify-content: center;
    align-items: center;
    min-height: 80px;
  }

  :global(.reading-mermaid-svg) {
    width: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
  }

  :global(.reading-mermaid-svg svg) {
    max-width: 100%;
    height: auto;
  }

  :global(.reading-mermaid-error) {
    width: 100%;
    padding: 10px;
    background-color: var(--error-bg, #ffebe9);
    border: 1px solid var(--error-border, rgba(207, 34, 46, 0.3));
    border-radius: 6px;
    color: var(--error-text, #cf222e);
    font-size: 12px;
  }

  :global(.reading-mermaid-error .error-badge) {
    font-weight: 600;
    margin-bottom: 4px;
  }

  :global(.reading-mermaid-error pre) {
    margin: 0;
    background: transparent;
    padding: 0;
  }

  :global(.reading-mermaid-loading) {
    font-size: 12px;
    color: var(--text-secondary, #656d76);
  }

  :global(.reading-mermaid-empty) {
    font-size: 12px;
    color: var(--text-secondary, #656d76);
    font-style: italic;
  }
</style>
