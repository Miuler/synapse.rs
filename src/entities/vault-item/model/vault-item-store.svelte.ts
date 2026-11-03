import type { VaultItem } from './types';
import { VaultItemIndex } from './vault-item-index';

/**
 * Elementos de la bóveda abierta: fuente única para el explorador, las pestañas y el Quick Open.
 *
 * Usa `$state.raw` (sin proxies por elemento): con miles de notas, un `$state` profundo cuesta
 * unas 6 veces más memoria. Cada cambio reemplaza el índice completo.
 */
class VaultItemStore {
  private index = $state.raw(new VaultItemIndex());

  get items(): readonly VaultItem[] {
    return this.index.items;
  }

  get count(): number {
    return this.index.count;
  }

  get(path: string): VaultItem | undefined {
    return this.index.get(path);
  }

  has(path: string): boolean {
    return this.index.has(path);
  }

  set(items: VaultItem[]): void {
    this.index = new VaultItemIndex(items);
  }

  clear(): void {
    this.index = new VaultItemIndex();
  }

  patch(path: string, patch: Partial<VaultItem>): void {
    this.index = this.index.patch(path, patch);
  }

  add(item: VaultItem): void {
    this.index = this.index.add(item);
  }
}

export const vaultItemStore = new VaultItemStore();
