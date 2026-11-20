import { describe, expect, it } from 'bun:test';
import { bisect, buildGeometry, colName, styleToCss, visibleMerges, visibleWindow } from './grid';
import type { SheetLayout } from '../model/types';

const layout = (over: Partial<SheetLayout> = {}): SheetLayout => ({
  rows: 1000,
  cols: 100,
  cellCount: 0,
  merges: [],
  colWidths: [],
  rowHeights: [],
  hiddenCols: [],
  hiddenRows: [],
  ...over,
});

describe('rejilla', () => {
  it('nombres de columna', () => {
    expect([1, 26, 27, 52, 703].map(colName)).toEqual(['A', 'Z', 'AA', 'AZ', 'AAA']);
  });

  it('bisect', () => {
    expect(bisect([0, 20, 40], 25)).toBe(1);
    expect(bisect([0, 20, 40], 0)).toBe(0);
    expect(bisect([0, 20, 40], 99)).toBe(2);
  });

  it('salta filas y columnas ocultas y aplica medidas', () => {
    const g = buildGeometry(layout({ hiddenRows: [2], hiddenCols: [3], colWidths: [[1, 20]], rowHeights: [[1, 30]] }));
    expect(g.rows.slice(0, 3)).toEqual([1, 3, 4]);
    expect(g.cols.slice(0, 3)).toEqual([1, 2, 4]);
    expect(g.colPx[0]).toBe(145);
    expect(g.rowPx[0]).toBe(40);
  });

  it('pinta solo un tramo de filas y columnas', () => {
    const g = buildGeometry(layout());
    const w = visibleWindow(g, 10_000, 3_000, 600, 800);
    const nRows = w.rows.to - w.rows.from + 1;
    const nCols = w.cols.to - w.cols.from + 1;
    expect(nRows).toBeLessThan(100);
    expect(nCols).toBeLessThan(40);
    expect(w.rows.padStart).toBe(g.rowTop[w.rows.from]);
    expect(w.rows.padStart + g.rowTop[w.rows.to] - g.rowTop[w.rows.from] + g.rowPx[w.rows.to] + w.rows.padEnd).toBe(g.height);
  });

  it('coloca las combinadas y encuentra las que tocan el tramo aunque empiecen antes', () => {
    const g = buildGeometry(layout({ hiddenRows: [3], merges: [{ r1: 2, c1: 2, r2: 900, c2: 60 }, { r1: 5, c1: 1, r2: 5, c2: 2 }] }));
    const big = g.merges[0];
    expect(big.top).toBe(20);
    expect(big.left).toBe(64);
    expect(big.height).toBe(898 * 20);
    expect(big.width).toBe(59 * 64);
    expect(g.covered.has('3:3') && g.covered.has('2:2')).toBe(true);
    expect(visibleMerges(g, 10_000, 10_600, 3_000, 3_800)).toEqual([big]);
  });

  it('CSS de estilos', () => {
    expect(styleToCss({ bold: true, bg: '#ffff00', align: 'center', size10: 140 })).toBe(
      'font-weight:700;background:#ffff00;font-size:18.7px;text-align:center',
    );
  });
});
