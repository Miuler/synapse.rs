<script lang="ts">
  import { appearance } from '@entities/appearance';
  import { Command, Dialog } from 'bits-ui';
  import { Check } from 'lucide-svelte';
  import { SearchBox } from '@shared/ui/search-box';

  interface Props {
    isOpen?: boolean;
  }

  let { isOpen = $bindable(false) }: Props = $props();

  let search = $state('');
  /** Fila resaltada; al abrir, la del tema en uso. */
  let selected = $state('');

  const themes = $derived([
    { id: null as string | null, label: 'Sin tema' },
    ...appearance.themes.map((id) => ({ id: id as string | null, label: appearance.themeLabel(id) })),
  ]);
  const valueOf = (t: { id: string | null }) => t.id ?? 'sin-tema';
  const plain = (s: string) => s.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();
  // Coincidencia literal, no difusa: Enter aplica lo primero de la lista
  const found = $derived.by(() => {
    const q = plain(search.trim());
    return themes.filter((t) => plain(`${t.label} ${t.id ?? ''}`).includes(q));
  });

  $effect.pre(() => {
    if (!isOpen) return;
    search = '';
    const current = themes.find((t) => t.id === appearance.theme);
    selected = current ? valueOf(current) : '';
  });

  $effect(() => {
    if (search && found.length) selected = valueOf(found[0]);
  });

  function pick(id: string | null) {
    isOpen = false;
    appearance.setTheme(id).catch((e) => console.warn('No se pudo cambiar el tema:', e));
  }
</script>

<Dialog.Root bind:open={isOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="palette-backdrop" />
    <!-- Comparte el aspecto de la paleta de comandos -->
    <Dialog.Content class="palette-container theme-picker synapse-dialog synapse-theme-picker">
      <Dialog.Title class="sr-only">Cambiar de tema</Dialog.Title>
      <Command.Root class="command-root" loop shouldFilter={false} bind:value={selected}>
        <SearchBox bind:value={search} placeholder="Buscar un tema..." />
        <Command.List class="results-container">
          {#if !found.length}<div class="empty-state">Ningún tema coincide</div>{/if}
          {#each found as t (t.id ?? '')}
            <Command.Item
              class="command-item"
              value={valueOf(t)}
              data-theme-id={t.id ?? ''}
              aria-current={t.id === appearance.theme}
              onSelect={() => pick(t.id)}
            >
              <span class="command-name">{t.label}</span>
              {#if t.id === appearance.theme}<Check size={14} />{/if}
            </Command.Item>
          {/each}
        </Command.List>
        <footer class="palette-footer">
          <span><kbd>↑</kbd> <kbd>↓</kbd> Navegar</span>
          <span><kbd>↵</kbd> Aplicar</span>
          <span><kbd>esc</kbd> Cerrar</span>
        </footer>
      </Command.Root>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.palette-container.theme-picker) {
    width: min(420px, 90vw);
    max-height: 60vh;
  }
</style>
