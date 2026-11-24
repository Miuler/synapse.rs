/** Distancia entre filas consecutivas a partir de su posición superior: la mediana, que ignora alguna fila más alta. */
export function medianRowPitch(tops: number[]): number {
  const gaps = tops.slice(1).map((top, i) => top - tops[i]).sort((a, b) => a - b);
  return gaps.length ? gaps[gaps.length >> 1] : 0;
}
