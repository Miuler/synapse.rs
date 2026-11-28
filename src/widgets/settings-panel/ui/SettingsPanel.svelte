<script lang="ts">
  import { t } from '@shared/lib/i18n';
  import { Dialog } from 'bits-ui';
  import { DialogResizeHandles, persistedDialogSize } from '@shared/ui/dialog-resize';
  import {
    appSettings,
    MAX_RECENTS_COUNT,
    NO_WALLPAPER,
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
  import { ConnectionsSettings } from '@features/board-editor';

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
    { id: 'appearance', get label() { return t('settings.sections.appearance'); } },
    { id: 'editor', get label() { return t('settings.sections.editor'); } },
    { id: 'diagrams', get label() { return t('settings.sections.diagrams'); } },
    { id: 'agents', get label() { return t('settings.sections.agents'); } },
    { id: 'git', get label() { return t('settings.sections.git'); } },
    { id: 'azure', get label() { return t('connections.title'); } },
    { id: 'updates', get label() { return t('settings.sections.updates'); } },
    { id: 'shortcuts', get label() { return t('settings.sections.shortcuts'); } },
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
      agentError = e === 'Cancelado' || e === 'Cancelled' ? '' : String(e);
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
  const shortcutGroups = $derived<{ section: keyof ShortcutsConfig; title: string; labels: Record<string, string> }[]>([
    {
      section: 'workspace',
      title: t('settings.shortcuts.generalGroup'),
      labels: {
        quick_open: t('shortcutLabels.actions.quick_open'),
        command_palette: t('shortcutLabels.actions.command_palette'),
        search_fulltext: t('shortcutLabels.actions.search_fulltext'),
        new_file: t('shortcutLabels.actions.new_file'),
        new_tab: t('shortcutLabels.actions.new_tab'),
        save_file: t('shortcutLabels.actions.save_file'),
        close_tab: t('shortcutLabels.actions.close_tab'),
        next_tab: t('shortcutLabels.actions.next_tab'),
        prev_tab: t('shortcutLabels.actions.prev_tab'),
        next_tab_outside_text: t('shortcutLabels.actions.next_tab_outside_text'),
        prev_tab_outside_text: t('shortcutLabels.actions.prev_tab_outside_text'),
        split_view: t('shortcutLabels.actions.split_view'),
        toggle_edit_mode: t('shortcutLabels.actions.toggle_edit_mode'),
        toggle_sidebar: t('shortcutLabels.actions.toggle_sidebar'),
        toggle_agent: t('shortcutLabels.actions.toggle_agent'),
        locate_active_file: t('shortcutLabels.actions.locate_active_file'),
        open_vault_folder: t('shortcutLabels.actions.open_vault_folder'),
        nav_back: t('shortcutLabels.actions.nav_back'),
        nav_forward: t('shortcutLabels.actions.nav_forward'),
        reload_active_file: t('shortcutLabels.actions.reload_active_file'),
        reload_vault: t('shortcutLabels.actions.reload_vault'),
        toggle_theme: t('shortcutLabels.actions.toggle_theme'),
        pick_theme: t('shortcutLabels.actions.pick_theme'),
        toggle_devtools: t('shortcutLabels.actions.toggle_devtools'),
        toggle_outline: t('shortcutLabels.actions.toggle_outline'),
        git_push: t('shortcutLabels.actions.git_push'),
        zoom_in: t('shortcutLabels.actions.zoom_in'),
        zoom_out: t('shortcutLabels.actions.zoom_out'),
        zoom_reset: t('shortcutLabels.actions.zoom_reset'),
        toggle_task: t('shortcutLabels.actions.toggle_task'),
      },
    },
    {
      section: 'welcome',
      title: t('settings.shortcuts.welcomeGroup'),
      labels: {
        new_workspace: t('shortcutLabels.welcome.new_workspace'),
        open_folder: t('shortcutLabels.welcome.open_folder'),
        clone_git: t('shortcutLabels.welcome.clone_git'),
        search_workspaces: t('shortcutLabels.welcome.search_workspaces'),
      },
    },
  ]);

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
      plantumlError = t('settings.diagrams.plantumlInvalid');
      return;
    }
    plantumlError = '';
    if (server !== appSettings.plantumlServer) appSettings.setPlantumlServer(server);
  }

  async function chooseWallpaper() {
    const path = await invokeTauri<string | null>('pick_image_file');
    if (path) appSettings.setWallpaper(path);
  }

  let wallpaperUrl = $derived(
    appSettings.wallpaper && appSettings.wallpaper !== NO_WALLPAPER ? convertFileSrc(appSettings.wallpaper) : null,
  );
