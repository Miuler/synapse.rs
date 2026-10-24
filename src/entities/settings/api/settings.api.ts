import { invokeTauri } from '@shared/api';

export interface OpenTabDto {
  path: string;
  is_pinned?: boolean;
}

export interface WorkspaceOpenTabsState {
  open_tabs: OpenTabDto[];
  active_tab?: string | null;
}

export interface VaultUiState {
  sidebar_width?: number;
  expanded_folders?: string[];
  diff_visual_mode?: string;
  diff_text_mode?: string;
  locked_files?: string[];
}

export interface RecentWorkspaceEntry {
  path: string;
  name: string;
  last_opened: number;
  pinned: boolean;
  notes_count?: number;
  git_branch?: string;
  is_git_repo?: boolean;
}

export interface SelectVaultFolderResult {
  folder_path: string;
  notes: Array<{
    relative_path: string;
    abs_path: string;
    title: string;
    content: string;
    encoding?: string;
    view_mode?: string;
  }>;
}

export type StartupTarget =
  | { type: 'vault'; path: string }
  | { type: 'standalone'; path: string }
  | null;

export const settingsApi = {
  async saveOpenTabsState(tabs: OpenTabDto[], activeTab?: string | null): Promise<void> {
    await invokeTauri('save_open_tabs_state', { tabs, activeTab: activeTab ?? null });
  },

  async getOpenTabsState(): Promise<WorkspaceOpenTabsState | null> {
    return await invokeTauri<WorkspaceOpenTabsState | null>('get_open_tabs_state');
  },

  async saveVaultUiState(state: {
    sidebarWidth?: number;
    expandedFolders?: string[];
    diffVisualMode?: string;
    diffTextMode?: string;
    lockedFiles?: string[];
  }): Promise<void> {
    await invokeTauri('save_vault_ui_state', {
      sidebarWidth: state.sidebarWidth ?? null,
      expandedFolders: state.expandedFolders ?? null,
      diffVisualMode: state.diffVisualMode ?? null,
      diffTextMode: state.diffTextMode ?? null,
      lockedFiles: state.lockedFiles ?? null,
    });
  },

  async getVaultUiState(): Promise<VaultUiState | null> {
    return await invokeTauri<VaultUiState | null>('get_vault_ui_state');
  },

  async getRecentWorkspaces(): Promise<RecentWorkspaceEntry[]> {
    return await invokeTauri<RecentWorkspaceEntry[]>('get_recent_workspaces');
  },

  async togglePinWorkspace(path: string): Promise<RecentWorkspaceEntry[]> {
    return await invokeTauri<RecentWorkspaceEntry[]>('toggle_pin_workspace', { path });
  },

  async removeRecentWorkspace(path: string): Promise<RecentWorkspaceEntry[]> {
    return await invokeTauri<RecentWorkspaceEntry[]>('remove_recent_workspace', { path });
  },

  async createNewWorkspace(startingDirectory?: string): Promise<SelectVaultFolderResult | null> {
    return await invokeTauri<SelectVaultFolderResult | null>('create_new_workspace', { startingDirectory });
  },

  async cloneGitWorkspace(repoUrl: string, startingDirectory?: string): Promise<SelectVaultFolderResult | null> {
    return await invokeTauri<SelectVaultFolderResult | null>('clone_git_workspace', { repoUrl, startingDirectory });
  },

  async openWorkspaceInNewWindow(path: string): Promise<void> {
    await invokeTauri('open_workspace_in_new_window', { path });
  },

  async selectVaultFolder(startingDirectory?: string): Promise<SelectVaultFolderResult | null> {
    return await invokeTauri<SelectVaultFolderResult | null>('select_vault_folder', { startingDirectory });
  },

  async getActiveVaultPath(): Promise<string | null> {
    return await invokeTauri<string | null>('get_active_vault_path');
  },

  async setActiveVaultPath(path: string): Promise<void> {
    await invokeTauri('set_active_vault_path', { newPath: path });
  },

  async getStartupTarget(): Promise<StartupTarget> {
    const raw = await invokeTauri<Record<string, string> | null>('get_startup_target');
    if (!raw) return null;
    if ('Vault' in raw) return { type: 'vault', path: raw.Vault };
    if ('Standalone' in raw) return { type: 'standalone', path: raw.Standalone };
    return null;
  },

  async selectStandaloneFile(): Promise<string | null> {
    return await invokeTauri<string | null>('select_standalone_file');
  },

  async openFileWithDefaultApp(path: string): Promise<void> {
    await invokeTauri('open_file_with_default_app', { path });
  },

  async showItemInFolder(path: string): Promise<void> {
    await invokeTauri('show_item_in_folder', { path });
  },
};
