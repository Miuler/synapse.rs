<!--
  Valores de un criterio del filtro: los elegidos como etiquetas y un buscador que ofrece los del
  proveedor. ↑/↓ recorren, Enter elige (o añade lo escrito) y Escape cierra la lista.
-->
<script lang="ts">
  import { X } from 'lucide-svelte';
  import { t } from '@shared/lib/i18n';

  interface Props {
    id: string;
    label: string;
    selected: string[];
    /** Los valores que ofrece el proveedor; se piden al entrar en el buscador. */
    load: () => Promise<string[]>;
    /** Cómo se muestra un valor. */
    display?: (value: string) => string;
    onChange: (selected: string[]) => void;
  }

  let { id, label, selected, load, display = (value) => value, onChange }: Props = $props();

  let search = $state('');
  let open = $state(false);
  let options = $state<string[] | null>(null);
  let failed = $state('');
  let loading = $state(false);
  let highlighted = $state(0);

  let matches = $derived.by(() => {
    const wanted = search.trim().toLowerCase();
    const found = (options ?? []).filter(
      (value) => !selected.includes(value) && display(value).toLowerCase().includes(wanted),
    );
    // Lo escrito también vale aunque el proveedor no lo ofrezca (una persona, una etiqueta nueva)
    const typed = search.trim();
    if (typed && !selected.includes(typed) && !found.some((v) => display(v).toLowerCase() === wanted)) found.push(typed);
    return found.slice(0, 100);
  });

  async function show() {
    open = true;
    if (options || loading) return;
    loading = true;
    failed = '';
    try {
      options = await load();
    } catch (error) {
      failed = String(error instanceof Error ? error.message : error);
    } finally {
      loading = false;
    }
  }

  function add(value: string) {
    onChange([...selected, value]);
    search = '';
    highlighted = 0;
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      if (!open) return void show();
      const step = event.key === 'ArrowDown' ? 1 : -1;
      highlighted = (highlighted + step + matches.length) % Math.max(matches.length, 1);
    } else if (event.key === 'Enter') {
      // Nunca envía el formulario del diálogo
      event.preventDefault();
      if (open && matches[highlighted] !== undefined) add(matches[highlighted]);
    } else if (event.key === 'Escape' && open) {
      // La lista se cierra sin cerrar el diálogo
      event.preventDefault();
      event.stopPropagation();
      open = false;
    } else if (event.key === 'Backspace' && !search && selected.length) {
      onChange(selected.slice(0, -1));
    }
  }

  $effect(() => {
    matches;
    const list = document.getElementById(`${id}-options`);
    list?.children[highlighted]?.scrollIntoView({ block: 'nearest' });
  });
</script>

<div class="filter-picker" data-criterion={id}>
  <label for={id}>{label}</label>
  <div class="chips">
    {#each selected as value (value)}
      <span class="chip">
        {display(value)}
        <button
          type="button"
          title={t('issues.source.removeValue', { value: display(value) })}
          aria-label={t('issues.source.removeValue', { value: display(value) })}
          onclick={() => onChange(selected.filter((v) => v !== value))}><X size={11} /></button
        >
      </span>
    {/each}
    <input
      {id}
      role="combobox"
      aria-expanded={open}
      aria-controls="{id}-options"
      autocomplete="off"
      spellcheck="false"
      placeholder={t('issues.source.search')}
      bind:value={search}
      onfocus={show}
      onclick={show}
      oninput={() => {
        highlighted = 0;
        show();
      }}
      onblur={() => (open = false)}
      {onkeydown}
    />
  </div>
  {#if open}
    <ul id="{id}-options" role="listbox">
      {#each matches as value, i (value)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <li
          role="option"
          aria-selected={i === highlighted}
          class:highlighted={i === highlighted}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => add(value)}
        >
          {display(value)}
        </li>
      {/each}
    </ul>
    {#if loading}<small>{t('issues.source.loading')}</small>{/if}
    {#if failed}<small class="error">{failed}</small>{/if}
  {/if}
</div>

<style>
  .filter-picker {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 4px;
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    background: var(--bg-secondary);
  }

  .chips:focus-within {
    border-color: var(--accent);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 4px 1px 8px;
    border-radius: 10px;
    background: var(--bg-tertiary);
    overflow-wrap: anywhere;
  }

  .chip button {
    display: inline-flex;
    padding: 2px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }

  input {
    flex: 1;
    min-width: 120px;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font: inherit;
  }

  ul {
    max-height: 160px;
    overflow-y: auto;
    margin: 0;
    padding: 4px;
    list-style: none;
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    background: var(--bg-primary);
  }

  li {
    padding: 4px 8px;
    border-radius: 4px;
    cursor: pointer;
  }

  li:hover,
  li.highlighted {
    background: var(--bg-tertiary);
  }

  small {
    color: var(--text-muted);
  }

  .error {
    color: var(--danger, #d1242f);
  }
</style>
