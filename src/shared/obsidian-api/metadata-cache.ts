import type { VaultRepository } from '@shared/repositories';
import { TFile, type CachedMetadataLike, type SynapseMetadataCacheLike, type SynapseVaultLike } from './types';

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

export class SynapseMetadataCache implements SynapseMetadataCacheLike {
  constructor(
    private readonly repository: VaultRepository,
    private readonly vault: SynapseVaultLike
  ) {}

  /**
   * Obtiene la caché analizada de metadatos de un archivo (frontmatter, headings, links, embeds, tags).
   */
  async getFileCache(file: TFile | string): Promise<CachedMetadataLike | null> {
    const path = pathOf(file);
    const note = await this.repository.readNote(path);
    if (!note) return null;
    const content = note.content ?? '';

    const headings = [...content.matchAll(/^(#{1,6})\s+(.+)$/gm)].map((match) => ({
      heading: match[2].trim(),
      level: match[1].length,
      position: { start: match.index ?? 0, end: (match.index ?? 0) + match[0].length },
    }));

    const links = [...content.matchAll(/(?<!!)(\[\[([^\]|#]+)(?:#[^\]|]+)?(?:\|([^\]]+))?\]\])/g)].map((match) => ({
      link: match[2].trim(),
      displayText: match[3]?.trim(),
      original: match[1],
      position: { start: match.index ?? 0, end: (match.index ?? 0) + match[0].length },
    }));

    const embeds = [...content.matchAll(/(!\[\[([^\]|#]+)(?:#[^\]|]+)?(?:\|([^\]]+))?\]\])/g)].map((match) => ({
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
    const sourceDir = sourcePath.slice(0, sourcePath.lastIndexOf('/'));
    if (path.startsWith(sourceDir + '/')) {
      return basename;
    }
    return basename;
  }
}
