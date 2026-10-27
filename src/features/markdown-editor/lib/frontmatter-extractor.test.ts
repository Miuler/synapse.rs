import { describe, it, expect } from 'vitest';
import { extractFrontmatterAndBody } from './frontmatter-extractor';

describe('extractFrontmatterAndBody', () => {
  it('retorna contenido intacto si no contiene frontmatter', () => {
    const raw = '# Título\n\nEste es un texto simple.';
    const res = extractFrontmatterAndBody(raw);
    expect(res.hasFrontmatter).toBe(false);
    expect(res.metadata).toEqual({});
    expect(res.cleanContent).toBe(raw);
  });

  it('extrae metadatos y separa el cuerpo limpiamente', () => {
    const raw = `---
title: Mi Nota
tags:
  - rust
  - svelte
draft: false
---

# Encabezado Principal

Contenido de la nota.`;

    const res = extractFrontmatterAndBody(raw);
    expect(res.hasFrontmatter).toBe(true);
    expect(res.metadata.title).toBe('Mi Nota');
    expect(res.metadata.tags).toEqual(['rust', 'svelte']);
    expect(res.metadata.draft).toBe(false);
    expect(res.cleanContent).toBe('\n# Encabezado Principal\n\nContenido de la nota.');
  });

  it('soporta frontmatter vacío o sin cuerpo', () => {
    const raw = `---
---`;
    const res = extractFrontmatterAndBody(raw);
    expect(res.hasFrontmatter).toBe(true);
    expect(res.metadata).toEqual({});
    expect(res.cleanContent).toBe('');
  });

  it('maneja strings vacías o nulos', () => {
    const res = extractFrontmatterAndBody('');
    expect(res.hasFrontmatter).toBe(false);
    expect(res.cleanContent).toBe('');
  });
});
