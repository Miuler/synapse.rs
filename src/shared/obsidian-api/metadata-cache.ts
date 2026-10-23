import type { VaultRepository } from '@shared/repositories';
import { Events } from './events';
import {
  TFile,
  type CachedMetadataLike,
  type HeadingCache,
  type LinkCache,
  type EmbedCache,
  type SynapseMetadataCacheLike,
  type SynapseVaultLike,
} from './types';

function pathOf(file: TFile | string): string {
  return typeof file === 'string' ? file.replace(/^\/+/, '') : file.path;
}

function parseFrontmatter(content: string): Record<string, unknown> | undefined {
  const match = content.match(/^---\s*\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/);
  if (!match) return undefined;
  const yamlContent = match[1];
  const result: Record<string, unknown> = {};
  for (const line of yamlContent.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const colonIdx = trimmed.indexOf(':');
    if (colonIdx === -1) continue;
    const key = trimmed.slice(0, colonIdx).trim();
    const rawVal = trimmed.slice(colonIdx + 1).trim();
    if (rawVal === 'true') {
      result[key] = true;
    } else if (rawVal === 'false') {
      result[key] = false;
    } else if (!isNaN(Number(rawVal)) && rawVal !== '') {
      result[key] = Number(rawVal);
    } else if ((rawVal.startsWith('"') && rawVal.endsWith('"')) || (rawVal.startsWith("'") && rawVal.endsWith("'"))) {
      result[key] = rawVal.slice(1, -1);
    } else {
      result[key] = rawVal;
    }
  }
  return result;
}

export class SynapseMetadataCache extends Events implements SynapseMetadataCacheLike {
  private readonly _cache: Map<string, CachedMetadataLike> = new Map();

  constructor(
    private readonly repository: VaultRepository,
    private readonly vault: SynapseVaultLike
  ) {
    super();
  }

  /**
   * Extrae metadatos (frontmatter, headings, links, embeds, tags) a partir de una cadena markdown.
   */
  extractMetadata(content: string): CachedMetadataLike {
    const headings: HeadingCache[] = [...content.matchAll(/^(#{1,6})\s+(.+)$/gm)].map((match) => ({
      heading: match[2].trim(),
      level: match[1].length,
      position: { start: match.index ?? 0, end: (match.index ?? 0) + match[0].length },
    }));

    const links: LinkCache[] = [...content.matchAll(/(?<!!)(\[\[([^\]|#]+)(?:#[^\]|]+)?(?:\|([^\]]+))?\]\])/g)].map((match) => ({
      link: match[2].trim(),
      displayText: match[3]?.trim(),
      original: match[1],
      position: { start: match.index ?? 0, end: (match.index ?? 0) + match[0].length },
    }));

    const embeds: EmbedCache[] = [...content.matchAll(/(!\[\[([^\]|#]+)(?:#[^\]|]+)?(?:\|([^\]]+))?\]\])/g)].map((match) => ({
      link: match[2].trim(),
      displayText: match[3]?.trim(),
      original: match[1],
      position: { start: match.index ?? 0, end: (match.index ?? 0) + match[0].length },
    }));

    const tags = [...content.matchAll(/(?:^|\s)#([a-zA-Z0-9_\-/]+)/gm)].map((match) => `#${match[1]}`);

    return {
      frontmatter: parseFrontmatter(content),
      headings,
      links,
      embeds,
      tags,
    };
  }

  /**
   * Calcula y guarda los metadatos de un archivo en la caché en memoria.
   */
  computeAndStoreCache(path: string, content: string): CachedMetadataLike {
    const cleanPath = pathOf(path);
    const metadata = this.extractMetadata(content);
    this._cache.set(cleanPath, metadata);
    return metadata;
  }

  /**
   * Almacena directamente un objeto de metadatos en la caché.
   */
  setCache(path: string, cache: CachedMetadataLike): void {
    const cleanPath = pathOf(path);
    this._cache.set(cleanPath, cache);
  }

  /**
   * Elimina la entrada de caché para un archivo.
   */
  deleteCache(path: string): void {
    const cleanPath = pathOf(path);
    this._cache.delete(cleanPath);
  }

  /**
   * Renombra una entrada en la caché.
   */
  renameCache(oldPath: string, newPath: string): void {
    const cleanOld = pathOf(oldPath);
    const cleanNew = pathOf(newPath);
    const cached = this._cache.get(cleanOld);
    if (cached) {
      this._cache.delete(cleanOld);
      this._cache.set(cleanNew, cached);
    }
  }

  /**
   * Consulta la caché de un archivo de manera síncrona. Retorna null si no está en memoria.
   */
  getFileCacheSync(file: TFile | string): CachedMetadataLike | null {
    const path = pathOf(file);
    return this._cache.get(path) ?? null;
  }

  /**
   * Obtiene la caché analizada de metadatos de un archivo (frontmatter, headings, links, embeds, tags).
   * Si ya está en memoria, la retorna síncronamente (compatible con `app.metadataCache.getFileCache(f)` y con `await`).
   * Si aún no está en memoria, la carga de disco asíncronamente y la almacena.
   */
  getFileCache(file: TFile | string): CachedMetadataLike | null | Promise<CachedMetadataLike | null> {
    const path = pathOf(file);
    const inMemory = this._cache.get(path);
    if (inMemory) {
      return inMemory;
    }

    // Si aún no está en caché en memoria, leemos bajo demanda y la poblamos
    return this.repository.readNote(path).then((note) => {
      if (!note) return null;
      const content = note.content ?? '';
      return this.computeAndStoreCache(path, content);
    });
  }

  /**
   * Resuelve el destino de un enlace de tipo wikilink en el contexto de un archivo origen.
   */
  async getFirstLinkpathDest(linkpath: string, sourcePath: string): Promise<TFile | null> {
    const resolved = await this.repository.resolveVaultLink(linkpath, sourcePath);
    if (!resolved) return null;
    const file = await this.vault.getAbstractFileByPath(resolved);
    return file instanceof TFile ? file : null;
  }

  /**
   * Genera el texto estándar de wikilink para referenciar un archivo.
   */
  fileToLinktext(file: TFile | { path: string }, sourcePath?: string): string {
    const path = file.path;
    const name = path.split('/').pop() || path;
    const dot = name.lastIndexOf('.');
    const basename = dot > 0 ? name.slice(0, dot) : name;

    if (!sourcePath || !sourcePath.includes('/')) {
      return basename;
    }
    return basename;
  }

  /**
   * Notifica a los observadores que los enlaces de un archivo han sido resueltos.
   */
  triggerResolve(file: TFile): void {
    this.trigger('resolve', file);
  }

  /**
   * Notifica a los observadores que todos los enlaces de la bóveda han sido resueltos.
   */
  triggerResolved(): void {
    this.trigger('resolved');
  }
}
