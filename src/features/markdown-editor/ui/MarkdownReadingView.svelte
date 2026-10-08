<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { Marked } from 'marked';
  import { resolveIncludes } from '../lib/include-resolver';
  import { renderUnifiedDiagramSvg } from '../lib/render-diagram';
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
        if (/^(?:mermaid|mermair|mermai|merman)$/i.test(lang?.trim() || '')) {
          return `<div class="reading-mermaid-card" data-code="${encodeURIComponent(text)}">
            <div class="reading-mermaid-header">
              <span class="reading-mermaid-badge">Diagrama Mermaid</span>
            </div>
            <div class="reading-mermaid-body">
              <span class="reading-mermaid-loading">Cargando diagrama...</span>
            </div>
          </div>`;
        }
        return false;
      },
    },
  });

  async function renderImages() {
    if (!containerRef) return;
    const figures = containerRef.querySelectorAll<HTMLElement>('.reading-image-figure');
    for (const fig of figures) {
      const rawSrc = fig.getAttribute('data-raw-src') || '';
      if (!rawSrc) continue;
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
    const cards = containerRef.querySelectorAll<HTMLDivElement>('.reading-mermaid-card');
    for (const card of cards) {
      const rawCode = card.getAttribute('data-code');
      if (!rawCode) continue;
      const code = decodeURIComponent(rawCode);
      const body = card.querySelector('.reading-mermaid-body');
      if (!body) continue;

      try {
        const resolvedCode = await resolveIncludes(code, filePath);
        const { svg, error } = await renderUnifiedDiagramSvg(resolvedCode);
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
</script>

<div class="markdown-reading-wrapper" bind:this={containerRef}>
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
