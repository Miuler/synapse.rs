import { expect, it } from 'bun:test';
import { needsRenderedPixelDiff, renderKindFrom, type FileKindFlags } from './render-kind';

const flags = (f: Partial<FileKindFlags>): FileKindFlags => ({ image: false, drawing: false, diagram: false, drawio: false, pdf: false, ...f });

it('reconoce lo que se puede convertir en imagen', () => {
  expect(renderKindFrom(flags({ image: true }))).toBe('image');
  expect(renderKindFrom(flags({ drawing: true, image: true }))).toBe('drawing');
  expect(renderKindFrom(flags({ diagram: true }))).toBe('diagram');
  expect(renderKindFrom(flags({ drawio: true }))).toBe('drawio');
  expect(renderKindFrom(flags({ pdf: true }))).toBeNull();
  expect(renderKindFrom(flags({}))).toBeNull();
});

it('renderiza los dos lados si son de tipos distintos o draw.io', () => {
  expect(needsRenderedPixelDiff('image', 'drawing', false)).toBe(true);
  expect(needsRenderedPixelDiff('diagram', 'diagram', false)).toBe(true);
  expect(needsRenderedPixelDiff('drawio', 'drawio', true)).toBe(true);
  expect(needsRenderedPixelDiff('diagram', 'diagram', true)).toBe(false);
  expect(needsRenderedPixelDiff('image', 'image', true)).toBe(false);
});

it('si un lado no se puede dibujar se queda el diff de texto', () => {
  expect(needsRenderedPixelDiff('image', null, false)).toBe(false);
  expect(needsRenderedPixelDiff(null, 'drawio', false)).toBe(false);
});