</script>

<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay class="settings-backdrop" />
    <Dialog.Content
      class={`settings-panel synapse-dialog synapse-settings ${dialogSize.resizing ? 'is-resizing' : ''}`}
      style={dialogSize.style}
    >
      <DialogResizeHandles onStart={dialogSize.startResize} />
      <nav class="settings-nav" aria-label={t('settings.navAria')}>
        <Dialog.Title class="settings-title">{t('settings.title')}</Dialog.Title>
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
          <h3>{t('settings.sections.appearance')}</h3>
          <label class="row">
            <span class="row-label">{t('settings.appearance.theme')}</span>
            <select
              value={appSettings.theme}
              onchange={(e) => appSettings.setTheme(e.currentTarget.value as ThemeMode)}
            >
              <option value="system">{t('settings.appearance.themeSystem')}</option>
              <option value="light">{t('settings.appearance.themeLight')}</option>
              <option value="dark">{t('settings.appearance.themeDark')}</option>
            </select>
          </label>
          <label class="row">
            <span class="row-label">{t('settings.appearance.language')}</span>
            <select
              value={appSettings.locale}
              onchange={(e) => appSettings.setLocale(e.currentTarget.value as SupportedLocale)}
            >
              <option value="es">Español</option>
              <option value="en">English</option>
            </select>
          </label>
          <div class="row">
            <span class="row-label">{t('settings.appearance.textSize')}</span>
            <div class="inline">
              <button type="button" onclick={() => appSettings.setContentZoom(appSettings.contentZoom - 0.1)}>−</button>
              <span class="value">{Math.round(appSettings.contentZoom * 100)}%</span>
              <button type="button" onclick={() => appSettings.setContentZoom(appSettings.contentZoom + 0.1)}>+</button>
              <button type="button" onclick={() => appSettings.setContentZoom(1)}>{t('settings.appearance.reset')}</button>
            </div>
          </div>
          <label class="row">
            <span class="row-label">
              {t('emptyTab.countSetting')}
              <span class="hint">{t('emptyTab.countHint')}</span>
            </span>
            <input
              type="number"
              min="1"
              max={MAX_RECENTS_COUNT}
              aria-label={t('emptyTab.countSetting')}
              value={appSettings.recentsCount}
              onchange={(e) => {
                appSettings.setRecentsCount(e.currentTarget.valueAsNumber);
                e.currentTarget.value = String(appSettings.recentsCount);
              }}
            />
          </label>
          <div class="row wallpaper-row">
            <span class="row-label">{t('settings.appearance.emptyTabBg')}</span>
            <div class="inline">
              {#if wallpaperUrl}
                <img class="wallpaper-thumb" src={wallpaperUrl} alt={t('settings.appearance.currentBgAlt')} />
              {/if}
              <button type="button" onclick={chooseWallpaper}>{t('settings.appearance.chooseImage')}</button>
              {#if appSettings.wallpaper}
                <button type="button" onclick={() => appSettings.setWallpaper(undefined)}>{t('settings.appearance.remove')}</button>
              {/if}
            </div>
          </div>

          <h4 class="subsection">{t('settings.appearance.cssHeading')}</h4>
          <label class="row">
            <span class="row-label">{t('settings.appearance.customTheme')}</span>
            <select
              class="custom-theme-select"
              value={appearance.theme ?? ''}
              onchange={(e) => appearance.setTheme(e.currentTarget.value || null)}
            >
              <option value="">{t('settings.appearance.none')}</option>
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
              {t('settings.appearance.cssLocationPrefix')} <code>{THEMES_DIR}/</code> {t('settings.appearance.cssLocationMiddle')} <code>{SNIPPETS_DIR}/</code>{t('settings.appearance.cssLocationSuffix')} <code>.css</code>{t('settings.appearance.cssLocationFiles')}
            </span>
            <button type="button" class="appearance-reload" onclick={() => appearance.rescan()}>{t('settings.appearance.reload')}</button>
          </div>

          <h4 class="subsection">{t('settings.appearance.globalDirsHeading')}</h4>
          <p class="hint">
            {t('settings.appearance.globalDirsPart1')} <code>.css</code> {t('settings.appearance.globalDirsPart2')} <code>appearance.theme_dirs</code> {t('settings.appearance.globalDirsPart3')}
            <code>config.jsonc</code>{t('settings.appearance.globalDirsPart4')}
          </p>
          {#if appearance.defaultThemeDir}
            <div class="row theme-dir theme-dir-default" data-dir={appearance.defaultThemeDir}>
              <code class="theme-dir-path">{appearance.defaultThemeDir}</code>
              <span class="theme-dir-count">
                {t('settings.appearance.themesAlways', { count: appearance.globalThemes.filter((t) => t.dir === appearance.defaultThemeDir).length })}
              </span>
            </div>
          {/if}
          {#each appearance.themeDirs as dir, i (dir)}
            <div class="row theme-dir" data-dir={dir}>
              <code class="theme-dir-path">{dir}</code>
              <span class="theme-dir-count">
                {t('settings.appearance.themesCount', { count: appearance.globalThemes.filter((t) => t.dir === dir).length })}
              </span>
              <button
                type="button"
                class="theme-dir-remove"
                aria-label={t('settings.appearance.removeFolderAria', { dir })}
                onclick={() => saveThemeDirs(appearance.themeDirs.filter((_, j) => j !== i))}
              >{t('settings.appearance.remove')}</button>
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
              placeholder={t('settings.appearance.dirPlaceholder')}
              bind:value={newThemeDir}
            />
            <button type="submit" class="theme-dir-add-btn">{t('settings.appearance.add')}</button>
            <button type="button" class="theme-dirs-rescan" onclick={() => appearance.rescan()}>{t('settings.appearance.rescan')}</button>
          </form>
          {#if themeDirsError}
            <p class="error">{themeDirsError}</p>
          {/if}
        {:else if section === 'editor'}
          <h3>{t('settings.sections.editor')}</h3>
          <label class="row">
            <span class="row-label">{t('settings.editor.vimMode')}</span>
            <input
              type="checkbox"
              checked={appSettings.vimMode}
              onchange={(e) => appSettings.setVimMode(e.currentTarget.checked)}
            />
          </label>
          <label class="row">
            <span class="row-label">{t('settings.editor.folderClick')}</span>
            <select
              value={appSettings.folderClick}
              onchange={(e) => appSettings.setFolderClick(e.currentTarget.value as FolderClickMode)}
            >
              <option value="index">{t('settings.editor.folderClickIndex')}</option>
              <option value="expand">{t('settings.editor.folderClickExpand')}</option>
            </select>
          </label>
          <label class="row">
            <span class="row-label">{t('settings.editor.noteProperties')}</span>
            <select
              value={appSettings.propertiesViewMode}
              onchange={(e) => appSettings.setPropertiesViewMode(e.currentTarget.value as PropertiesViewMode)}
            >
              <option value="expanded">{t('settings.editor.propertiesExpanded')}</option>
              <option value="collapsed">{t('settings.editor.propertiesCollapsed')}</option>
              <option value="hidden">{t('settings.editor.propertiesHidden')}</option>
            </select>
          </label>
        {:else if section === 'diagrams'}
          <h3>{t('settings.sections.diagrams')}</h3>
          <label class="row">
            <span class="row-label">{t('settings.diagrams.mermaidViewer')}</span>
            <select
              value={appSettings.mermaidRenderer}
              onchange={(e) => appSettings.setMermaidRenderer(e.currentTarget.value as MermaidRendererType)}
            >
              <option value="mermaidjs">Mermaid.js</option>
              <option value="merman">Merman</option>
            </select>
          </label>
          <label class="row">
            <span class="row-label">{t('settings.diagrams.excalidrawViewer')}</span>
            <select
              value={appSettings.defaultExcalidrawViewer}
              onchange={(e) => appSettings.setDefaultExcalidrawViewer(e.currentTarget.value as ExcalidrawViewerType)}
            >
              <option value="native">{t('settings.diagrams.excalidrawNative')}</option>
              <option value="obsidian">{t('settings.diagrams.excalidrawObsidian')}</option>
            </select>
          </label>
          <div class="row column">
            <span class="row-label">{t('settings.diagrams.plantumlServer')}</span>
            <div class="inline grow">
              <input
                type="url"
                bind:value={plantumlDraft}
                onkeydown={(e) => e.key === 'Enter' && savePlantuml()}
                onblur={savePlantuml}
              />
            </div>
            <span class="hint">{t('settings.diagrams.plantumlHint')}</span>
            {#if plantumlError}<span class="error">{plantumlError}</span>{/if}
          </div>
        {:else if section === 'agents'}
          <h3>{t('settings.sections.agents')}</h3>
          <p class="hint">
            {t('settings.agents.intro')}
          </p>
          {#if agentStore.listError}<span class="error">{agentStore.listError}</span>{/if}
          <p class="hint">
            {t('settings.agents.mcpPrefix')}<code>synapse_instance</code>{t('settings.agents.mcpSuffix')}
          </p>
          <label class="row">
            <span class="row-label">
              {t('settings.agents.defaultAgent')}
              <span class="hint">{t('settings.agents.defaultAgentHint')}</span>
            </span>
            <select
              aria-label={t('settings.agents.defaultAgent')}
              value={agentStore.defaultAgentId ?? ''}
              onchange={(e) => agentStore.setDefaultAgent(e.currentTarget.value || null)}
            >
              <option value="">{t('settings.agents.firstInstalled')}</option>
              {#each agentStore.agents as a (a.id)}
                <option value={a.id}>{a.name}{a.installed ? '' : t('settings.agents.willDownload')}</option>
              {/each}
            </select>
          </label>
          <label class="row">
            <span class="row-label">
              {t('settings.agents.incomingAgent')}
              <span class="hint">{t('settings.agents.incomingAgentHint')}</span>
            </span>
            <select
              aria-label={t('settings.agents.incomingAgent')}
              value={agentStore.incomingAgentId ?? ''}
              onchange={(e) => agentStore.setIncomingAgent(e.currentTarget.value || null)}
            >
              <option value="">{t('settings.agents.lastUsedOrDefault')}</option>
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
              {t('settings.agents.completionSound')}
              <span class="hint">{t('settings.agents.completionSoundHint')}</span>
            </span>
            <div class="inline">
              <select
                aria-label={t('settings.agents.completionSound')}
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
              >{t('settings.agents.testSound')}</button>
            </div>
          </div>
          <label class="row">
            <span class="row-label">
              {t('agentNotice.setting')}
              <span class="hint">{t('agentNotice.hint')}</span>
            </span>
            <select
              class="agent-notice-content"
              value={appSettings.agentNoticeContent}
              onchange={(e) => appSettings.setAgentNoticeContent(e.currentTarget.value as 'answer' | 'query')}
            >
              <option value="answer">{t('agentNotice.answer')}</option>
              <option value="query">{t('agentNotice.query')}</option>
            </select>
          </label>
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
              {t('settings.agents.peerVaults')}
              <span class="hint">{t('settings.agents.peerVaultsHint')}</span>
            </span>
            {#if peerDecisions.length}
              <ul>
                {#each peerDecisions as d (d.path)}
                  <li title={d.path}>
                    <span>{d.allowed ? '✅' : '🚫'} {d.vault}</span>
                    <button
                      type="button"
                      aria-label="{t('settings.appearance.remove')} {d.vault}"
                      onclick={() => {
                        agentStore.peers.forget(d.path);
                        peerDecisions = agentStore.peers.decisions();
                      }}>{t('settings.appearance.remove')}</button
                    >
                  </li>
                {/each}
              </ul>
            {:else}
              <span class="hint">{t('settings.agents.noneYet')}</span>
            {/if}
          </div>
          <label class="row">
            <span class="row-label">
              {t('settings.agents.debugLog')}
              <span class="hint">{t('settings.agents.debugLogPart1')}{#if agentStore.debugLogPath} {t('settings.agents.debugLogIn')} <code>{agentStore.debugLogPath}</code>{/if}{t('settings.agents.debugLogPart2')}</span>
            </span>
            <input
              type="checkbox"
              aria-label={t('settings.agents.debugLog')}
              checked={agentStore.debugLog}
              onchange={(e) => agentStore.setDebugLog(e.currentTarget.checked)}
            />
          </label>
          {#if agentStore.debugLogPath}
            <button
              type="button"
              class="link-btn"
              onclick={() => invokeTauri('show_item_in_folder', { path: agentStore.debugLogPath }).catch(() => {})}
            >{t('settings.agents.showLogFolder')}</button>
          {/if}
          {#if feedbackAvailable}
          <label class="row">
            <span class="row-label">
              {t('settings.agents.feedbackMode')}
              <span class="hint">{t('settings.agents.feedbackPart1')} <code>~/.config/synapsers/feedback.md</code> {t('settings.agents.feedbackPart2')}</span>
            </span>
            <input
              type="checkbox"
              checked={agentStore.feedbackMode}
              onchange={(e) => agentStore.setFeedbackMode(e.currentTarget.checked)}
            />
          </label>
          {/if}
          <h4 class="agents-subtitle">{t('settings.agents.verified')}</h4>
          {#each agentStore.agents.filter((a) => a.source === 'verified') as a (a.id)}
            <div class="row agent-row">
              <span class="row-label">
                {a.name}
                {#if a.version}<span class="hint">v{a.version}</span>{/if}
              </span>
              <select
                class="process-limit"
                title={t('settings.agents.processLimitTitle')}
                aria-label={t('settings.agents.processLimitAria', { name: a.name })}
                value={String(agentStore.processLimits[a.id] ?? DEFAULT_PROCESS_LIMIT)}
                onchange={(e) => agentStore.setProcessLimit(a.id, Number(e.currentTarget.value))}
              >
                {#each [1, 2, 3, 4, 6, 8] as n (n)}
                  <option value={String(n)}>{t('settings.agents.perProcess', { n })}</option>
                {/each}
              </select>
              {#if a.installed}
                <span class="hint">{t('settings.agents.installed')}</span>
              {:else}
                <button type="button" disabled={installing !== null} onclick={() => installAgent(a.id)}>
                  {installing === a.id ? t('settings.agents.installing') : t('settings.agents.install')}
                </button>
              {/if}
            </div>
          {:else}
            <p class="hint">{t('settings.agents.noVerified')}</p>
          {/each}

          <h4 class="agents-subtitle">{t('settings.agents.custom')}</h4>
          {#each agentStore.agents.filter((a) => a.source === 'custom') as a (a.id)}
            <div class="row agent-row">
              <span class="row-label">
                {a.name}
                <span class="hint">{a.launch?.command} {a.launch?.args.join(' ')}</span>
              </span>
              <select
                class="process-limit"
                title={t('settings.agents.processLimitTitle')}
                aria-label={t('settings.agents.processLimitAria', { name: a.name })}
                value={String(agentStore.processLimits[a.id] ?? DEFAULT_PROCESS_LIMIT)}
                onchange={(e) => agentStore.setProcessLimit(a.id, Number(e.currentTarget.value))}
              >
                {#each [1, 2, 3, 4, 6, 8] as n (n)}
                  <option value={String(n)}>{t('settings.agents.perProcess', { n })}</option>
                {/each}
              </select>
              <button type="button" onclick={() => deleteCustomAgent(a.id)}>{t('settings.appearance.remove')}</button>
            </div>
          {/each}
          <form
            class="agent-form"
            onsubmit={(e) => {
              e.preventDefault();
              saveCustomAgent();
            }}
          >
            <input placeholder={t('settings.agents.idPlaceholder')} bind:value={draftAgent.id} required />
            <input placeholder={t('settings.agents.namePlaceholder')} bind:value={draftAgent.name} required />
            <input placeholder={t('settings.agents.commandPlaceholder')} bind:value={draftAgent.command} required />
            <input placeholder={t('settings.agents.argsPlaceholder')} bind:value={draftAgent.args} />
            <textarea placeholder={t('settings.agents.envPlaceholder')} rows="2" bind:value={draftAgent.env}
            ></textarea>
            <button type="submit">{t('settings.agents.addAgent')}</button>
          </form>
          {#if agentError}<span class="error">{agentError}</span>{/if}
        {:else if section === 'git'}
          <h3>{t('settings.sections.git')}</h3>
          <div class="row">
            <span class="row-label">{t('settings.git.executable')}</span>
            <span class="git-path" title={gitBinary.info?.configuredPath ?? t('settings.git.systemPathTitle')}>
              {gitBinary.info?.configuredPath ?? t('settings.git.systemPath')}
            </span>
          </div>
          <div class="row">
            <span class="row-label">{t('settings.git.status')}</span>
            {#if !gitBinary.info}
              <span>{t('settings.git.checking')}</span>
            {:else if gitBinary.info.version}
              <span>✓ {gitBinary.info.version}</span>
            {:else}
              <span class="error">{t('settings.git.notFound')}</span>
            {/if}
          </div>
          <div class="row">
            <span class="row-label"></span>
            <span class="git-actions">
              <button type="button" class="theme-dirs-rescan" disabled={gitBinary.busy} onclick={() => gitBinary.choose()}>{t('settings.git.chooseExecutable')}</button>
              {#if gitBinary.info?.configuredPath}
                <button type="button" class="theme-dirs-rescan" disabled={gitBinary.busy} onclick={() => gitBinary.setPath(null)}>{t('settings.git.useSystem')}</button>
              {/if}
            </span>
          </div>
          {#if gitBinary.error}<p class="error">{gitBinary.error}</p>{/if}
          <p class="hint">
            {t('settings.git.hintPrefix')} <code>config.jsonc</code> (<code>git.path</code>).
          </p>
        {:else if section === 'azure'}
          <h3>{t('connections.title')}</h3>
          <ConnectionsSettings />
        {:else if section === 'updates'}
          <h3>{t('settings.sections.updates')}</h3>
          <label class="row">
            <span class="row-label">{t('settings.updates.checkOnStartup')}</span>
            <input
              type="checkbox"
              checked={appSettings.autoCheckUpdates}
              onchange={(e) => appSettings.setAutoCheckUpdates(e.currentTarget.checked)}
            />
          </label>
          <p class="hint">
            {t('settings.updates.hintPart1')} <code>latest.json</code> {t('settings.updates.hintPart2')}
          </p>
          <div class="row">
            <span class="row-label">{t('settings.updates.now')}</span>
            <button
              type="button"
              class="theme-dirs-rescan"
              disabled={appUpdates.status === 'checking' || appUpdates.status === 'installing'}
              onclick={() => appUpdates.check(false)}
            >
              {appUpdates.status === 'checking' ? t('settings.updates.checking') : t('settings.updates.checkNow')}
            </button>
          </div>
        {:else}
          <h3>{t('settings.sections.shortcuts')}</h3>
          <p class="hint">
            {t('settings.shortcuts.hintPart1')} <code>~/.config/synapsers/shortcuts.jsonc</code>{t('settings.shortcuts.hintPart2')} <code>Alt+1, Ctrl+B</code>{t('settings.shortcuts.hintPart3')}
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
