<script lang="ts">
  import { Dialog } from 'bits-ui';
  import {
    appSettings,
    shortcutsConfig,
    type ShortcutsConfig,
    type ThemeMode,
    type MermaidRendererType,
    type ExcalidrawViewerType,
    type PropertiesViewMode,
  } from '@entities/settings';
  import type { SupportedLocale } from '@shared/lib/i18n';
  import { invokeTauri } from '@shared/repositories/tauri';
  import { convertFileSrc } from '@tauri-apps/api/core';

  interface Props {
    open?: boolean;
  }

  let { open = $bindable(false) }: Props = $props();

  type Section = 'appearance' | 'editor' | 'diagrams' | 'shortcuts';
  const sections: { id: Section; label: string }[] = [
    { id: 'appearance', label: 'Apariencia' },
    { id: 'editor', label: 'Editor' },
    { id: 'diagrams', label: 'Diagramas' },
    { id: 'shortcuts', label: 'Atajos de teclado' },
  ];
  let section = $state<Section>('appearance');

  // Nombres legibles de cada acción de shortcuts.jsonc
  const shortcutGroups: { section: keyof ShortcutsConfig; title: string; labels: Record<string, string> }[] = [
    {
      section: 'workspace',
      title: 'Espacio de trabajo',
      labels: {
        quick_open: 'Abrir archivo rápido',
        command_palette: 'Paleta de comandos',
        search_fulltext: 'Buscar en el contenido',
        new_file: 'Nuevo archivo',
        new_tab: 'Nueva pestaña',
        save_file: 'Guardar',
        close_tab: 'Cerrar pestaña',
        next_tab: 'Pestaña siguiente',
        prev_tab: 'Pestaña anterior',
        split_view: 'Vista dividida',
        toggle_edit_mode: 'Alternar edición / lectura',
        toggle_sidebar: 'Mostrar / ocultar barra lateral',
        locate_active_file: 'Localizar archivo activo',
        open_vault_folder: 'Abrir carpeta de la bóveda',
        nav_back: 'Atrás',
        nav_forward: 'Adelante',
        reload_active_file: 'Recargar archivo activo',
        reload_vault: 'Recargar bóveda',
        toggle_theme: 'Cambiar tema',
        toggle_devtools: 'Herramientas de desarrollo',
        git_push: 'Git: push',
        zoom_in: 'Aumentar tamaño del texto',
        zoom_out: 'Reducir tamaño del texto',
        zoom_reset: 'Restablecer tamaño del texto',
      },
    },
    {
      section: 'welcome',
      title: 'Pantalla de bienvenida',
      labels: {
        new_workspace: 'Nuevo espacio de trabajo',
        open_folder: 'Abrir carpeta',
        clone_git: 'Clonar repositorio git',
        search_workspaces: 'Buscar espacios de trabajo',
      },
    },
  ];

  let shortcutErrors = $state<Record<string, string>>({});
  let savingKey = $state<string | null>(null);

  async function saveShortcut(group: keyof ShortcutsConfig, action: string, input: HTMLInputElement) {
    const key = `${group}.${action}`;
    const value = input.value.trim();
    const current = (shortcutsConfig.config[group] as unknown as Record<string, string>)[action];
    if (value === current) return;
    if (!value) {
      input.value = current;
      return;
    }
    savingKey = key;
    try {
      await shortcutsConfig.save(group, action, value);
      const { [key]: _, ...rest } = shortcutErrors;
      shortcutErrors = rest;
    } catch (err) {
      shortcutErrors = { ...shortcutErrors, [key]: String(err) };
    } finally {
      savingKey = null;
    }
  }

  let plantumlDraft = $state(appSettings.plantumlServer);
  let plantumlError = $state('');
  $effect(() => {
    if (open) {
      plantumlDraft = appSettings.plantumlServer;
      plantumlError = '';
    }
  });

  function savePlantuml() {
    const server = plantumlDraft.trim().replace(/\/+$/, '');
    if (!/^https?:\/\//i.test(server)) {
      plantumlError = 'Debe empezar por http:// o https://';
      return;
    }
    plantumlError = '';
    if (server !== appSettings.plantumlServer) appSettings.setPlantumlServer(server);
  }

  async function chooseWallpaper() {
    const path = await invokeTauri<string | null>('pick_image_file');
    if (path) appSettings.setWallpaper(path);
  }

  let wallpaperUrl = $derived(appSettings.wallpaper ? convertFileSrc(appSettings.wallpaper) : null);
