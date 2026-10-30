<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { shortcutsConfig, appSettings } from '@entities/settings';
  import { Kbd } from '@shared/ui/kbd';
  import { SearchBox } from '@shared/ui/search-box';

  interface Props {
    isOpen?: boolean;
  }

  let { isOpen = $bindable(false) }: Props = $props();

  interface Entry {
    label: string;
    shortcut: string;
  }
  interface Group {
    title: string;
    entries: Entry[];
  }

  let query = $state('');

  // Los atajos configurables salen de shortcuts.jsonc; el resto son atajos fijos de los editores
  let groups = $derived.by<Group[]>(() => {
    const ws = shortcutsConfig.workspace;
    const all: Group[] = [
      {
        title: 'Archivos y pestañas',
        entries: [
          { label: 'Abrir archivo rápido', shortcut: ws.quick_open },
          { label: 'Nuevo archivo', shortcut: ws.new_file },
          { label: 'Nueva pestaña', shortcut: ws.new_tab },
          { label: 'Guardar', shortcut: ws.save_file },
          { label: 'Cerrar pestaña', shortcut: ws.close_tab },
          { label: 'Pestaña siguiente', shortcut: ws.next_tab },
          { label: 'Pestaña anterior', shortcut: ws.prev_tab },
          { label: 'Vista dividida', shortcut: ws.split_view },
          { label: 'Alternar edición / lectura', shortcut: ws.toggle_edit_mode },
        ],
      },
      {
        title: 'Navegación',
        entries: [
          { label: 'Paleta de comandos', shortcut: ws.command_palette },
          { label: 'Buscar en el contenido', shortcut: ws.search_fulltext },
          { label: 'Atrás', shortcut: ws.nav_back },
          { label: 'Adelante', shortcut: ws.nav_forward },
          { label: 'Mostrar / ocultar barra lateral', shortcut: ws.toggle_sidebar },
          { label: 'Localizar archivo activo', shortcut: ws.locate_active_file },
          { label: 'Abrir carpeta de la bóveda', shortcut: ws.open_vault_folder },
          { label: 'Atajos de teclado', shortcut: 'F1' },
        ],
      },
      {
        title: 'Editor',
        entries: [
          { label: 'Deshacer', shortcut: 'Ctrl+Z' },
          { label: 'Rehacer', shortcut: 'Ctrl+Shift+Z, Ctrl+Y' },
          { label: 'Comentar / descomentar líneas', shortcut: 'Ctrl+/' },
          { label: 'Buscar en el archivo', shortcut: 'Ctrl+F' },
        ],
      },
      {
        title: 'Explorador',
        entries: [
          { label: 'Copiar', shortcut: 'Ctrl+C' },
          { label: 'Cortar (para mover)', shortcut: 'Ctrl+X' },
          { label: 'Pegar', shortcut: 'Ctrl+V' },
          { label: 'Recargar', shortcut: 'F5, Ctrl+R' },
          { label: 'Eliminar', shortcut: 'Delete' },
        ],
      },
      {
        title: 'Vista y desarrollo',
        entries: [
          { label: 'Recargar archivo activo', shortcut: ws.reload_active_file },
          { label: 'Recargar bóveda', shortcut: ws.reload_vault },
          { label: 'Cambiar tema', shortcut: ws.toggle_theme },
          { label: 'Herramientas de desarrollo', shortcut: ws.toggle_devtools },
          { label: 'Git: push', shortcut: ws.git_push },
        ],
      },
    ];
    // Los atajos de vim solo aparecen si el modo vim está activo
    if (appSettings.vimMode) {
      all.push({
        title: 'Vim',
        entries: [
          { label: 'Deshacer', shortcut: 'u' },
          { label: 'Deshacer cambios de la última línea', shortcut: 'U' },
          { label: 'Rehacer', shortcut: 'Ctrl+R' },
        ],
      });
    }
    return all;
  });

  let filtered = $derived.by<Group[]>(() => {
    const q = query.trim().toLowerCase();
    if (!q) return groups;
    return groups
      .map((g) => ({
        ...g,
        entries: g.entries.filter((e) => `${g.title} ${e.label} ${e.shortcut}`.toLowerCase().includes(q)),
      }))
      .filter((g) => g.entries.length > 0);
  });

  $effect(() => {
    if (!isOpen) query = '';
  });
</script>

<Dialog.Root bind:open={isOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="shortcuts-backdrop" />
    <Dialog.Content class="shortcuts-panel">
      <Dialog.Title class="shortcuts-title">Atajos de teclado</Dialog.Title>
      <SearchBox bind:value={query} placeholder="Buscar un atajo o una acción..." />

      <div class="shortcuts-body">
        {#each filtered as group (group.title)}
          <section class="shortcuts-group">
            <h3>{group.title}</h3>
            <ul>
              {#each group.entries as entry (entry.label + entry.shortcut)}
                <li>
                  <span class="shortcut-label">{entry.label}</span>
                  <Kbd shortcut={entry.shortcut} all />
                </li>
              {/each}
            </ul>
          </section>
        {:else}
          <p class="shortcuts-empty">No hay atajos que coincidan.</p>
        {/each}
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.shortcuts-backdrop) {
    position: fixed;
    inset: 0;
    background-color: var(--dialog-backdrop, rgba(10, 12, 16, 0.7));
    z-index: 1000;
  }

  :global(.shortcuts-panel) {
    position: fixed;
    top: 10vh;
    left: 50%;
    transform: translateX(-50%);
    width: min(680px, calc(100vw - 32px));
    max-height: 78vh;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    background: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 10px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.35);
    z-index: 1001;
  }

  :global(.shortcuts-title) {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }

  .shortcuts-body {
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 16px 24px;
    padding-right: 4px;
  }

  .shortcuts-group h3 {
    margin: 0 0 6px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-secondary, #656d76);
  }

  .shortcuts-group ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .shortcuts-group li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 5px 0;
    font-size: 13px;
    border-bottom: 1px solid var(--border-primary, #d0d7de33);
  }

  .shortcuts-empty {
    grid-column: 1 / -1;
    text-align: center;
    color: var(--text-secondary, #656d76);
  }
</style>
