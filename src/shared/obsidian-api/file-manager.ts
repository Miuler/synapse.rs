import type { SynapseFileManagerLike, SynapseVaultLike, TFile } from './types';

function pathOf(file: TFile | string): string {
  return typeof file === 'string' ? file.replace(/^\/+/, '') : file.path;
}

function parseYamlFrontmatter(yamlStr: string): Record<string, any> {
  const result: Record<string, any> = {};
  for (const line of yamlStr.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const colonIdx = trimmed.indexOf(':');
    if (colonIdx === -1) continue;
    const key = trimmed.slice(0, colonIdx).trim();
    const rawVal = trimmed.slice(colonIdx + 1).trim();
    if (rawVal === 'true') result[key] = true;
    else if (rawVal === 'false') result[key] = false;
    else if (!isNaN(Number(rawVal)) && rawVal !== '') result[key] = Number(rawVal);
    else if ((rawVal.startsWith('"') && rawVal.endsWith('"')) || (rawVal.startsWith("'") && rawVal.endsWith("'"))) {
      result[key] = rawVal.slice(1, -1);
    } else {
      result[key] = rawVal;
    }
  }
  return result;
}

function stringifyYamlFrontmatter(data: Record<string, any>): string {
  const lines: string[] = ['---'];
  for (const [key, value] of Object.entries(data)) {
    if (value === undefined) continue;
    if (typeof value === 'string') {
      if (value.includes(':') || value.includes('\n') || value.includes('"') || value.includes("'")) {
        lines.push(`${key}: "${value.replace(/"/g, '\\"')}"`);
      } else {
        lines.push(`${key}: ${value}`);
      }
    } else if (typeof value === 'boolean' || typeof value === 'number') {
      lines.push(`${key}: ${value}`);
    } else if (Array.isArray(value)) {
      lines.push(`${key}:`);
      for (const item of value) {
        lines.push(`  - ${item}`);
      }
    } else if (typeof value === 'object' && value !== null) {
      lines.push(`${key}: ${JSON.stringify(value)}`);
    } else {
      lines.push(`${key}: ${value}`);
    }
  }
  lines.push('---');
  return lines.join('\n');
}

export class SynapseFileManager implements SynapseFileManagerLike {
  constructor(private readonly vault: SynapseVaultLike) {}

  /**
   * Modifica el frontmatter YAML de un archivo de forma atómica sin corromper el contenido.
   */
  async processFrontMatter(
    file: TFile | string,
    fn: (frontmatter: Record<string, any>) => void
  ): Promise<void> {
    const path = pathOf(file);
    const content = await this.vault.read(path);

    const match = content.match(/^---\s*\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/);
    let frontmatterData: Record<string, any> = {};
    let body = content;

    if (match) {
      frontmatterData = parseYamlFrontmatter(match[1]);
      body = content.slice(match[0].length);
    }

    fn(frontmatterData);

    const newFrontmatterStr = stringifyYamlFrontmatter(frontmatterData);
    const updatedContent = `${newFrontmatterStr}\n${body.replace(/^\r?\n/, '')}`;

    await this.vault.modify(path, updatedContent);
  }

  /**
   * Calcula una ruta disponible sin colisiones para un archivo adjunto.
   */
  async getAvailablePathForAttachment(filename: string, extension: string): Promise<string> {
    const cleanExt = extension.replace(/^\./, '');
    let candidate = `${filename}.${cleanExt}`;
    let counter = 1;

    while (await this.vault.exists(candidate)) {
      candidate = `${filename} ${counter}.${cleanExt}`;
      counter += 1;
    }

    return candidate;
  }

  /**
   * Genera la sintaxis de enlace Markdown o wikilink hacia un archivo.
   */
  generateMarkdownLink(
    file: TFile | { path: string },
    _sourcePath: string,
    subpath?: string,
    alias?: string
  ): string {
    const name = file.path.split('/').pop() || file.path;
    const dot = name.lastIndexOf('.');
    const base = dot > 0 ? name.slice(0, dot) : name;
    const sub = subpath ? (subpath.startsWith('#') ? subpath : `#${subpath}`) : '';
    const al = alias ? `|${alias}` : '';

    return `[[${base}${sub}${al}]]`;
  }
}