</script>

<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay class="settings-backdrop" />
    <Dialog.Content class="settings-panel">
      <nav class="settings-nav" aria-label="Secciones de configuración">
        <Dialog.Title class="settings-title">Configuración</Dialog.Title>
        {#each sections as s (s.id)}
          <button
            type="button"
            class="settings-nav-item"
            class:active={section === s.id}
            onclick={() => (section = s.id)}
          >{s.label}</button>
        {/each}
      </nav>

      <div class="settings-body">
        {#if section === 'appearance'}
          <h3>Apariencia</h3>
          <label class="row">
            <span class="row-label">Tema</span>
            <select
              value={appSettings.theme}
              onchange={(e) => appSettings.setTheme(e.currentTarget.value as ThemeMode)}
            >
              <option value="system">Seguir al sistema</option>
              <option value="light">Claro</option>
              <option value="dark">Oscuro</option>
            </select>
          </label>
          <label class="row">
            <span class="row-label">Idioma</span>
            <select
              value={appSettings.locale}
              onchange={(e) => appSettings.setLocale(e.currentTarget.value as SupportedLocale)}
            >
              <option value="es">Español</option>
              <option value="en">English</option>
            </select>
          </label>
          <div class="row">
            <span class="row-label">Tamaño del texto</span>
            <div class="inline">
              <button type="button" onclick={() => appSettings.setContentZoom(appSettings.contentZoom - 0.1)}>−</button>
              <span class="value">{Math.round(appSettings.contentZoom * 100)}%</span>
              <button type="button" onclick={() => appSettings.setContentZoom(appSettings.contentZoom + 0.1)}>+</button>
              <button type="button" onclick={() => appSettings.setContentZoom(1)}>Restablecer</button>
            </div>
          </div>
          <div class="row wallpaper-row">
            <span class="row-label">Fondo de la pestaña vacía</span>
            <div class="inline">
              {#if wallpaperUrl}
                <img class="wallpaper-thumb" src={wallpaperUrl} alt="Fondo actual" />
              {/if}
              <button type="button" onclick={chooseWallpaper}>Elegir imagen…</button>
              {#if appSettings.wallpaper}
                <button type="button" onclick={() => appSettings.setWallpaper(undefined)}>Quitar</button>
              {/if}
            </div>
          </div>
        {:else if section === 'editor'}
          <h3>Editor</h3>
          <label class="row">
            <span class="row-label">Modo Vim</span>
            <input
              type="checkbox"
              checked={appSettings.vimMode}
              onchange={(e) => appSettings.setVimMode(e.currentTarget.checked)}
            />
          </label>
          <label class="row">
            <span class="row-label">Propiedades de la nota</span>
            <select
              value={appSettings.propertiesViewMode}
              onchange={(e) => appSettings.setPropertiesViewMode(e.currentTarget.value as PropertiesViewMode)}
            >
              <option value="expanded">Visibles</option>
              <option value="collapsed">Plegadas</option>
              <option value="hidden">Ocultas</option>
            </select>
          </label>
        {:else if section === 'diagrams'}
          <h3>Diagramas</h3>
          <label class="row">
            <span class="row-label">Visor de Mermaid</span>
            <select
              value={appSettings.mermaidRenderer}
              onchange={(e) => appSettings.setMermaidRenderer(e.currentTarget.value as MermaidRendererType)}
            >
              <option value="mermaidjs">Mermaid.js</option>
              <option value="merman">Merman</option>
            </select>
          </label>
          <label class="row">
            <span class="row-label">Visor de Excalidraw</span>
            <select
              value={appSettings.defaultExcalidrawViewer}
              onchange={(e) => appSettings.setDefaultExcalidrawViewer(e.currentTarget.value as ExcalidrawViewerType)}
            >
              <option value="native">Excalidraw nativo</option>
              <option value="obsidian">Compatible con Obsidian</option>
            </select>
          </label>
          <div class="row column">
            <span class="row-label">Servidor de PlantUML</span>
            <div class="inline grow">
              <input
                type="url"
                bind:value={plantumlDraft}
                onkeydown={(e) => e.key === 'Enter' && savePlantuml()}
                onblur={savePlantuml}
              />
            </div>
            <span class="hint">El código de los diagramas PlantUML se envía a este servidor para dibujarlos.</span>
            {#if plantumlError}<span class="error">{plantumlError}</span>{/if}
          </div>
        {:else}
          <h3>Atajos de teclado</h3>
          <p class="hint">
            Se guardan en <code>~/.config/synapsers/shortcuts.jsonc</code>. Para varios atajos en una misma
            acción, sepáralos con comas: <code>Alt+1, Ctrl+B</code>. Se guarda al pulsar Enter o al salir del campo.
          </p>
          {#each shortcutGroups as group (group.section)}
            <h4>{group.title}</h4>
            {#each Object.entries(group.labels) as [action, label] (action)}
              {@const key = `${group.section}.${action}`}
              <label class="row">
                <span class="row-label">{label}</span>
                <input
                  class="shortcut-input"
                  class:saving={savingKey === key}
                  value={(shortcutsConfig.config[group.section] as unknown as Record<string, string>)[action]}
                  onkeydown={(e) => e.stopPropagation()}
                  onchange={(e) => saveShortcut(group.section, action, e.currentTarget)}
                />
              </label>
              {#if shortcutErrors[key]}<span class="error">{shortcutErrors[key]}</span>{/if}
            {/each}
          {/each}
        {/if}
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.settings-backdrop) {
    position: fixed;
    inset: 0;
    background-color: var(--dialog-backdrop, rgba(10, 12, 16, 0.7));
    z-index: 1000;
  }

  :global(.settings-panel) {
    position: fixed;
    top: 8vh;
    left: 50%;
    transform: translateX(-50%);
    width: min(820px, calc(100vw - 32px));
    height: min(620px, 84vh);
    display: flex;
    background: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 10px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.35);
    overflow: hidden;
    z-index: 1001;
  }

  .settings-nav {
    width: 200px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 16px 8px;
    background: var(--bg-secondary, #f6f8fa);
    border-right: 1px solid var(--border-primary, #d0d7de);
  }

  :global(.settings-title) {
    margin: 0 8px 12px;
    font-size: 15px;
    font-weight: 600;
  }

  .settings-nav-item {
    text-align: left;
    padding: 7px 10px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary, #656d76);
    font-size: 13px;
    cursor: pointer;
  }

  .settings-nav-item:hover {
    background: var(--bg-hover, rgba(127, 127, 127, 0.12));
    color: var(--text-primary, #1f2328);
  }

  .settings-nav-item.active {
    background: var(--accent-bg, rgba(9, 105, 218, 0.1));
    color: var(--accent, #0969da);
    font-weight: 500;
  }

  .settings-body {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: 18px 24px 24px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  h3 {
    margin: 0 0 10px;
    font-size: 16px;
    font-weight: 600;
  }

  h4 {
    margin: 16px 0 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-secondary, #656d76);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 9px 0;
    border-bottom: 1px solid var(--border-secondary, rgba(127, 127, 127, 0.15));
    font-size: 13px;
  }

  .row.column {
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
  }

  .row-label {
    flex: 1;
    min-width: 0;
  }

  .inline {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .inline.grow input {
    flex: 1;
  }

  .value {
    min-width: 44px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }

  select,
  input:not([type='checkbox']),
  button {
    font: inherit;
    font-size: 13px;
    color: var(--text-primary, #1f2328);
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    padding: 5px 9px;
  }

  button {
    cursor: pointer;
  }

  button:hover {
    border-color: var(--accent, #0969da);
  }

  .shortcut-input {
    width: 240px;
    font-family: var(--font-mono, monospace);
  }

  .shortcut-input.saving {
    opacity: 0.6;
  }

  .wallpaper-thumb {
    width: 64px;
    height: 40px;
    object-fit: cover;
    border-radius: 4px;
    border: 1px solid var(--border-primary, #d0d7de);
  }

  .hint {
    margin: 0 0 4px;
    font-size: 12px;
    color: var(--text-secondary, #656d76);
  }

  .error {
    font-size: 12px;
    color: var(--danger, #cf222e);
  }

  code {
    font-size: 12px;
  }

  @media (max-width: 640px) {
    :global(.settings-panel) {
      flex-direction: column;
    }

    .settings-nav {
      width: auto;
      flex-direction: row;
      flex-wrap: wrap;
      border-right: none;
      border-bottom: 1px solid var(--border-primary, #d0d7de);
    }

    .shortcut-input {
      width: 160px;
    }
  }
</style>
