<script lang="ts">
  import {
    ChevronDown,
    ChevronRight,
    Tag,
    Calendar,
    Link as LinkIcon,
    FileText,
    User,
    CheckCircle2,
    XCircle,
    Copy,
    Check,
  } from 'lucide-svelte';
  import type { PropertiesViewMode } from '@entities/settings';

  interface Props {
    metadata: Record<string, any>;
    defaultMode?: PropertiesViewMode;
    onNavigate?: (path: string) => void;
    onTagClick?: (tag: string) => void;
  }

  let {
    metadata = {},
    defaultMode = 'expanded',
    onNavigate,
    onTagClick,
  }: Props = $props();

  let isUserToggled = $state<boolean | null>(null);
  let isCollapsed = $derived(isUserToggled !== null ? isUserToggled : defaultMode === 'collapsed');
  let copiedKey = $state<string | null>(null);

  // Filtrar campos internos o nulos
  let visibleEntries = $derived(
    Object.entries(metadata).filter(([key, value]) => {
      if (key.startsWith('_')) return false;
      if (value === null || value === undefined || value === '') return false;
      if (Array.isArray(value) && value.length === 0) return false;
      return true;
    })
  );

  function isTagKey(key: string): boolean {
    const k = key.toLowerCase();
    return k === 'tag' || k === 'tags' || k === 'etiqueta' || k === 'etiquetas';
  }

  function isDateKey(key: string): boolean {
    const k = key.toLowerCase();
    return (
      k.includes('date') ||
      k.includes('fecha') ||
      k.includes('created') ||
      k.includes('updated') ||
      k.includes('modified')
    );
  }

  function isAuthorKey(key: string): boolean {
    const k = key.toLowerCase();
    return k === 'author' || k === 'autor' || k === 'creator';
  }

  function isAliasKey(key: string): boolean {
    const k = key.toLowerCase();
    return k === 'alias' || k === 'aliases';
  }

  function isLinkValue(val: string): boolean {
    return /^https?:\/\//i.test(val) || /^\[\[.+\]\]$/.test(val);
  }

  function extractWikilinkTarget(val: string): string {
    const clean = val.replace(/^\[\[/, '').replace(/\]\]$/, '');
    const parts = clean.split('|');
    return parts[0].trim();
  }

  function extractWikilinkLabel(val: string): string {
    const clean = val.replace(/^\[\[/, '').replace(/\]\]$/, '');
    const parts = clean.split('|');
    return (parts[1] || parts[0]).trim();
  }

  async function copyValue(key: string, val: any) {
    try {
      const text = typeof val === 'object' ? JSON.stringify(val, null, 2) : String(val);
      await navigator.clipboard.writeText(text);
      copiedKey = key;
      setTimeout(() => {
        if (copiedKey === key) copiedKey = null;
      }, 1500);
    } catch {
      // Fallback silencioso
    }
  }

  function handleTagClick(event: MouseEvent, tag: string) {
    event.stopPropagation();
    const cleanTag = tag.replace(/^#/, '');
    if (onTagClick) {
      onTagClick(cleanTag);
    }
  }

  function handleLinkClick(event: MouseEvent, urlOrPath: string) {
    if (urlOrPath.startsWith('http://') || urlOrPath.startsWith('https://')) {
      return; // Navegación estándar mediante <a href target="_blank">
    }
    event.preventDefault();
    if (onNavigate) {
      onNavigate(urlOrPath);
    }
  }
</script>

{#if visibleEntries.length > 0 && defaultMode !== 'hidden'}
  <div class="note-properties-card">
    <button
      type="button"
      class="note-properties-header"
      onclick={() => (isUserToggled = !isCollapsed)}
      aria-expanded={!isCollapsed}
    >
      <div class="header-left">
        {#if isCollapsed}
          <ChevronRight size={15} class="header-icon" />
        {:else}
          <ChevronDown size={15} class="header-icon" />
        {/if}
        <span class="header-title">Propiedades</span>
        <span class="header-badge">{visibleEntries.length}</span>
      </div>
      <span class="header-state-hint">
        {isCollapsed ? 'Mostrar' : 'Ocultar'}
      </span>
    </button>

    {#if !isCollapsed}
      <div class="note-properties-content">
        <dl class="properties-grid">
          {#each visibleEntries as [key, value] (key)}
            <dt class="property-key" title={key}>
              <span class="property-icon">
                {#if isTagKey(key)}
                  <Tag size={13} />
                {:else if isDateKey(key)}
                  <Calendar size={13} />
                {:else if isAuthorKey(key)}
                  <User size={13} />
                {:else if isAliasKey(key)}
                  <LinkIcon size={13} />
                {:else}
                  <FileText size={13} />
                {/if}
              </span>
              <span class="property-key-label">{key}</span>
            </dt>

            <dd class="property-value">
              <div class="property-value-body">
                {#if isTagKey(key) || (Array.isArray(value) && isTagKey(key))}
                  <!-- Etiquetas / Tags -->
                  <div class="tags-container">
                    {#each (Array.isArray(value) ? value : [value]) as tagItem}
                      <button
                        type="button"
                        class="tag-pill"
                        onclick={(e) => handleTagClick(e, String(tagItem))}
                        title="Buscar notas con esta etiqueta"
                      >
                        <span class="tag-hash">#</span>{String(tagItem).replace(/^#/, '')}
                      </button>
                    {/each}
                  </div>
                {:else if Array.isArray(value)}
                  <!-- Array genérico -->
                  <div class="pills-container">
                    {#each value as item}
                      {#if typeof item === 'string' && isLinkValue(item)}
                        {#if item.startsWith('http')}
                          <a href={item} target="_blank" rel="noopener noreferrer" class="link-pill">
                            {item}
                          </a>
                        {:else}
                          <button
                            type="button"
                            class="link-pill internal"
                            onclick={(e) => handleLinkClick(e, extractWikilinkTarget(item))}
                          >
                            {extractWikilinkLabel(item)}
                          </button>
                        {/if}
                      {:else}
                        <span class="generic-pill">{String(item)}</span>
                      {/if}
                    {/each}
                  </div>
                {:else if typeof value === 'boolean'}
                  <!-- Valor Booleano -->
                  <div class="boolean-badge" class:is-true={value} class:is-false={!value}>
                    {#if value}
                      <CheckCircle2 size={13} />
                      <span>Verdadero</span>
                    {:else}
                      <XCircle size={13} />
                      <span>Falso</span>
                    {/if}
                  </div>
                {:else if typeof value === 'string' && isLinkValue(value)}
                  <!-- Enlace URL o Wikilink -->
                  {#if value.startsWith('http')}
                    <a href={value} target="_blank" rel="noopener noreferrer" class="value-link">
                      {value}
                    </a>
                  {:else}
                    <button
                      type="button"
                      class="value-link internal"
                      onclick={(e) => handleLinkClick(e, extractWikilinkTarget(value))}
                    >
                      {extractWikilinkLabel(value)}
                    </button>
                  {/if}
                {:else if typeof value === 'object'}
                  <!-- Objeto JSON / estructura anidada -->
                  <pre class="object-preview">{JSON.stringify(value, null, 2)}</pre>
                {:else}
                  <!-- String, número u otro valor simple -->
                  <span class="simple-value" title={String(value)}>{String(value)}</span>
                {/if}
              </div>

              <button
                type="button"
                class="copy-btn"
                title="Copiar valor"
                onclick={() => copyValue(key, value)}
              >
                {#if copiedKey === key}
                  <Check size={12} class="copy-success-icon" />
                {:else}
                  <Copy size={12} />
                {/if}
              </button>
            </dd>
          {/each}
        </dl>
      </div>
    {/if}
  </div>
{/if}

<style>
  .note-properties-card {
    margin-bottom: 24px;
    border: 1px solid var(--border-secondary, #e1e4e8);
    border-radius: 8px;
    background-color: var(--bg-secondary, #f6f8fa);
    overflow: hidden;
    transition: border-color 0.15s ease;
  }

  .note-properties-card:hover {
    border-color: var(--border-primary, #d0d7de);
  }

  .note-properties-header {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    font-family: inherit;
    color: var(--text-secondary, #656d76);
    user-select: none;
    transition: background-color 0.1s ease;
  }

  .note-properties-header:hover {
    background-color: rgba(0, 0, 0, 0.03);
    color: var(--text-primary, #1f2328);
  }

  :global(.dark) .note-properties-header:hover {
    background-color: rgba(255, 255, 255, 0.04);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .header-title {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .header-badge {
    font-size: 11px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 10px;
    background-color: var(--bg-tertiary, #eaeef2);
    color: var(--text-secondary, #656d76);
  }

  .header-state-hint {
    font-size: 11px;
    opacity: 0.65;
  }

  .note-properties-content {
    border-top: 1px solid var(--border-secondary, #e1e4e8);
    padding: 10px 14px 12px 14px;
    background-color: var(--bg-primary, #ffffff);
  }

  .properties-grid {
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: minmax(110px, 160px) 1fr;
    row-gap: 8px;
    column-gap: 16px;
    align-items: center;
  }

  .property-key {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary, #656d76);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .property-icon {
    display: inline-flex;
    align-items: center;
    opacity: 0.7;
    flex-shrink: 0;
  }

  .property-key-label {
    text-transform: capitalize;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .property-value {
    margin: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    font-size: 13px;
    min-width: 0;
  }

  .property-value-body {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
  }

  .simple-value {
    color: var(--text-primary, #1f2328);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Tags */
  .tags-container {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .tag-pill {
    display: inline-flex;
    align-items: center;
    padding: 2px 8px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--accent, #0969da);
    background-color: rgba(9, 105, 218, 0.08);
    border: 1px solid rgba(9, 105, 218, 0.2);
    border-radius: 12px;
    cursor: pointer;
    transition: all 0.12s ease;
    font-family: inherit;
  }

  .tag-pill:hover {
    background-color: rgba(9, 105, 218, 0.15);
    border-color: var(--accent, #0969da);
  }

  .tag-hash {
    opacity: 0.6;
    margin-right: 1px;
  }

  /* Arrays genéricos y enlaces */
  .pills-container {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .generic-pill {
    padding: 2px 7px;
    font-size: 12px;
    background-color: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 4px;
    color: var(--text-primary, #1f2328);
  }

  .link-pill,
  .value-link {
    font-size: 12.5px;
    color: var(--accent, #0969da);
    text-decoration: none;
    border-radius: 4px;
    display: inline-flex;
    align-items: center;
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 1px 4px;
    font-family: inherit;
  }

  .link-pill:hover,
  .value-link:hover {
    text-decoration: underline;
  }

  .link-pill.internal,
  .value-link.internal {
    background-color: rgba(9, 105, 218, 0.05);
  }

  /* Booleano */
  .boolean-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    font-size: 11.5px;
    font-weight: 500;
    border-radius: 4px;
  }

  .boolean-badge.is-true {
    background-color: rgba(46, 160, 67, 0.12);
    color: #2ea043;
  }

  .boolean-badge.is-false {
    background-color: rgba(209, 36, 47, 0.12);
    color: #cf222e;
  }

  /* Objeto / JSON */
  .object-preview {
    margin: 0;
    padding: 6px 8px;
    font-size: 11px;
    font-family: var(--code-font, monospace);
    background-color: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-secondary, #e1e4e8);
    border-radius: 4px;
    max-height: 100px;
    overflow: auto;
  }

  /* Botón de copiar */
  .copy-btn {
    opacity: 0;
    background: transparent;
    border: none;
    padding: 3px;
    cursor: pointer;
    color: var(--text-secondary, #656d76);
    border-radius: 3px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: opacity 0.12s ease, background-color 0.12s ease;
    flex-shrink: 0;
  }

  .property-value:hover .copy-btn {
    opacity: 1;
  }

  .copy-btn:hover {
    background-color: var(--bg-tertiary, #eaeef2);
    color: var(--text-primary, #1f2328);
  }

  :global(.copy-success-icon) {
    color: #2ea043 !important;
  }
</style>
