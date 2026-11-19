import { describe, expect, it } from 'bun:test';
import { buildRepoLogGraph, reachableFrom } from './repo-log-graph';

// Log de todas las ramas, del más nuevo al más antiguo:
//   f (feature)   m (main) = fusión de d y c
//   |             d
//   e             c (rama ya fusionada)
//    \            b
//     `---------- a (raíz)
const all = [
  { commit_hash: 'f', parents: ['e'] },
  { commit_hash: 'm', parents: ['d', 'c'] },
  { commit_hash: 'd', parents: ['b'] },
  { commit_hash: 'e', parents: ['a'] },
  { commit_hash: 'c', parents: ['b'] },
  { commit_hash: 'b', parents: ['a'] },
  { commit_hash: 'a', parents: [] },
];

describe('buildRepoLogGraph con todas las ramas', () => {
  it('cada punta de rama abre su carril y todos convergen en la raíz', () => {
    const rows = buildRepoLogGraph(all);
    const col = Object.fromEntries(all.map((c, i) => [c.commit_hash, rows[i].col]));
    expect(col.f).toBe(0);
    expect(col.m).toBe(1);
    // La línea de feature sigue en su carril hasta e
    expect(col.e).toBe(0);
    // La fusión abre un carril nuevo para c, distinto del de d
    expect(col.c).not.toBe(col.d);
    // c vuelve al carril de d (su padre b ya lo espera ahí) y b se une al carril de feature en a
    const idx = (h: string) => all.findIndex((c) => c.commit_hash === h);
    expect(rows[idx('c')].segments.filter((s) => s.half === 'bottom' && s.from === col.c).map((s) => s.to)).toEqual([col.d]);
    expect(rows[idx('b')].segments.filter((s) => s.half === 'bottom' && s.from === col.b).map((s) => s.to)).toEqual([col.a]);
    expect(rows.at(-1)!.segments.filter((s) => s.half === 'bottom')).toEqual([]);
  });

  it('ramas distintas llevan colores distintos', () => {
    const rows = buildRepoLogGraph(all);
    expect(rows[0].color).not.toBe(rows[1].color);
  });

  it('el ancho cubre todos los carriles abiertos a la vez', () => {
    const rows = buildRepoLogGraph(all);
    expect(Math.max(...rows.map((r) => r.width))).toBe(3);
  });

  it('un log cortado (padres fuera de la lista) no deja líneas colgando', () => {
    const rows = buildRepoLogGraph(all.slice(0, 3));
    expect(rows.flatMap((r) => r.segments).every((s) => s.from >= 0 && s.to >= 0)).toBe(true);
    expect(rows[2].segments.filter((s) => s.half === 'bottom')).toEqual([]);
  });
});

it('una rama que se une a un carril existente no lo corta: el carril sigue de largo', () => {
  // r (col 0) espera a «base»; t, otra punta (col 1), se une a ese carril en diagonal
  const rows = buildRepoLogGraph([
    { commit_hash: 'r', parents: ['base'] },
    { commit_hash: 't', parents: ['base'] },
    { commit_hash: 'base', parents: [] },
  ]);
  const bottoms = rows[1].segments.filter((s) => s.half === 'bottom').map((s) => [s.from, s.to]);
  expect(bottoms).toContainEqual([1, 0]);
  expect(bottoms).toContainEqual([0, 0]);
});

describe('reachableFrom', () => {
  it('filtra por rama conservando el orden', () => {
    expect(reachableFrom(all, 'f').map((c) => c.commit_hash)).toEqual(['f', 'e', 'a']);
    expect(reachableFrom(all, 'm').map((c) => c.commit_hash)).toEqual(['m', 'd', 'c', 'b', 'a']);
  });

  it('sobre una rama filtrada el grafo es lineal salvo en las fusiones', () => {
    const rows = buildRepoLogGraph(reachableFrom(all, 'f'));
    expect(rows.map((r) => r.col)).toEqual([0, 0, 0]);
  });

  it('una punta desconocida no devuelve nada', () => {
    expect(reachableFrom(all, 'zz')).toEqual([]);
  });
});
