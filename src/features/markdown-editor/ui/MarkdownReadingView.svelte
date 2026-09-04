<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { Marked } from 'marked';
  import { resolveIncludes } from '../lib/include-resolver';
  import { renderUnifiedDiagramSvg } from '../lib/render-diagram';

  interface Props {
    content: string;
    filePath?: string | null;
  }

  let { content = '', filePath = null }: Props = $props();

  let containerRef = $state<HTMLDivElement | null>(null);
  let renderedHtml = $state('');

  const marked = new Marked();

  marked.use({
    renderer: {
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
            <pre>${escapeHtml(String(e))}</pre>
          </div>
        `;
      }
    }
  }

  function escapeHtml(str: string): string {
    return str
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;');
  }

  onMount(() => {
    renderDiagrams();
  });

  $effect(() => {
    const raw = content;
    const parsed = marked.parse(raw);
    renderedHtml = typeof parsed === 'string' ? parsed : '';

    tick().then(() => {
      renderDiagrams();
    });
  });
</script>

<div class="markdown-reading-wrapper" bind:this={containerRef}>
  <article class="markdown-body">
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
    max-width: 860px;
    margin: 0 auto;
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
    background-color: rgba(175, 184, 193, 0.2);
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

  :global(.reading-mermaid-card) {
    margin: 20px 0;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    background-color: var(--bg-primary, #ffffff);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04);
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
    background-color: #ffebe9;
    border-radius: 6px;
    color: #cf222e;
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
