import type { VaultRepository } from '@shared/repositories';
import { SynapseVault } from './vault';
import { SynapseMetadataCache } from './metadata-cache';
import { SynapseWorkspace } from './workspace';

export class SynapseApp {
  readonly vault: SynapseVault;
  readonly metadataCache: SynapseMetadataCache;
  readonly workspace: SynapseWorkspace;

  constructor(repository: VaultRepository) {
    this.vault = new SynapseVault(repository);
    this.metadataCache = new SynapseMetadataCache(repository);
    this.workspace = new SynapseWorkspace(repository);
  }
}
