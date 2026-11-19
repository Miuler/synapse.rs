// Utilidades del log de todas las ramas: el grafo de carriles es el mismo del historial de un
// archivo (`buildFileLogGraph`); aquí se filtra por rama sin romper las líneas.
export { buildFileLogGraph as buildRepoLogGraph } from './file-log-graph';
import type { GraphCommit } from './file-log-graph';

/**
 * Commits alcanzables desde `tip` siguiendo a los padres, en el mismo orden que `commits`.
 * Es lo que muestra `git log <rama>`: el conjunto queda cerrado por ancestros, así que el grafo
 * se puede volver a calcular sobre él.
 */
export function reachableFrom<T extends GraphCommit>(commits: T[], tip: string): T[] {
  const byHash = new Map(commits.map((c) => [c.commit_hash, c]));
  const seen = new Set<string>();
  const stack = [tip];
  while (stack.length) {
    const h = stack.pop()!;
    if (seen.has(h) || !byHash.has(h)) continue;
    seen.add(h);
    stack.push(...(byHash.get(h)!.parents ?? []));
  }
  return commits.filter((c) => seen.has(c.commit_hash));
}
