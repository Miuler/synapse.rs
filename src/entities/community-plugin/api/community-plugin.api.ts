import { invokeTauri } from '@shared/api';
import type {
  CommunityPluginDto,
  InstalledPluginDto,
  PluginUpdateInfoDto,
  CommunityPluginInstallResultDto,
} from '../model/types';

export const communityPluginApi = {
  async fetchCommunityPlugins(forceRefresh = false): Promise<CommunityPluginDto[]> {
    return await invokeTauri<CommunityPluginDto[]>('fetch_community_plugins', { forceRefresh });
  },

  async getInstalledCommunityPlugins(vaultPath?: string): Promise<InstalledPluginDto[]> {
    return await invokeTauri<InstalledPluginDto[]>('get_installed_community_plugins', { vaultPath });
  },

  async checkCommunityPluginUpdates(
    plugins: Array<{ id: string; current_version: string; repo: string }>
  ): Promise<PluginUpdateInfoDto[]> {
    const raw = plugins.map((p) => [p.id, p.current_version, p.repo] as [string, string, string]);
    return await invokeTauri<PluginUpdateInfoDto[]>('check_community_plugin_updates', { plugins: raw });
  },

  async installCommunityPlugin(
    pluginId: string,
    repo: string,
    vaultPath?: string
  ): Promise<CommunityPluginInstallResultDto> {
    return await invokeTauri<CommunityPluginInstallResultDto>('install_community_plugin', {
      pluginId,
      repo,
      vaultPath,
    });
  },

  async uninstallCommunityPlugin(pluginId: string, vaultPath?: string): Promise<void> {
    await invokeTauri('uninstall_community_plugin', { pluginId, vaultPath });
  },

  async getEnabledPlugins(vaultPath?: string): Promise<string[]> {
    const configPath = vaultPath
      ? `${vaultPath.replace(/\\/g, '/').replace(/\/$/, '')}/.synapse/community-plugins.json`
      : '.synapse/community-plugins.json';
    try {
      const bytes = await invokeTauri<number[]>('read_binary_file', { relativePath: configPath });
      const text = new TextDecoder('utf-8').decode(new Uint8Array(bytes));
      const parsed = JSON.parse(text);
      if (Array.isArray(parsed)) {
        return parsed;
      }
      return [];
    } catch {
      return [];
    }
  },

  async setEnabledPlugins(plugins: string[], vaultPath?: string): Promise<void> {
    const configPath = vaultPath
      ? `${vaultPath.replace(/\\/g, '/').replace(/\/$/, '')}/.synapse/community-plugins.json`
      : '.synapse/community-plugins.json';
    const text = JSON.stringify(plugins, null, 2);
    const bytes = Array.from(new TextEncoder().encode(text));
    await invokeTauri('write_binary_file', { relativePath: configPath, data: bytes });
  },
};
