import type {
  CommunityPluginDto,
  CommunityPluginInstallResultDto,
  InstalledPluginDto,
  PluginUpdateInfoDto,
} from '@shared/repositories';
import type { PluginManifest } from '@shared/obsidian-api';

export type {
  CommunityPluginDto,
  CommunityPluginInstallResultDto,
  InstalledPluginDto,
  PluginUpdateInfoDto,
};

export interface CommunityPluginItem extends CommunityPluginDto {
  isInstalled?: boolean;
  isEnabled?: boolean;
  installedVersion?: string;
  updateAvailable?: {
    latestVersion: string;
    repo: string;
  };
}

export interface InstalledPluginItem {
  id: string;
  manifest: PluginManifest;
  dir: string;
  isEnabled: boolean;
  hasSettingsTab: boolean;
  updateAvailable?: {
    latestVersion: string;
    repo: string;
  };
}

export type PluginModalTab = 'community' | 'installed';
export type PluginSortOrder = 'downloads' | 'name';
