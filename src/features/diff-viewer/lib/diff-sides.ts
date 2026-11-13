// Qué se compara en cada diff: una versión «antes» y otra «después» del archivo. Así los visores
// visuales (2 Paneles y Diferencias) sirven igual para el disco contra Git, el stage contra HEAD,
// el parche de un commit (contra su padre) y la comparación de dos archivos.
import { vaultRepository } from '@shared/repositories';

/** De dónde sale una versión del archivo. */
export type DiffSource =
  | { kind: 'disk'; path: string }
  | { kind: 'rev'; path: string; rev: string }
  | { kind: 'staged'; path: string };

export interface DiffSides {
  before: DiffSource;
  after: DiffSource;
  beforeLabel: string;
  afterLabel: string;
}

export interface DiffTarget {
  filePath: string;
  isStaged?: boolean;
  baseRevision?: string;
  /** Compara `filePath` con este otro archivo de la bóveda. */
  compareWith?: string;
  /** Parche de un commit: el archivo en ese commit contra su commit padre. */
  commitRevision?: string;
}

const short = (rev: string) => (/^[0-9a-f]{8,}$/i.test(rev) ? rev.slice(0, 7) : rev);
const nameOf = (p: string) => p.split('/').pop() || p;

export function diffSides(t: DiffTarget): DiffSides {
  const path = t.filePath;
  if (t.compareWith) {
    return {
      before: { kind: 'disk', path: t.compareWith },
      after: { kind: 'disk', path },
      beforeLabel: nameOf(t.compareWith),
      afterLabel: nameOf(path),
    };
  }
  if (t.commitRevision) {
    return {
      before: { kind: 'rev', path, rev: `${t.commitRevision}^` },
      after: { kind: 'rev', path, rev: t.commitRevision },
      beforeLabel: `Antes (${short(t.commitRevision)}^)`,
      afterLabel: `Commit ${short(t.commitRevision)}`,
    };
  }
  if (t.isStaged) {
    // Como `git diff --staged`: el stage contra HEAD
    return {
      before: { kind: 'rev', path, rev: 'HEAD' },
      after: { kind: 'staged', path },
      beforeLabel: 'HEAD (Último commit)',
      afterLabel: 'Stage / Index',
    };
  }
  const rev = t.baseRevision || 'HEAD';
  return {
    before: { kind: 'rev', path, rev },
    after: { kind: 'disk', path },
    beforeLabel: t.baseRevision ? `REV: ${short(rev)}` : 'HEAD (Último commit)',
    afterLabel: 'Disco Duro (Local)',
  };
}

export interface LoadedSide {
  exists: boolean;
  content: string | null;
  dataUrl: string | null;
}

/** Lee una versión del archivo: texto, o data URL si es binario (`binary`). */
export async function loadSide(source: DiffSource, binary: boolean): Promise<LoadedSide> {
  if (source.kind === 'disk') {
    if (binary) {
      const dataUrl = await vaultRepository.readAssetDataUrl(source.path);
      return { exists: dataUrl !== null, content: null, dataUrl };
    }
    const note = await vaultRepository.readNote(source.path).catch(() => null);
    return { exists: note !== null, content: note?.content ?? '', dataUrl: null };
  }
  const rev = await vaultRepository.getGitFileRevision(
    source.path,
    source.kind === 'staged',
    source.kind === 'rev' ? source.rev : undefined,
  );
  return { exists: !!rev?.exists, content: rev?.content ?? null, dataUrl: rev?.data_url ?? null };
}
