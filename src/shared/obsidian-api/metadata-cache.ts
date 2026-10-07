import type { VaultRepository } from '@shared/repositories';
import type { CachedMetadataLike, TFileLike } from './types';

function pathOf(file: TFileLike | string): string {
  return typeof file === 'string' ? file : file.path;
}

function position(source: string, index: number, length: number) {
  return { start: index, end: index + length };
}

function frontmatter(content: string): Record<string, unknown> | undefined {
  const match = content.match(/^---\s*\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/);
  if (!match) return undefined;
  const result: Record<string, unknown> = {};
  for (const line of match[1].split(/\r?\n/)) {
    const item = line.match(/^\s*([^:#][^:]*):\s*(.*?)\s*$/);
    if (!item) continue;
    const value = item[2];
    result[item[1].trim()] = value === 'true' ? true : value === 'false' ? false : value;
  }
  return result;
}

export class SynapseMetadataCache {
  constructor(private readonly repository: VaultRepository) {}

  async getFileCache(file: TFileLike | string): Promise<CachedMetadataLike | null> {
    const note = await this.repository.readNote(pathOf(file));
    if (!note) return null;
    const content = note.content ?? '';
    const headings = [...content.matchAll(/^(#{1,6})\s+(.+)$/gm)].map((match) => ({
      heading: match[2].trim(),
      level: match[1].length,
      position: position(content, match.index ?? 0, match[0].length),
    }));
    const links = [...content.matchAll(/(?<!!)(\[\[([^\]|#]+)(?:#[^\]|]+)?(?:\|[^\]]+)?\]\])/g)].map((match) => ({
      link: match[2].trim(), original: match[1], position: position(content, match.index ?? 0, match[0].length),
    }));
    const embeds = [...content.matchAll(/(!\[\[([^\]|#]+)(?:#[^\]|]+)?(?:\|[^\]]+)?\]\])/g)].map((match) => ({
      link: match[2].trim(), original: match[1], position: position(content, match.index ?? 0, match[0].length),
    }));
    const tags = [...content.matchAll(/(^|\s)#([\w/-]+)/gm)].map((match) => `#${match[2]}`);
    return { frontmatter: frontmatter(content), headings, links, embeds, tags };
  }

  async getFirstLinkpathDest(link: string): Promise<TFileLike | null> {
    const files = await this.repository.getNotes();
    const normalized = link.replace(/^\//, '').toLowerCase();
    const note = files.find((item) => {
      const path = item.relative_path.toLowerCase();
      return path === normalized || path.replace(/\.(md|markdown)$/i, '') === normalized;
    });
    if (!note) return null;
    const path = note.relative_path;
    const name = path.split('/').pop() ?? path;
    const dot = name.lastIndexOf('.');
    return {
      path,
      name,
      basename: dot > 0 ? name.slice(0, dot) : name,
      extension: dot > 0 ? name.slice(dot + 1) : '',
      parent: path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : undefined,
      stat: { ctime: 0, mtime: 0, size: 0 },
      vault: undefined as never,
    };
  }
}
