import { describe, expect, it } from 'bun:test';
import { extractWikilinkTokens, isNoteTarget, resolveMarkdownWikilinks } from './link-resolver';

describe('link-resolver: enlaces con encabezado y embeds de notas', () => {
  it('extrae el ancla de [[nota#Título]] y marca los embeds', () => {
    const [link, embed] = extractWikilinkTokens('Ir a [[Plan#Presupuesto anual]] y ![[Plan#Meta|la meta]]');
    expect(link).toMatchObject({ target: 'Plan', anchor: 'Presupuesto anual', isEmbed: false, isImage: false });
    expect(embed).toMatchObject({ target: 'Plan', anchor: 'Meta', alias: 'la meta', isEmbed: true, isImage: false });
    // El token del embed incluye el `!` para que el widget lo reemplace entero
    expect(embed.raw.startsWith('![[')).toBe(true);
  });

  it('distingue notas de otros recursos', () => {
    expect(isNoteTarget('Plan')).toBe(true);
    expect(isNoteTarget('carpeta/Plan.md#Título')).toBe(true);
    expect(isNoteTarget('foto.png')).toBe(false);
    expect(isNoteTarget('dibujo.excalidraw')).toBe(false);
    expect(isNoteTarget('manual.pdf')).toBe(false);
  });

  it('renderiza un embed de nota como enlace y un embed de imagen como imagen', async () => {
    const out = await resolveMarkdownWikilinks('![[Plan#Meta]] ![[foto.png]]');
    expect(out).toBe('[Plan#Meta](Plan.md#Meta) ![foto.png](foto.png)');
  });
});
