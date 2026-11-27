<script lang="ts">
  import { t } from '@shared/lib/i18n';
  import { Dialog } from 'bits-ui';
  import { DialogResizeHandles, persistedDialogSize } from '@shared/ui/dialog-resize';
  import {
    appSettings,
    shortcutsConfig,
    type ShortcutsConfig,
    type ThemeMode,
    type MermaidRendererType,
    type ExcalidrawViewerType,
    type PropertiesViewMode,
    type FolderClickMode,
    type AgentCompletionSound,
  } from '@entities/settings';
  import { AGENT_SOUND_OPTIONS, playAgentCompletionSound } from '@shared/lib/sound';
  import type { SupportedLocale } from '@shared/lib/i18n';
  import { invokeTauri } from '@shared/repositories/tauri';
  import { appearance, SNIPPETS_DIR, THEMES_DIR } from '@entities/appearance';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { agentApi, agentStore, DEFAULT_PROCESS_LIMIT, MAX_QUOTA_MINUTES } from '@entities/acp-agent';
  import { appUpdates } from '@features/app-updates';
  import { gitBinary } from '@entities/git';
  import { AzureTokenField } from '@features/board-editor';

  // Bóvedas aceptadas o rechazadas por esta (se leen al abrir Ajustes)
  let peerDecisions = $state(agentStore.peers.decisions());

  // El modo feedback solo existe en compilaciones con la feature `develop`
  let feedbackAvailable = $state(false);
  agentApi
    .feedbackAvailable()
    .then((v) => (feedbackAvailable = v))
    .catch(() => {});

  const dialogSize = persistedDialogSize(
    'synapse.settingsPanel.size',
    { w: 820, h: 620 },
    { w: 560, h: 380 },
    { centeredX: true, centeredY: false, maxHeightMargin: 96 },
  );

  type Section = 'appearance' | 'editor' | 'diagrams' | 'agents' | 'git' | 'azure' | 'updates' | 'shortcuts';

  interface Props {
    open?: boolean;
    section?: Section;
  }

  let { open = $bindable(false), section = $bindable<Section>('appearance') }: Props = $props();

  const sections: { id: Section; label: string }[] = [
    { id: 'appearance', label: 'Apariencia' },
    { id: 'editor', label: 'Editor' },
    { id: 'diagrams', label: 'Diagramas' },
    { id: 'agents', label: 'Agentes de IA' },
    { id: 'git', label: 'Git' },
    { id: 'azure', label: 'Azure DevOps' },
    { id: 'updates', label: 'Actualizaciones' },
    { id: 'shortcuts', label: 'Atajos de teclado' },
  ];

  // Agentes de IA: catálogo verificado y personalizados
  let installing = $state<string | null>(null);
  let agentError = $state('');
  let draftAgent = $state({ id: '', name: '', command: '', args: '', env: '' });

  $effect(() => {
    if (open && section === 'agents') agentStore.refresh();
    if (open && section === 'git') void gitBinary.load();
  });

  async function installAgent(id: string) {
    agentError = '';
    installing = id;
    try {
      await agentApi.install(id);
      await agentStore.refresh();
    } catch (e) {
      agentError = String(e);
    } finally {
      installing = null;
    }
  }

  async function saveCustomAgent() {
    agentError = '';
    const env: Record<string, string> = {};
    for (const line of draftAgent.env.split('\n')) {
      const i = line.indexOf('=');
      if (i > 0) env[line.slice(0, i).trim()] = line.slice(i + 1).trim();
    }
    try {
      await agentApi.saveProfile({
        id: draftAgent.id,
        name: draftAgent.name,
        command: draftAgent.command.trim(),
        args: draftAgent.args.split(/\s+/).filter(Boolean),
        env,
      });
      draftAgent = { id: '', name: '', command: '', args: '', env: '' };
      await agentStore.refresh();
    } catch (e) {
      // Cancelar la confirmación no es un error: el borrador se conserva
      agentError = e === 'Cancelado' ? '' : String(e);
    }
  }

  async function deleteCustomAgent(id: string) {
    agentError = '';
    try {
      await agentApi.deleteProfile(id);
      await agentStore.refresh();
    } catch (e) {
      agentError = String(e);
    }
  }

  // Carpetas de temas globales: la que se está escribiendo y el error del último guardado
  let newThemeDir = $state('');
  let themeDirsError = $state('');

  async function saveThemeDirs(dirs: string[]) {
    themeDirsError = '';
    try {
      await appearance.setThemeDirs(dirs);
    } catch (e) {
      themeDirsError = String(e);
    }
  }

  async function addThemeDir() {
    if (!newThemeDir.trim()) return;
    await saveThemeDirs([...appearance.themeDirs, newThemeDir]);
    if (!themeDirsError) newThemeDir = '';
  }

  // Nombres legibles de cada acción de shortcuts.jsonc
  const shortcutGroups: { section: keyof ShortcutsConfig; title: string; labels: Record<string, string> }[] = [
    {
      section: 'workspace',
      title: 'General',
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
        toggle_agent: 'Mostrar / ocultar el agente',
        locate_active_file: 'Localizar archivo activo',
        open_vault_folder: 'Abrir carpeta de la bóveda',
        nav_back: 'Atrás',
        nav_forward: 'Adelante',
        reload_active_file: 'Recargar archivo activo',
        reload_vault: 'Recargar bóveda',
        toggle_theme: 'Alternar tema claro / oscuro',
        pick_theme: 'Elegir tema',
        toggle_devtools: 'Herramientas de desarrollo',
        toggle_outline: 'Índice de la nota o de las claves',
        git_push: 'Git: push',
        zoom_in: 'Aumentar tamaño del texto',
        zoom_out: 'Reducir tamaño del texto',
        zoom_reset: 'Restablecer tamaño del texto',
        toggle_task: 'Marcar / desmarcar tarea',
      },
    },
    {
      section: 'welcome',
      title: 'Pantalla de bienvenida',
      labels: {
        new_workspace: 'Nueva bóveda',
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
    <Dialog.Content
      class={`settings-panel synapse-dialog synapse-settings ${dialogSize.resizing ? 'is-resizing' : ''}`}
      style={dialogSize.style}
    >
      <DialogResizeHandles onStart={dialogSize.startResize} />
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
            <span class="row-label">Fondo de la pestaña vacía (de esta bóveda)</span>
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

          <h4 class="subsection">Tema y fragmentos CSS (de esta bóveda)</h4>
          <label class="row">
            <span class="row-label">Tema personalizado</span>
            <select
              class="custom-theme-select"
              value={appearance.theme ?? ''}
              onchange={(e) => appearance.setTheme(e.currentTarget.value || null)}
            >
              <option value="">Ninguno</option>
              {#each appearance.themes as name (name)}
                <option value={name}>{appearance.themeLabel(name)}</option>
              {/each}
            </select>
          </label>
          {#each appearance.snippets as name (name)}
            <label class="row">
              <span class="row-label">{name}</span>
              <input
                type="checkbox"
                class="snippet-toggle"
                data-snippet={name}
                checked={appearance.enabledSnippets.includes(name)}
                onchange={(e) => appearance.toggleSnippet(name, e.currentTarget.checked)}
              />
            </label>
          {/each}
          <div class="row">
            <span class="row-label hint">
              Temas en <code>{THEMES_DIR}/</code> y fragmentos en <code>{SNIPPETS_DIR}/</code>, como archivos <code>.css</code>.
            </span>
            <button type="button" class="appearance-reload" onclick={() => appearance.rescan()}>Recargar</button>
          </div>

          <h4 class="subsection">Carpetas de temas globales (todas las bóvedas)</h4>
          <p class="hint">
            Cada <code>.css</code> de estas carpetas aparece como tema en todas las bóvedas. La primera se lee
            siempre; las que agregues se guardan en <code>appearance.theme_dirs</code> del
            <code>config.jsonc</code> global. «~» es tu carpeta de usuario.
          </p>
          {#if appearance.defaultThemeDir}
            <div class="row theme-dir theme-dir-default" data-dir={appearance.defaultThemeDir}>
              <code class="theme-dir-path">{appearance.defaultThemeDir}</code>
              <span class="theme-dir-count">
                {appearance.globalThemes.filter((t) => t.dir === appearance.defaultThemeDir).length} temas · siempre
              </span>
            </div>
          {/if}
          {#each appearance.themeDirs as dir, i (dir)}
            <div class="row theme-dir" data-dir={dir}>
              <code class="theme-dir-path">{dir}</code>
              <span class="theme-dir-count">
                {appearance.globalThemes.filter((t) => t.dir === dir).length} temas
              </span>
              <button
                type="button"
                class="theme-dir-remove"
                aria-label="Quitar carpeta {dir}"
                onclick={() => saveThemeDirs(appearance.themeDirs.filter((_, j) => j !== i))}
              >Quitar</button>
            </div>
          {/each}
          <form
            class="row theme-dir-add"
            onsubmit={(e) => {
              e.preventDefault();
              addThemeDir();
            }}
          >
            <input
              class="theme-dir-input"
              placeholder="~/mis-temas o /ruta/absoluta"
              bind:value={newThemeDir}
            />
            <button type="submit" class="theme-dir-add-btn">Agregar</button>
            <button type="button" class="theme-dirs-rescan" onclick={() => appearance.rescan()}>Reescanear</button>
          </form>
          {#if themeDirsError}
            <p class="error">{themeDirsError}</p>
          {/if}
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
            <span class="row-label">Clic en una carpeta del explorador</span>
            <select
              value={appSettings.folderClick}
              onchange={(e) => appSettings.setFolderClick(e.currentTarget.value as FolderClickMode)}
            >
              <option value="index">Abrir su índice (index.md)</option>
              <option value="expand">Desplegarla</option>
            </select>
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
        {:else if section === 'agents'}
          <h3>Agentes de IA</h3>
          <p class="hint">
            Agentes compatibles con ACP (Agent Client Protocol). Los verificados se descargan del registro
            público de ACP, el mismo que usa Zed, y se verifican con su sha256.
          </p>
          {#if agentStore.listError}<span class="error">{agentStore.listError}</span>{/if}
          <p class="hint">
            Cada conversación usa el MCP de esta ventana de Synapse (<code>synapse_instance</code>), y se le pide al
            agente que cree y edite las notas con él en lugar de escribir los archivos directamente.
          </p>
          <label class="row">
            <span class="row-label">
              Agente por defecto
              <span class="hint">Para todas las bóvedas: el que se usa en una bóveda que todavía no usó ninguno (por ejemplo, cuando otra bóveda le escribe por primera vez) y el de «＋ Nueva conversación». Si no está instalado, se descarga al usarlo.</span>
            </span>
            <select
              aria-label="Agente por defecto"
              value={agentStore.defaultAgentId ?? ''}
              onchange={(e) => agentStore.setDefaultAgent(e.currentTarget.value || null)}
            >
              <option value="">El primero instalado</option>
              {#each agentStore.agents as a (a.id)}
                <option value={a.id}>{a.name}{a.installed ? '' : ' (se descargará)'}</option>
              {/each}
            </select>
          </label>
          <label class="row">
            <span class="row-label">
              Agente para conversaciones de otras bóvedas
              <span class="hint">🐙 Cuando un agente de otra bóveda abierta le escribe a esta, se abre una conversación con este agente. Se guarda para esta bóveda.</span>
            </span>
            <select
              aria-label="Agente para conversaciones de otras bóvedas"
              value={agentStore.incomingAgentId ?? ''}
              onchange={(e) => agentStore.setIncomingAgent(e.currentTarget.value || null)}
            >
              <option value="">El último usado o el agente por defecto</option>
              {#each agentStore.agents as a (a.id)}
                <option value={a.id}>{a.name}</option>
              {/each}
            </select>
          </label>
          <label class="row">
            <span class="row-label">
              {t('quota.setting')}
              <span class="hint">{t('quota.settingHint')}</span>
            </span>
            <input
              type="number"
              min="1"
              max={MAX_QUOTA_MINUTES}
              aria-label={t('quota.setting')}
              value={agentStore.quotaMinutes}
              onchange={(e) => {
                agentStore.setQuotaMinutes(e.currentTarget.valueAsNumber);
                e.currentTarget.value = String(agentStore.quotaMinutes);
              }}
            />
          </label>
          <div class="row">
            <span class="row-label">
              Sonido al terminar un turno
              <span class="hint">Se reproduce cuando el agente termina de responder.</span>
            </span>
            <div class="inline">
              <select
                aria-label="Sonido al terminar un turno"
                value={appSettings.agentCompletionSound}
                onchange={(e) => {
                  const chosen = e.currentTarget.value as AgentCompletionSound;
                  appSettings.setAgentCompletionSound(chosen);
                  void playAgentCompletionSound(chosen);
                }}
              >
                {#each AGENT_SOUND_OPTIONS as opt (opt.id)}
                  <option value={opt.id}>{opt.label}</option>
                {/each}
              </select>
              <button
                type="button"
                disabled={appSettings.agentCompletionSound === 'none'}
                onclick={() => void playAgentCompletionSound(appSettings.agentCompletionSound)}
              >Probar</button>
            </div>
          </div>
          <div class="row">
            <span class="row-label">
              {t('watermark.setting')}
              <span class="hint">{t('watermark.hint')}</span>
            </span>
            <div class="inline">
              <input
                type="range"
                min="0"
                max="40"
                step="1"
                aria-label={t('watermark.setting')}
                value={appSettings.agentWatermark}
                oninput={(e) => appSettings.setAgentWatermark(e.currentTarget.valueAsNumber)}
              />
              <span>{appSettings.agentWatermark} %</span>
            </div>
          </div>
          <div class="row">
            <span class="row-label">
              {t('laser.sizeSetting')}
              <span class="hint">{t('laser.sizeHint')}</span>
            </span>
            <div class="inline">
              <input
                type="range"
                min="1"
                max="8"
                step="0.5"
                aria-label={t('laser.sizeSetting')}
                value={appSettings.laserSize}
                oninput={(e) => appSettings.setLaserSize(e.currentTarget.valueAsNumber)}
              />
              <span>{appSettings.laserSize} px</span>
            </div>
          </div>
          <div class="row peer-decisions">
            <span class="row-label">
              Bóvedas que pueden conversar con esta
              <span class="hint">🐙 Lo que respondiste cuando un agente de otra bóveda quiso conversar con esta. Al quitar una, se vuelve a preguntar la próxima vez.</span>
            </span>
            {#if peerDecisions.length}
              <ul>
                {#each peerDecisions as d (d.path)}
                  <li title={d.path}>
                    <span>{d.allowed ? '✅' : '🚫'} {d.vault}</span>
                    <button
                      type="button"
                      aria-label="Quitar {d.vault}"
                      onclick={() => {
                        agentStore.peers.forget(d.path);
                        peerDecisions = agentStore.peers.decisions();
                      }}>Quitar</button
                    >
                  </li>
                {/each}
              </ul>
            {:else}
              <span class="hint">Ninguna todavía.</span>
            {/if}
          </div>
          <label class="row">
            <span class="row-label">
              Registro de depuración de ACP
              <span class="hint">Guarda, sin filtrar, cada mensaje que entra y sale de los agentes (herramientas con su resultado incluido, si el agente lo manda){#if agentStore.debugLogPath} en <code>{agentStore.debugLogPath}</code>{/if}. Útil para ver qué envía cada agente; puede crecer rápido.</span>
            </span>
            <input
              type="checkbox"
              aria-label="Registro de depuración de ACP"
              checked={agentStore.debugLog}
              onchange={(e) => agentStore.setDebugLog(e.currentTarget.checked)}
            />
          </label>
          {#if agentStore.debugLogPath}
            <button
              type="button"
              class="link-btn"
              onclick={() => invokeTauri('show_item_in_folder', { path: agentStore.debugLogPath }).catch(() => {})}
            >Mostrar el registro en su carpeta</button>
          {/if}
          {#if feedbackAvailable}
          <label class="row">
            <span class="row-label">
              Modo feedback del MCP
              <span class="hint">El agente anota en <code>~/.config/synapsers/feedback.md</code> lo que le falte al MCP. Aplica a las conversaciones nuevas.</span>
            </span>
            <input
              type="checkbox"
              checked={agentStore.feedbackMode}
              onchange={(e) => agentStore.setFeedbackMode(e.currentTarget.checked)}
            />
          </label>
          {/if}
          <h4 class="agents-subtitle">Verificados</h4>
          {#each agentStore.agents.filter((a) => a.source === 'verified') as a (a.id)}
            <div class="row agent-row">
              <span class="row-label">
                {a.name}
                {#if a.version}<span class="hint">v{a.version}</span>{/if}
              </span>
              <select
                class="process-limit"
                title="Conversaciones que comparte un mismo proceso del agente; con más, se lanza otro. Usa 1 si el agente atiende una conversación a la vez."
                aria-label="Conversaciones por proceso de {a.name}"
                value={String(agentStore.processLimits[a.id] ?? DEFAULT_PROCESS_LIMIT)}
                onchange={(e) => agentStore.setProcessLimit(a.id, Number(e.currentTarget.value))}
              >
                {#each [1, 2, 3, 4, 6, 8] as n (n)}
                  <option value={String(n)}>{n} por proceso</option>
                {/each}
              </select>
              {#if a.installed}
                <span class="hint">Instalado</span>
              {:else}
                <button type="button" disabled={installing !== null} onclick={() => installAgent(a.id)}>
                  {installing === a.id ? 'Instalando…' : 'Instalar'}
                </button>
              {/if}
            </div>
          {:else}
            <p class="hint">Sin agentes verificados disponibles.</p>
          {/each}

          <h4 class="agents-subtitle">Personalizados</h4>
          {#each agentStore.agents.filter((a) => a.source === 'custom') as a (a.id)}
            <div class="row agent-row">
              <span class="row-label">
                {a.name}
                <span class="hint">{a.launch?.command} {a.launch?.args.join(' ')}</span>
              </span>
              <select
                class="process-limit"
                title="Conversaciones que comparte un mismo proceso del agente; con más, se lanza otro. Usa 1 si el agente atiende una conversación a la vez."
                aria-label="Conversaciones por proceso de {a.name}"
                value={String(agentStore.processLimits[a.id] ?? DEFAULT_PROCESS_LIMIT)}
                onchange={(e) => agentStore.setProcessLimit(a.id, Number(e.currentTarget.value))}
              >
                {#each [1, 2, 3, 4, 6, 8] as n (n)}
                  <option value={String(n)}>{n} por proceso</option>
                {/each}
              </select>
              <button type="button" onclick={() => deleteCustomAgent(a.id)}>Quitar</button>
            </div>
          {/each}
          <form
            class="agent-form"
            onsubmit={(e) => {
              e.preventDefault();
              saveCustomAgent();
            }}
          >
            <input placeholder="Identificador (mi-agente)" bind:value={draftAgent.id} required />
            <input placeholder="Nombre" bind:value={draftAgent.name} required />
            <input placeholder="Comando (ruta o nombre en el PATH)" bind:value={draftAgent.command} required />
            <input placeholder="Argumentos separados por espacios" bind:value={draftAgent.args} />
            <textarea placeholder="Variables de entorno, una por línea: CLAVE=valor" rows="2" bind:value={draftAgent.env}
            ></textarea>
            <button type="submit">Agregar agente</button>
          </form>
          {#if agentError}<span class="error">{agentError}</span>{/if}
        {:else if section === 'git'}
          <h3>Git</h3>
          <div class="row">
            <span class="row-label">Ejecutable</span>
            <span class="git-path" title={gitBinary.info?.configuredPath ?? 'git del PATH'}>
              {gitBinary.info?.configuredPath ?? 'git del PATH del sistema'}
            </span>
          </div>
          <div class="row">
            <span class="row-label">Estado</span>
            {#if !gitBinary.info}
              <span>Comprobando…</span>
            {:else if gitBinary.info.version}
              <span>✓ {gitBinary.info.version}</span>
            {:else}
              <span class="error">No se encontró Git</span>
            {/if}
          </div>
          <div class="row">
            <span class="row-label"></span>
            <span class="git-actions">
              <button type="button" class="theme-dirs-rescan" disabled={gitBinary.busy} onclick={() => gitBinary.choose()}>Elegir ejecutable…</button>
              {#if gitBinary.info?.configuredPath}
                <button type="button" class="theme-dirs-rescan" disabled={gitBinary.busy} onclick={() => gitBinary.setPath(null)}>Usar el del sistema</button>
              {/if}
            </span>
          </div>
          {#if gitBinary.error}<p class="error">{gitBinary.error}</p>{/if}
          <p class="hint">
            Ver el estado, el historial y las versiones no necesita Git; commit, cambiar de rama, push, clone y
            Git LFS sí. La ruta se guarda en <code>config.jsonc</code> (<code>git.path</code>).
          </p>
        {:else if section === 'azure'}
          <h3>Azure DevOps</h3>
          <AzureTokenField autosave />
        {:else if section === 'updates'}
          <h3>Actualizaciones</h3>
          <label class="row">
            <span class="row-label">Buscar actualizaciones al iniciar</span>
            <input
              type="checkbox"
              checked={appSettings.autoCheckUpdates}
              onchange={(e) => appSettings.setAutoCheckUpdates(e.currentTarget.checked)}
            />
          </label>
          <p class="hint">
            Consulta el <code>latest.json</code> de la última release en GitHub. Solo se envía la petición
            (sin datos de tus notas); el paquete se verifica con su firma antes de instalarlo.
          </p>
          <div class="row">
            <span class="row-label">Ahora</span>
            <button
              type="button"
              class="theme-dirs-rescan"
              disabled={appUpdates.status === 'checking' || appUpdates.status === 'installing'}
              onclick={() => appUpdates.check(false)}
            >
              {appUpdates.status === 'checking' ? 'Buscando…' : 'Buscar actualizaciones'}
            </button>
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
  .agents-subtitle {
    margin: 14px 0 4px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
  }
  .process-limit {
    margin-left: auto;
    margin-right: 8px;
    font-size: 11px;
  }
  .agent-row .row-label {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .agent-form {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
    margin-top: 8px;
  }
  .agent-form textarea,
  .agent-form button {
    grid-column: 1 / -1;
  }
  .agent-form button {
    justify-self: start;
  }
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
    display: flex;
    background: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 10px;
    box-shadow: var(--modal-shadow);
    overflow: hidden;
    z-index: 1001;
  }

  :global(.settings-panel.is-resizing) {
    user-select: none !important;
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

  .theme-dir-path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .theme-dir-count {
    color: var(--text-secondary, #656d76);
    font-size: 12px;
  }

  .theme-dir-input {
    flex: 1;
    min-width: 0;
  }

  .wallpaper-thumb {
    width: 64px;
    height: 40px;
    object-fit: cover;
    border-radius: 4px;
    border: 1px solid var(--border-primary, #d0d7de);
  }

  .git-path {
    flex: 1;
    min-width: 0;
    font-family: var(--code-font, monospace);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .git-actions {
    display: inline-flex;
    gap: 8px;
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
  .peer-decisions ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 200px;
  }
  .peer-decisions li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
</style>
