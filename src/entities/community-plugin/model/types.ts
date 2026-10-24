import type { PluginManifest } from '@shared/obsidian-api';

export interface CommunityPluginDto {
  id: string;
  name: string;
  author: string;
  description: string;
  repo: string;
  branch?: string;
  downloads?: number;
}

export interface CommunityPluginInstallResultDto {
  id: string;
  version: string;
  installed_path: string;
  has_styles: boolean;
}

export interface InstalledPluginDto {
  id: string;
  name: string;
  version: string;
  description?: string;
  author?: string;
  dir: string;
  has_styles: boolean;
  repo?: string;
}

export interface PluginUpdateInfoDto {
  id: string;
  current_version: string;
  latest_version: string;
  has_update: boolean;
  repo: string;
}

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
