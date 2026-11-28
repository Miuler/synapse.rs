<script lang="ts">
  // Visor de hojas de cálculo (solo lectura). Rust lee el libro (umya para .xlsx con estilos,
  // calamine para .xls/.xlsb/.ods y los .xlsx muy grandes, solo con valores) y lo guarda en caché;
  // aquí se piden la lista de hojas, la forma de la hoja abierta y sus celdas por bloques de filas
  // según se desplaza. Solo se pintan las filas y columnas a la vista (más un margen).
  import { invokeTauri } from '@shared/api/tauri-client';
  import { systemApi } from '@shared/api';
  import { appSettings } from '@entities/settings';
  import { ZoomControl } from '@shared/ui/zoom-control';
  import { FindBar } from '@shared/ui/find-bar';
  import { t } from '@shared/lib/i18n';
  import { tick, untrack } from 'svelte';
  import { stepViewerZoom } from '@shared/lib/viewer-zoom';
  import { FunctionSquare, EyeOff, Eye, TriangleAlert, ExternalLink } from 'lucide-svelte';
  import type { WorkbookInfo, SheetLayout, CellBlock, Cell, SheetMatch, SheetMatches } from '../model/types';
  import { buildGeometry, visibleWindow, visibleMerges, colName, styleToCss, DEFAULT_ROW_PX } from '../lib/grid';

  const HEAD_H = 22;
  const HEAD_W = 46;

  interface Props {
    /** Ruta relativa a la bóveda */
    path: string;
    /** Cambia cuando el archivo cambió en disco: se vuelve a leer */
    reloadKey?: number;
  }
  let { path, reloadKey = 0 }: Props = $props();

  /** Filas por bloque pedido a Rust */
  const BLOCK = 200;

  // Posición recordada por archivo durante la sesión
  const memory: Map<string, { sheet: string; r: number; c: number }> = ((globalThis as any).__synapseSheetMemory ??= new Map());

  let info = $state<WorkbookInfo | null>(null);
  let layout = $state<SheetLayout | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);
  let sheetIndex = $state(0);
  let showHidden = $state(false);
  let showFormulas = $state(false);
  let sel = $state({ r: 1, c: 1 });
  let scroller = $state<HTMLDivElement | null>(null);
  let scrollTop = $state(0);
  let scrollLeft = $state(0);
  let viewHeight = $state(600);
  let viewWidth = $state(800);

  /** Bloques de celdas de la hoja abierta: celdas por fila y su CSS ya calculado */
  interface Block {
    rows: Map<number, Map<number, Cell>>;
    css: string[];
  }
  let blocks = new Map<number, Block>();
  let pending = new Set<number>();
  /** Se incrementa al llegar un bloque (los bloques no son reactivos por sí solos) */
  let blocksVersion = $state(0);
  /** Evita aplicar respuestas de una hoja o lectura anterior */
  let generation = 0;

  async function load() {
    const gen = ++generation;
    loading = true;
    blocks = new Map();
    pending = new Set();
    try {
      const wb = await invokeTauri<WorkbookInfo>('spreadsheet_open', { path });
      if (gen !== generation) return;
      info = wb;
      error = null;
      const m = memory.get(path);
      const idx = m ? wb.sheets.findIndex((s) => s.name === m.sheet) : -1;
      const first = Math.max(0, wb.sheets.findIndex((s) => !s.hidden));
      await openSheet(idx >= 0 ? idx : first, m && idx >= 0 ? { r: m.r, c: m.c } : undefined);
    } catch (e) {
      if (gen === generation) error = String(e);
    } finally {
      if (gen === generation) loading = false;
    }
  }

  async function openSheet(i: number, at?: { r: number; c: number }) {
    const gen = ++generation;
    sheetIndex = i;
    blocks = new Map();
    pending = new Set();
    blocksVersion++;
    try {
      const l = await invokeTauri<SheetLayout>('spreadsheet_layout', { path, sheet: i });
      if (gen !== generation) return;
      layout = l;
      error = null;
      sel = at ?? { r: 1, c: 1 };
      scrollTop = 0;
      scrollLeft = 0;
      if (scroller) {
        scroller.scrollTop = 0;
        scroller.scrollLeft = 0;
      }
      if (at) queueMicrotask(() => revealSelection());
    } catch (e) {
      if (gen === generation) error = String(e);
    }
  }

  async function fetchBlock(b: number) {
    if (blocks.has(b) || pending.has(b)) return;
    pending.add(b);
    const gen = generation;
    try {
      const block = await invokeTauri<CellBlock>('spreadsheet_cells', {
        path,
        sheet: sheetIndex,
        fromRow: b * BLOCK + 1,
        toRow: (b + 1) * BLOCK,
      });
      if (gen !== generation) return;
      const rows = new Map<number, Map<number, Cell>>();
      const colWidths = new Map(layout?.colWidths ?? []);
      const rowHeights = new Map(layout?.rowHeights ?? []);
      let heightsChanged = false;
      for (const cell of block.cells) {
        let row = rows.get(cell.r);
        if (!row) rows.set(cell.r, (row = new Map()));
        row.set(cell.c, cell);

        // Si la celda tiene ajuste de línea (`wrap`) o saltos `\n`, asegurar alto suficiente para no recortar la cabecera
        const st = cell.s !== undefined ? block.styles[cell.s] : undefined;
        if (cell.v && (st?.wrap || cell.v.includes('\n'))) {
          const colW = colWidths.get(cell.c);
          const colPx = colW ? Math.max(12, colW * 7 + 5) : 64;
          const fontPt = st?.size10 ? st.size10 / 10 : 11;
          const fontPx = (fontPt * 4) / 3;
          const charsPerLine = Math.max(4, Math.floor((colPx - 8) / (fontPx * 0.6)));
          let lines = 0;
          for (const seg of cell.v.split('\n')) {
            lines += Math.max(1, Math.ceil(seg.length / charsPerLine));
          }
          if (lines > 1) {
            const neededPt = Math.ceil(lines * fontPt * 1.35 + 4);
            const currentPt = rowHeights.get(cell.r) ?? 15;
            if (neededPt > currentPt) {
              rowHeights.set(cell.r, neededPt);
              heightsChanged = true;
            }
          }
        }
      }
      if (heightsChanged && layout) {
        layout = { ...layout, rowHeights: Array.from(rowHeights.entries()) };
      }
      blocks.set(b, { rows, css: block.styles.map(styleToCss) });
      blocksVersion++;
    } catch (e) {
      console.warn('No se pudo leer un tramo de la hoja:', e);
    } finally {
      pending.delete(b);
    }
  }

  $effect(() => {
    void reloadKey;
    void path;
    load();
  });

  const sheets = $derived(info?.sheets ?? []);
  const tabs = $derived(sheets.map((s, i) => ({ s, i })).filter(({ s }) => showHidden || !s.hidden));
  const hiddenCount = $derived(sheets.filter((s) => s.hidden).length);
  const sheetName = $derived(sheets[sheetIndex]?.name ?? '');

  $effect(() => {
    if (sheetName) memory.set(path, { sheet: sheetName, r: sel.r, c: sel.c });
  });

  // Zoom de toda la rejilla: se multiplican las medidas y la letra (no se escala la imagen, así
  // se siguen pintando solo las filas y columnas a la vista)
  const zoom = $derived(appSettings.sheetZoom);
  const headH = $derived(Math.round(HEAD_H * zoom));
  const headW = $derived(Math.round(HEAD_W * zoom));

  function setZoom(z: number) {
    const before = appSettings.sheetZoom;
    appSettings.setViewerZoom('sheet', z);
    const ratio = appSettings.sheetZoom / before;
    if (!scroller || ratio === 1) return;
    // Mantener a la vista el mismo sitio de la hoja
    const cx = scroller.scrollLeft + scroller.clientWidth / 2;
    const cy = scroller.scrollTop + scroller.clientHeight / 2;
    requestAnimationFrame(() => {
      if (!scroller) return;
      scroller.scrollLeft = cx * ratio - scroller.clientWidth / 2;
      scroller.scrollTop = cy * ratio - scroller.clientHeight / 2;
    });
  }

  // Ctrl+rueda: zoom (no pasivo para poder evitar el desplazamiento)
  $effect(() => {
    const el = scroller;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if (!e.ctrlKey || !e.deltaY) return;
      e.preventDefault();
      setZoom(stepViewerZoom(appSettings.sheetZoom, e.deltaY < 0 ? 1 : -1));
    };
    el.addEventListener('wheel', onWheel, { passive: false });
    return () => el.removeEventListener('wheel', onWheel);
  });

  const grid = $derived(layout ? buildGeometry(layout, zoom) : null);
  const win = $derived(grid ? visibleWindow(grid, scrollTop, scrollLeft, viewHeight, viewWidth) : null);
  const visRows = $derived(grid && win ? grid.rows.slice(win.rows.from, win.rows.to + 1) : []);
  const visCols = $derived(grid && win ? grid.cols.slice(win.cols.from, win.cols.to + 1) : []);
  const visMerges = $derived(
    grid ? visibleMerges(grid, scrollTop - 120, scrollTop + viewHeight + 120, scrollLeft - 100, scrollLeft + viewWidth + 100) : [],
  );
  /** Índice de la columna/fila en la geometría (para su posición) */
  const ci = (c: number) => grid!.colIndex.get(c)!;
  const ri = (r: number) => grid!.rowIndex.get(r)!;

  // Pedir los bloques de las filas a la vista
  $effect(() => {
    if (!layout || !visRows.length) return;
    const last = Math.min(visRows[visRows.length - 1], Math.max(layout.rows, 1));
    for (let b = Math.floor((visRows[0] - 1) / BLOCK); b <= Math.floor((last - 1) / BLOCK); b++) fetchBlock(b);
  });

  function cellAt(r: number, c: number): { cell: Cell; css: string } | null {
    void blocksVersion;
    const block = blocks.get(Math.floor((r - 1) / BLOCK));
    const cell = block?.rows.get(r)?.get(c);
    if (!block || !cell) return null;
    return { cell, css: cell.s != null ? (block.css[cell.s] ?? '') : '' };
  }

  const selected = $derived.by(() => {
    void blocksVersion;
    return cellAt(sel.r, sel.c)?.cell ?? null;
  });

  function cellCss(css: string, cell: Cell | undefined, box: string): string {
    const align = cell?.n && !showFormulas && !css.includes('text-align') ? ';text-align:right' : '';
    return `${box}${css ? ';' + css : ''}${align}`;
  }

  function cellText(cell: Cell | undefined): string {
    if (!cell) return '';
    if (cell.f && (showFormulas || !cell.v)) return `=${cell.f}`;
    return cell.v;
  }

  function revealSelection() {
    const g = grid;
    if (!g || !scroller) return;
    const i = g.rowIndex.get(sel.r);
    if (i != null) {
      const top = g.rowTop[i];
      const bottom = top + g.rowPx[i];
      if (top < scroller.scrollTop) scroller.scrollTop = top;
      else if (bottom > scroller.scrollTop + scroller.clientHeight) scroller.scrollTop = bottom - scroller.clientHeight;
    }
    const j = g.colIndex.get(sel.c);
    if (j != null) {
      const left = g.colLeft[j];
      const right = left + g.colPx[j];
      if (left < scroller.scrollLeft) scroller.scrollLeft = left;
      else if (right > scroller.scrollLeft + scroller.clientWidth) scroller.scrollLeft = right - scroller.clientWidth;
    }
  }

  function move(dr: number, dc: number) {
    const g = grid;
    if (!g) return;
    const ri = g.rowIndex.get(sel.r) ?? 0;
    const ci = g.colIndex.get(sel.c) ?? 0;
    sel = {
      r: g.rows[Math.min(g.rows.length - 1, Math.max(0, ri + dr))],
      c: g.cols[Math.min(g.cols.length - 1, Math.max(0, ci + dc))],
    };
    revealSelection();
  }

  // Búsqueda (Ctrl+F) en todo el libro: la hace Rust, aquí solo están las filas a la vista
  let searchOpen = $state(false);
  let searchRequest = $state(0);
  let query = $state('');
  let matches = $state.raw<SheetMatch[]>([]);
  let moreMatches = $state(false);
  let matchIndex = $state(0);
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let searchGen = 0;

  export function triggerSearch() {
    // Al reabrirla se repite lo último buscado
    if (!searchOpen && query) void search(true);
    searchOpen = true;
    searchRequest++;
  }
  function closeSearch() {
    searchOpen = false;
    clearTimeout(searchTimer);
    searchGen++;
    matches = [];
    scroller?.focus();
  }

  /** `fresh`: búsqueda nueva, va a la primera desde la celda elegida; si no, se conserva la posición. */
  async function search(fresh: boolean) {
    clearTimeout(searchTimer);
    searchTimer = undefined;
    const gen = ++searchGen;
    try {
      const found = query.trim()
        ? await invokeTauri<SheetMatches>('spreadsheet_find', { path, text: query, hiddenSheets: showHidden, formulas: showFormulas })
        : { matches: [], more: false };
      if (gen !== searchGen) return;
      matches = found.matches;
      moreMatches = found.more;
      if (!fresh) {
        matchIndex = Math.min(matchIndex, Math.max(0, matches.length - 1));
        return;
      }
      const after = matches.findIndex(
        (m) => m.sheet > sheetIndex || (m.sheet === sheetIndex && (m.r > sel.r || (m.r === sel.r && m.c >= sel.c))),
      );
      if (matches.length) await showMatch(Math.max(0, after));
      else matchIndex = 0;
    } catch (e) {
      console.warn('No se pudo buscar en el libro:', e);
    }
  }

  async function showMatch(i: number) {
    matchIndex = i;
    const m = matches[i];
    if (m.sheet !== sheetIndex) {
      await openSheet(m.sheet, { r: m.r, c: m.c });
      await tick();
    } else sel = { r: m.r, c: m.c };
    revealSelection();
  }

  function stepMatch(by: number) {
    if (searchTimer) void search(true);
    else if (matches.length) void showMatch((matchIndex + by + matches.length) % matches.length);
  }

  $effect(() => {
    void query;
    untrack(() => {
      if (!searchOpen) return;
      clearTimeout(searchTimer);
      searchTimer = setTimeout(() => search(true), 200);
    });
  });
  // Otro archivo, cambió en disco, o se muestran otras hojas o las fórmulas
  $effect(() => {
    void [path, reloadKey, showHidden, showFormulas];
    untrack(() => {
      if (searchOpen && query) void search(false);
    });
  });
  const foundHere = $derived(new Set(matches.filter((m) => m.sheet === sheetIndex).map((m) => `${m.r}:${m.c}`)));

  function switchSheet(delta: number) {
    if (!tabs.length) return;
    const pos = tabs.findIndex((t) => t.i === sheetIndex);
    const next = tabs[(pos + delta + tabs.length) % tabs.length];
    openSheet(next.i);
  }

  function onKeydown(e: KeyboardEvent) {
    // Ctrl+RePág/AvPág ya cambia de pestaña del editor: las hojas van con Alt o Ctrl+Mayús
    if ((e.altKey || (e.ctrlKey && e.shiftKey)) && (e.key === 'PageDown' || e.key === 'PageUp')) {
      e.stopPropagation();
      e.preventDefault();
      switchSheet(e.key === 'PageDown' ? 1 : -1);
      return;
    }
    if (e.ctrlKey && (e.key === '`' || e.code === 'Backquote')) {
      e.preventDefault();
      showFormulas = !showFormulas;
      return;
    }
    if (e.ctrlKey && e.key.toLowerCase() === 'c') {
      const v = showFormulas && selected?.f ? `=${selected.f}` : (selected?.v ?? '');
      navigator.clipboard?.writeText(v).catch(() => {});
      e.preventDefault();
      return;
    }
    const page = Math.max(1, Math.floor((scroller?.clientHeight ?? 400) / DEFAULT_ROW_PX) - 1);
    const moves: Record<string, [number, number]> = {
      ArrowUp: [-1, 0],
      ArrowDown: [1, 0],
      ArrowLeft: [0, -1],
      ArrowRight: [0, 1],
      Tab: [0, e.shiftKey ? -1 : 1],
      Enter: [e.shiftKey ? -1 : 1, 0],
      PageDown: [page, 0],
      PageUp: [-page, 0],
    };
    if (e.key === 'Home' || e.key === 'End') {
      e.preventDefault();
      const g = grid;
      if (!g || !layout) return;
      if (e.key === 'Home') sel = { r: e.ctrlKey ? g.rows[0] : sel.r, c: g.cols[0] };
      else sel = { r: e.ctrlKey ? Math.max(1, layout.rows) : sel.r, c: Math.max(1, layout.cols) };
      revealSelection();
      return;
    }
    const m = moves[e.key];
    if (m) {
      e.preventDefault();
      move(m[0], m[1]);
    }
  }
