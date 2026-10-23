import type { CommunityPluginDto, CommunityPluginInstallResultDto } from '@shared/repositories';
import type { PluginManifest } from '@shared/obsidian-api';

export type { CommunityPluginDto, CommunityPluginInstallResultDto };

export interface CommunityPluginItem extends CommunityPluginDto {
  isInstalled?: boolean;
  isEnabled?: boolean;
  installedVersion?: string;
}

export interface InstalledPluginItem {
  id: string;
  manifest: PluginManifest;
  dir: string;
  isEnabled: boolean;
  hasSettingsTab: boolean;
}

export type PluginModalTab = 'community' | 'installed';
export type PluginSortOrder = 'downloads' | 'name';
