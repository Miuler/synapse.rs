import type { VaultRepository } from '@shared/repositories';
import { SynapseVault } from './vault';
import { SynapseMetadataCache } from './metadata-cache';
import { SynapseWorkspace } from './workspace';
import { SynapseFileManager } from './file-manager';

export class SynapseApp {
  readonly vault: SynapseVault;
  readonly metadataCache: SynapseMetadataCache;
  readonly workspace: SynapseWorkspace;
  readonly fileManager: SynapseFileManager;

  constructor(repository: VaultRepository) {
    this.vault = new SynapseVault(repository);
    this.workspace = new SynapseWorkspace(repository, this.vault);
    this.metadataCache = new SynapseMetadataCache(repository, this.vault);
    this.fileManager = new SynapseFileManager(this.vault);

    // Conectar caché reactiva a la bóveda para sincronización bidireccional automática de eventos
    this.vault.setMetadataCache(this.metadataCache);
  }
}

export function createSynapseApp(repository: VaultRepository): SynapseApp {
  return new SynapseApp(repository);
}