</script>

{#snippet cellView(r: number, c: number, box: string, merged: boolean)}
  {@const hit = cellAt(r, c)}
  {@const cell = hit?.cell}
  <div
    class="sheet-cell"
    class:merged
    class:selected={r === sel.r && c === sel.c}
    class:found={foundHere.has(`${r}:${c}`)}
    class:has-formula={!!cell?.f}
    class:no-cache={!!cell?.f && !cell.v}
    title={cell?.f && !cell.v ? t('spreadsheetViewer.uncomputedFormulaTitle') : undefined}
    style={cellCss(hit?.css ?? '', cell, box)}
    data-cell={`${colName(c)}${r}`}
    onpointerdown={() => (sel = { r, c })}
  >{cellText(cell)}</div>
{/snippet}

<div class="spreadsheet-viewer" data-path={path}>
  {#if loading && !info}
    <div class="sheet-message">{t('spreadsheetViewer.loadingWorkbook')}</div>
  {:else if error}
    <div class="sheet-message error"><TriangleAlert size={16} /> {error}</div>
  {:else if info}
    <div class="formula-bar">
      <span class="cell-ref" title={t('spreadsheetViewer.selectedCellTitle')}>{colName(sel.c)}{sel.r}</span>
      <span class="fx" class:active={!!selected?.f}><FunctionSquare size={14} /></span>
      <span class="formula-text" title={selected?.f ? `=${selected.f}` : (selected?.v ?? '')}>
        {#if selected?.f}={selected.f}{:else}{selected?.v ?? ''}{/if}
      </span>
      {#if selected?.f}
        <span class="formula-result" title={t('spreadsheetViewer.cachedResultTitle')}>
          {selected.v ? `→ ${selected.v}` : t('spreadsheetViewer.noCachedResult')}
        </span>
      {/if}
      <button
        type="button"
        class="sheet-btn show-formulas"
        class:active={showFormulas}
        title={t('spreadsheetViewer.showFormulasTitle')}
        onclick={() => (showFormulas = !showFormulas)}
      >
        <FunctionSquare size={14} /> {t('spreadsheetViewer.formulas')}
      </button>
      <ZoomControl {zoom} onChange={setZoom} />
      <button
        type="button"
        class="sheet-btn open-external"
        title={t('docxViewer.openExternalTitle')}
        onclick={() => systemApi.openFileWithDefaultApp(path).catch((e) => console.warn('No se pudo abrir con la aplicación del sistema:', e))}
      >
        <ExternalLink size={14} /> {t('docxViewer.openExternal')}
      </button>
    </div>

    {#if searchOpen}
      <FindBar
        bind:query
        count={matches.length}
        index={matchIndex}
        more={moreMatches}
        request={searchRequest}
        placeholder={t('find.workbook')}
        onStep={stepMatch}
        onClose={closeSearch}
      />
    {/if}

    {#if info.valuesOnly}
      <div class="sheet-notice">
        {#if info.large}{t('spreadsheetViewer.valuesOnlyLarge')}
        {:else}{t('spreadsheetViewer.valuesOnlyFormat')}{/if}
      </div>
    {/if}

    <div class="sheet-area">
      {#if !layout || !grid || !win}
        <div class="sheet-message">{t('spreadsheetViewer.loadingSheet')}</div>
      {:else if !layout.cellCount && !layout.merges.length}
        <div class="sheet-message">{t('spreadsheetViewer.emptySheet')}</div>
      {:else}
        <!-- Cabeceras fuera del área que se desplaza: se mueven con ella por transform -->
        <div class="corner" style={`width:${headW}px;height:${headH}px`}></div>
        <div class="col-heads" style={`left:${headW}px;height:${headH}px;font-size:${11 * zoom}px`}>
          <div class="heads-inner" style={`transform:translateX(${-scrollLeft}px)`}>
            {#each visCols as c (c)}
              <div class="col-head" class:sel={c === sel.c} style={`left:${grid.colLeft[ci(c)]}px;width:${grid.colPx[ci(c)]}px`}>
                {colName(c)}
              </div>
            {/each}
          </div>
        </div>
        <div class="row-heads" style={`top:${headH}px;width:${headW}px;font-size:${11 * zoom}px`}>
          <div class="heads-inner" style={`transform:translateY(${-scrollTop}px)`}>
            {#each visRows as r (r)}
              <div class="row-head" class:sel={r === sel.r} style={`top:${grid.rowTop[ri(r)]}px;height:${grid.rowPx[ri(r)]}px`}>{r}</div>
            {/each}
          </div>
        </div>
      {/if}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div
        class="sheet-scroll"
        class:formulas={showFormulas}
        style={`top:${headH}px;left:${headW}px`}
        tabindex="0"
        role="grid"
        aria-label={`Hoja ${sheetName}`}
        bind:this={scroller}
        bind:clientHeight={viewHeight}
        bind:clientWidth={viewWidth}
        onscroll={() => {
          scrollTop = scroller?.scrollTop ?? 0;
          scrollLeft = scroller?.scrollLeft ?? 0;
        }}
        onkeydown={onKeydown}
      >
        {#if layout && grid && win && (layout.cellCount || layout.merges.length)}
          <div class="sheet-canvas" style={`width:${grid.width}px;height:${grid.height}px;font-size:${(14.67 * zoom).toFixed(2)}px;--cell-pad:${(3 * zoom).toFixed(1)}px`}>
            {#each visRows as r (r)}
              {@const top = grid.rowTop[ri(r)]}
              {@const h = grid.rowPx[ri(r)]}
              {#each visCols as c (c)}
                {#if !grid.covered.has(`${r}:${c}`)}
                  {@render cellView(r, c, `left:${grid.colLeft[ci(c)]}px;top:${top}px;width:${grid.colPx[ci(c)]}px;height:${h}px`, false)}
                {/if}
              {/each}
            {/each}
            {#each visMerges as m (`${m.r1}:${m.c1}`)}
              {@render cellView(m.r1, m.c1, `left:${m.left}px;top:${m.top}px;width:${m.width}px;height:${m.height}px`, true)}
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <div class="sheet-tabs" role="tablist">
      {#each tabs as { s, i } (s.name)}
        <button
          type="button"
          role="tab"
          class="sheet-tab"
          class:active={i === sheetIndex}
          class:hidden-sheet={s.hidden}
          aria-selected={i === sheetIndex}
          title={s.hidden ? `${s.name} (oculta)` : s.name}
          style={i === sheetIndex && layout?.tabColor ? `--tab-color:${layout.tabColor}` : undefined}
          onclick={() => openSheet(i)}
        >
          {#if s.hidden}<EyeOff size={12} />{/if}{s.name}
        </button>
      {/each}
      {#if hiddenCount}
        <button
          type="button"
          class="sheet-btn toggle-hidden"
          title={showHidden ? t('spreadsheetViewer.hideHiddenSheets') : t('spreadsheetViewer.showHiddenSheets', { count: hiddenCount })}
          onclick={() => {
            showHidden = !showHidden;
            if (!showHidden && sheets[sheetIndex]?.hidden) openSheet(tabs[0]?.i ?? 0);
          }}
        >
          {#if showHidden}<EyeOff size={13} />{:else}<Eye size={13} />{/if}
          {hiddenCount === 1 ? t('spreadsheetViewer.hiddenOne', { count: hiddenCount }) : t('spreadsheetViewer.hiddenMany', { count: hiddenCount })}
        </button>
      {/if}
      <span class="sheet-hint">{t('spreadsheetViewer.readOnlyHint')}</span>
    </div>
  {/if}
</div>

<style>
  .spreadsheet-viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 13px;
  }
  .sheet-message {
    padding: 24px;
    color: var(--text-secondary);
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .sheet-message.error {
    color: var(--danger);
  }
  .formula-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    min-height: 30px;
  }
  .cell-ref {
    min-width: 56px;
    padding: 2px 6px;
    border: 1px solid var(--border-primary);
    border-radius: 4px;
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    text-align: center;
  }
  .fx {
    display: inline-flex;
    color: var(--text-secondary);
    opacity: 0.5;
  }
  .fx.active {
    color: var(--accent);
    opacity: 1;
  }
  .formula-text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono, monospace);
    font-size: 12px;
  }
  .formula-result {
    color: var(--text-secondary);
    font-size: 12px;
    white-space: nowrap;
  }
  .sheet-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border: 1px solid var(--border-primary);
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
  }
  .sheet-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .sheet-btn.active {
    border-color: var(--accent);
    color: var(--accent);
  }
  .sheet-notice {
    padding: 4px 10px;
    font-size: 12px;
    color: var(--text-secondary);
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-primary);
  }
  .sheet-area {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background: var(--bg-secondary);
  }
  .sheet-area > .sheet-message {
    position: absolute;
    inset: 0;
    z-index: 3;
    background: var(--bg-primary);
  }
  .sheet-scroll {
    position: absolute;
    right: 0;
    bottom: 0;
    overflow: auto;
    outline: none;
    /* La rejilla es «papel»: los colores del archivo están pensados para fondo blanco */
    background: #fff;
    contain: strict;
  }
  .sheet-canvas {
    position: relative;
    color: #000;
    font-family: Calibri, Carlito, 'Segoe UI', Arial, sans-serif;
    line-height: 1.2;
  }
  .sheet-cell {
    position: absolute;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    padding: 0 var(--cell-pad, 3px);
    border-right: 1px solid #e1e1e1;
    border-bottom: 1px solid #e1e1e1;
    overflow: hidden;
    white-space: nowrap;
    contain: strict;
  }
  .sheet-cell.merged {
    background: #fff;
    z-index: 1;
  }
  .sheet-cell.found {
    box-shadow: inset 0 0 0 100vmax rgba(255, 223, 93, 0.45); /* fijo: resaltado de búsqueda, no depende del tema */
  }
  .sheet-cell.selected {
    outline: 2px solid #1a73e8;
    outline-offset: -2px;
    z-index: 2;
  }
  .sheet-cell.has-formula::after {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    border-top: 5px solid #7cb342;
    border-right: 5px solid transparent;
  }
  .sheet-cell.no-cache {
    color: #8a8a8a;
    font-style: italic;
  }
  .sheet-scroll.formulas .sheet-cell.has-formula {
    font-family: var(--font-mono, monospace);
    font-size: 0.82em;
    text-align: left !important;
  }
  .corner,
  .col-heads,
  .row-heads {
    position: absolute;
    overflow: hidden;
    background: var(--bg-secondary);
    color: var(--text-secondary);
  }
  .corner {
    top: 0;
    left: 0;
    border-right: 1px solid var(--border-primary);
    border-bottom: 1px solid var(--border-primary);
  }
  .col-heads {
    top: 0;
    right: 0;
    border-bottom: 1px solid var(--border-primary);
  }
  .row-heads {
    left: 0;
    bottom: 0;
    border-right: 1px solid var(--border-primary);
  }
  .heads-inner {
    position: absolute;
    inset: 0;
    will-change: transform;
  }
  .col-head,
  .row-head {
    position: absolute;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .col-head {
    top: 0;
    height: 100%;
    border-right: 1px solid var(--border-primary);
  }
  .row-head {
    left: 0;
    width: 100%;
    border-bottom: 1px solid var(--border-primary);
  }
  .col-head.sel,
  .row-head.sel {
    color: var(--accent);
    font-weight: 600;
  }
  .sheet-tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px 6px;
    border-top: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    overflow-x: auto;
    min-height: 30px;
  }
  .sheet-tab {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 12px;
    border: 1px solid transparent;
    border-bottom: 3px solid var(--tab-color, transparent);
    border-radius: 4px 4px 0 0;
    background: transparent;
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
  }
  .sheet-tab:hover {
    background: var(--bg-hover);
  }
  .sheet-tab.active {
    background: var(--bg-primary);
    color: var(--text-primary);
    border-color: var(--border-primary);
    font-weight: 600;
  }
  .sheet-tab.hidden-sheet {
    font-style: italic;
    opacity: 0.75;
  }
  .sheet-hint {
    margin-left: auto;
    padding-left: 12px;
    color: var(--text-secondary);
    font-size: 11px;
    white-space: nowrap;
  }
</style>
