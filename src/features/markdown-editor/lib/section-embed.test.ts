import { describe, expect, it } from 'bun:test';
import { expandSectionEmbeds, extractSection, hasSectionEmbeds } from './section-embed';

const PLAN = [
  '---',
  'tags: [plan]',
  '---',
  '# Plan',
  'Intro.',
  '## Presupuesto',
  'Cifras del presupuesto.',
  '### Gastos fijos',
  'Alquiler.',
  '```sh',
  '## no es un titulo',
  '```',
  '## Metas',
  'Crecer.',
].join('\n');

const loaderFor = (notes: Record<string, string>) => ({
  resolvePath: async (target: string) => (notes[`${target}.md`] !== undefined ? `${target}.md` : null),
  readContent: async (path: string) => notes[path] ?? null,
});

describe('section-embed', () => {
  it('la sección llega hasta el siguiente encabezado del mismo nivel e incluye los subtítulos', () => {
    expect(extractSection(PLAN, 'Presupuesto')).toBe(
      '## Presupuesto\nCifras del presupuesto.\n### Gastos fijos\nAlquiler.\n```sh\n## no es un titulo\n```',
    );
    // Un subtítulo termina en el siguiente de su nivel o superior
    expect(extractSection(PLAN, 'Gastos fijos')).toBe('### Gastos fijos\nAlquiler.\n```sh\n## no es un titulo\n```');
    // El último título llega hasta el final
    expect(extractSection(PLAN, 'Metas')).toBe('## Metas\nCrecer.');
    expect(extractSection(PLAN, 'No existe')).toBeNull();
  });

  it('reemplaza ![[nota#Título]] por la sección dentro de un contenedor', async () => {
    const out = await expandSectionEmbeds('Antes\n![[Plan#Metas]]\nDespués', loaderFor({ 'Plan.md': PLAN }), 'x.md');
    expect(out).toContain('<div class="note-embed" data-embed-source="Plan.md#Metas">');
    expect(out).toContain('## Metas\nCrecer.');
    expect(out.startsWith('Antes\n')).toBe(true);
    expect(out.endsWith('\nDespués')).toBe(true);
  });

  it('deja intactos los embeds que no puede resolver, las imágenes y lo que está en código', async () => {
    const text = '![[Falta#Algo]]\n![[Plan#Inexistente]]\n![[foto.png]]\n![[Plan]]\n```\n![[Plan#Metas]]\n```';
    expect(await expandSectionEmbeds(text, loaderFor({ 'Plan.md': PLAN }), 'x.md')).toBe(text);
  });

  it('no entra en ciclos y limita la profundidad', async () => {
    const loop = { 'A.md': '# A\n![[B#B]]', 'B.md': '# B\n![[A#A]]' };
    const out = await expandSectionEmbeds('![[A#A]]', loaderFor(loop), 'x.md');
    expect(out.split('class="note-embed"').length - 1).toBe(2);
    expect(out).toContain('![[A#A]]'); // el que cerraría el ciclo queda sin expandir
  });

  it('detecta si hay algo que expandir', () => {
    expect(hasSectionEmbeds('![[Plan#Metas]]')).toBe(true);
    expect(hasSectionEmbeds('![[Plan]] ![[foto.png]] [[Plan#Metas]]')).toBe(false);
  });
});
