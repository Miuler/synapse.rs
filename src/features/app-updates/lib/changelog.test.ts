import { describe, expect, it } from 'bun:test';
import { newsMarkdown, newsToOpenOnStartup, parseChangelog, previousVersion } from './changelog';

const RAW = `# Changelog

Intro.

## [Sin publicar] (Unreleased)

- algo pendiente

## [1.0.0-beta.3] - 2026-11-20

### Añadido

- tres

## [1.0.0-beta.2] - 2026-11-19

- dos

## [1.0.0-beta.1] - 2026-11-18

- uno

[1.0.0-beta.3]: https://github.com/x/y/compare/a...b
`;

describe('changelog', () => {
  it('lee las versiones publicadas, sin «Sin publicar» ni los enlaces del final', () => {
    const s = parseChangelog(RAW);
    expect(s.map((x) => x.version)).toEqual(['1.0.0-beta.3', '1.0.0-beta.2', '1.0.0-beta.1']);
    expect(s[0].markdown).toBe('## [1.0.0-beta.3] - 2026-11-20\n\n### Añadido\n\n- tres');
    expect(s[2].markdown).not.toContain('compare');
  });

  it('novedades desde una versión hasta la actual', () => {
    const md = newsMarkdown(RAW, '1.0.0-beta.3', '1.0.0-beta.1');
    expect(md.startsWith('# Novedades de Synapse desde 1.0.0-beta.1')).toBe(true);
    expect(md).toContain('- tres');
    expect(md).toContain('- dos');
    expect(md).not.toContain('- uno');
    expect(md).not.toContain('pendiente');
  });

  it('sin versión de partida, todo el historial; versión desconocida empieza por la más nueva', () => {
    expect(newsMarkdown(RAW, '1.0.0-beta.2')).toContain('- uno');
    expect(newsMarkdown(RAW, '1.0.0-beta.2')).not.toContain('- tres');
    expect(newsMarkdown(RAW, '9.9.9')).toContain('- tres');
  });

  it('versión anterior', () => {
    expect(previousVersion(RAW, '1.0.0-beta.3')).toBe('1.0.0-beta.2');
    expect(previousVersion(RAW, '1.0.0-beta.1')).toBeUndefined();
  });

  it('cuándo abrir las novedades al arrancar', () => {
    expect(newsToOpenOnStartup(RAW, '1.0.0-beta.3', '1.0.0-beta.3', true)).toBeNull();
    expect(newsToOpenOnStartup(RAW, '1.0.0-beta.3', '1.0.0-beta.1', true)).toEqual({ since: '1.0.0-beta.1' });
    expect(newsToOpenOnStartup(RAW, '1.0.0-beta.3', null, true)).toEqual({ since: '1.0.0-beta.2' });
    expect(newsToOpenOnStartup(RAW, '1.0.0-beta.3', null, false)).toBeNull();
  });
});
