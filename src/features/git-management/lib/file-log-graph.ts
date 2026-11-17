// Grafo de ramas del historial de un archivo (como `git log --graph`): cada commit ocupa una
// columna y las líneas unen cada commit con sus padres. Se calcula fila a fila; cada fila se dibuja
// en dos mitades: de arriba al punto del commit y del punto hacia abajo.

export interface GraphCommit {
  commit_hash: string;
  parents?: string[];
}

/** Un tramo de línea dentro de una fila, en columnas: `from` arriba (o en el punto), `to` abajo. */
export interface GraphSegment {
  from: number;
  to: number;
  /** Mitad de la fila: `top` (borde superior → punto) o `bottom` (punto → borde inferior). */
  half: 'top' | 'bottom';
  color: number;
}

export interface GraphRow {
  /** Columna del punto del commit. */
  col: number;
  color: number;
  segments: GraphSegment[];
  /** Columnas que ocupa la fila (para el ancho de la celda). */
  width: number;
}

interface Lane {
  hash: string;
  color: number;
}

export function buildFileLogGraph(commits: GraphCommit[]): GraphRow[] {
  const known = new Set(commits.map((c) => c.commit_hash));
  let lanes: (Lane | null)[] = [];
  let nextColor = 0;
  const free = () => {
    const i = lanes.indexOf(null);
    return i === -1 ? lanes.length : i;
  };

  return commits.map((c) => {
    const segments: GraphSegment[] = [];
    let col = lanes.findIndex((l) => l?.hash === c.commit_hash);
    let color: number;
    if (col === -1) {
      col = free();
      color = nextColor++;
    } else {
      color = lanes[col]!.color;
    }

    // Mitad superior: las ramas que esperan este commit convergen en su punto; las demás siguen
    lanes.forEach((l, j) => {
      if (!l) return;
      segments.push({ from: j, to: l.hash === c.commit_hash ? col : j, half: 'top', color: l.color });
    });

    // Después de este commit, su columna sigue con el primer padre y los otros abren o reusan ramas
    const after = lanes.map((l) => (l?.hash === c.commit_hash ? null : l));
    while (after.length <= col) after.push(null);
    const parents = (c.parents ?? []).filter((p) => known.has(p));
    parents.forEach((p, k) => {
      const existing = after.findIndex((l) => l?.hash === p);
      if (existing !== -1) {
        segments.push({ from: col, to: existing, half: 'bottom', color: after[existing]!.color });
        return;
      }
      let target: number;
      let laneColor: number;
      if (k === 0 && !after[col]) {
        target = col;
        laneColor = color;
      } else {
        target = after.indexOf(null);
        if (target === -1) target = after.length;
        laneColor = nextColor++;
      }
      after[target] = { hash: p, color: laneColor };
      segments.push({ from: col, to: target, half: 'bottom', color: laneColor });
    });
    after.forEach((l, j) => {
      if (l && !segments.some((s) => s.half === 'bottom' && s.to === j)) {
        segments.push({ from: j, to: j, half: 'bottom', color: l.color });
      }
    });

    const width = Math.max(lanes.length, after.length, col + 1);
    while (after.length && !after[after.length - 1]) after.pop();
    lanes = after;
    return { col, color, segments, width };
  });
}
