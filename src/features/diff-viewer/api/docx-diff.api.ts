import { invokeTauri } from '@shared/api';
import type { DiffSource } from '../lib/diff-sides';

export interface DocxText {
  text: string;
  heading: number | null;
  listLevel: number | null;
}
export interface DocxImage {
  /** Vacío si está fuera del documento o no se pudo leer. */
  sha256: string;
  filename: string;
  /** `null` si el formato no se puede mostrar (EMF, WMF…). */
  mime: string | null;
  alt: string;
  external: boolean;
}
export interface DocxCell {
  blocks: DocxBlock[];
  colSpan: number;
  merged: boolean;
}
export type DocxBlock =
  | ({ type: 'text' } & DocxText)
  | ({ type: 'image' } & DocxImage)
  | { type: 'table'; rows: DocxCell[][] };

export type DocxChangeKind = 'equal' | 'added' | 'removed' | 'modified';
export interface DocxSegment {
  kind: DocxChangeKind;
  text: string;
}
export interface DocxCellChange {
  kind: DocxChangeKind;
  colSpan: number;
  merged: boolean;
  entries: DocxDiffEntry[];
}
export type DocxBlockChange =
  | { type: 'text'; oldHeading: number | null; newHeading: number | null; listLevel: number | null; segments: DocxSegment[] }
  | { type: 'table'; rows: { kind: DocxChangeKind; cells: DocxCellChange[] }[] }
  | { type: 'image'; old: DocxImage; new: DocxImage };
export type DocxDiffEntry =
  | { kind: 'equal' | 'added' | 'removed'; block: DocxBlock }
  | { kind: 'modified'; change: DocxBlockChange };

export interface DocxDiff {
  oldExists: boolean;
  newExists: boolean;
  sameBytes: boolean;
  approximate: boolean;
  entries: DocxDiffEntry[];
}

export const docxDiffApi = {
  diff: (before: DiffSource, after: DiffSource) => invokeTauri<DocxDiff>('docx_diff', { before, after }),
  /** Vacío si el documento no existe en esa versión. */
  bytes: (source: DiffSource) => invokeTauri<ArrayBuffer>('docx_bytes', { source }),
  image: (source: DiffSource, sha256: string) => invokeTauri<string | null>('docx_image', { source, sha256 }),
};
