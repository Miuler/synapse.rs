<script lang="ts">
  import { tick, untrack } from 'svelte';
  import {
    Table,
    Plus,
    Trash2,
    AlignLeft,
    AlignCenter,
    AlignRight,
    Code,
    ChevronDown,
  } from 'lucide-svelte';
  import type { TableData, ColumnAlignment } from '../lib/table-parser';

  interface Props {
    initialData: TableData;
    onUpdate?: (data: TableData) => void;
    onSwitchToSource?: () => void;
  }

  let { initialData, onUpdate, onSwitchToSource }: Props = $props();

  let headers = $state<string[]>(untrack(() => [...initialData.headers]));
  let alignments = $state<ColumnAlignment[]>(untrack(() => [...initialData.alignments]));
  let rows = $state<string[][]>(untrack(() => initialData.rows.map((r) => [...r])));
  let containerRef = $state<HTMLDivElement | null>(null);
  let isInternalChange = false;

  $effect(() => {
    const data = initialData;
    if (!isInternalChange) {
      untrack(() => {
        headers = [...data.headers];
        alignments = [...data.alignments];
        rows = data.rows.map((r) => [...r]);
      });
    }
  });

  export function setData(data: TableData) {
    if (isInternalChange) return;
    headers = [...data.headers];
    alignments = [...data.alignments];
    rows = data.rows.map((r) => [...r]);
  }

  function emitUpdate() {
    isInternalChange = true;
    if (onUpdate) {
      onUpdate({
        headers: $state.snapshot(headers),
        alignments: $state.snapshot(alignments),
        rows: $state.snapshot(rows),
      });
    }
    setTimeout(() => {
      isInternalChange = false;
    }, 60);
  }

  function updateHeader(index: number, val: string) {
    headers[index] = val;
    emitUpdate();
  }

  function updateCell(rIndex: number, cIndex: number, val: string) {
    if (!rows[rIndex]) return;
    rows[rIndex][cIndex] = val;
    emitUpdate();
  }

  function addColumn(afterIndex?: number) {
    const insertIdx = afterIndex !== undefined ? afterIndex + 1 : headers.length;
    headers.splice(insertIdx, 0, `Columna ${headers.length + 1}`);
    alignments.splice(insertIdx, 0, 'none');
    for (const r of rows) {
      r.splice(insertIdx, 0, '');
    }
    emitUpdate();
    focusHeader(insertIdx);
  }

  function deleteColumn(index: number) {
    if (headers.length <= 1) return;
    headers.splice(index, 1);
    alignments.splice(index, 1);
    for (const r of rows) {
      r.splice(index, 1);
    }
    emitUpdate();
  }

  function cycleAlignment(colIndex: number) {
    const current = alignments[colIndex] || 'none';
    let next: ColumnAlignment = 'none';
    if (current === 'none') next = 'left';
    else if (current === 'left') next = 'center';
    else if (current === 'center') next = 'right';
    else next = 'none';

    alignments[colIndex] = next;
    emitUpdate();
  }

  function addRow(afterIndex?: number) {
    const insertIdx = afterIndex !== undefined ? afterIndex + 1 : rows.length;
    const newRow = new Array(headers.length).fill('');
    rows.splice(insertIdx, 0, newRow);
    emitUpdate();
    focusCell(insertIdx, 0);
  }

  function deleteRow(index: number) {
    rows.splice(index, 1);
    emitUpdate();
  }

  function focusCell(r: number, c: number) {
    tick().then(() => {
      const el = containerRef?.querySelector<HTMLInputElement>(
        `input[data-row="${r}"][data-col="${c}"]`
      );
      if (el) {
        el.focus();
        el.select();
      }
    });
  }

  function focusHeader(c: number) {
    tick().then(() => {
      const el = containerRef?.querySelector<HTMLInputElement>(
        `input[data-header="${c}"]`
      );
      if (el) {
        el.focus();
        el.select();
      }
    });
  }

  function handleHeaderKeyDown(e: KeyboardEvent, c: number) {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (rows.length === 0) {
        addRow();
      } else {
        focusCell(0, c);
      }
    } else if (e.key === 'Tab') {
      if (e.shiftKey) {
        if (c > 0) {
          e.preventDefault();
          focusHeader(c - 1);
        }
      } else {
        if (c < headers.length - 1) {
          e.preventDefault();
          focusHeader(c + 1);
        } else if (rows.length > 0) {
          e.preventDefault();
          focusCell(0, 0);
        } else {
          e.preventDefault();
          addRow();
        }
      }
    } else if (e.key === 'ArrowDown') {
      if (rows.length > 0) {
        e.preventDefault();
        focusCell(0, c);
      }
    }
  }

  function handleCellKeyDown(e: KeyboardEvent, r: number, c: number) {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (r < rows.length - 1) {
        focusCell(r + 1, c);
      } else {
        addRow(r);
      }
    } else if (e.key === 'Tab') {
      e.preventDefault();
      if (e.shiftKey) {
        if (c > 0) {
          focusCell(r, c - 1);
        } else if (r > 0) {
          focusCell(r - 1, headers.length - 1);
        } else {
          focusHeader(headers.length - 1);
        }
      } else {
        if (c < headers.length - 1) {
          focusCell(r, c + 1);
        } else if (r < rows.length - 1) {
          focusCell(r + 1, 0);
        } else {
          addRow();
        }
      }
    } else if (e.key === 'ArrowDown') {
      if (r < rows.length - 1) {
        e.preventDefault();
        focusCell(r + 1, c);
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (r > 0) {
        focusCell(r - 1, c);
      } else {
        focusHeader(c);
      }
    }
  }
</script>

<div class="table-card" bind:this={containerRef}>
  <!-- Barra superior de herramientas de tabla -->
  <div class="table-card-header">
    <div class="table-badge">
      <Table size={13} />
      <span>Tabla</span>
      <span class="table-stats-pill">
        {rows.length} {rows.length === 1 ? 'fila' : 'filas'} × {headers.length} {headers.length === 1 ? 'columna' : 'columnas'}
      </span>
    </div>

    <div class="table-actions">
      <button
        type="button"
        class="table-btn"
        onclick={() => addRow()}
        title="Añadir fila al final"
      >
        <Plus size={12} />
        <span>Fila</span>
      </button>

      <button
        type="button"
        class="table-btn"
        onclick={() => addColumn()}
        title="Añadir columna a la derecha"
      >
        <Plus size={12} />
        <span>Columna</span>
      </button>

      {#if onSwitchToSource}
        <button
          type="button"
          class="table-btn table-source-btn"
          onclick={onSwitchToSource}
          title="Ver o editar sintaxis Markdown pura"
        >
          <Code size={12} />
          <span>Markdown</span>
        </button>
      {/if}
    </div>
  </div>

  <!-- Contenedor scrollable de la tabla interactiva -->
  <div class="table-scroll-wrapper">
    <table class="interactive-table">
      <thead>
        <tr>
          <!-- Esquina superior izquierda / gutter de número -->
          <th class="table-corner-cell">#</th>

          {#each headers as header, colIndex}
            <th class="table-header-cell">
              <div class="header-content-wrapper">
                <input
                  type="text"
                  class="table-input header-input"
                  style="text-align: {alignments[colIndex] === 'center' ? 'center' : alignments[colIndex] === 'right' ? 'right' : 'left'};"
                  value={header}
                  data-header={colIndex}
                  placeholder={`Columna ${colIndex + 1}`}
                  oninput={(e) => updateHeader(colIndex, (e.target as HTMLInputElement).value)}
                  onkeydown={(e) => handleHeaderKeyDown(e, colIndex)}
                />

                <div class="header-actions">
                  <!-- Botón de alineación de columna -->
                  <button
                    type="button"
                    class="column-tool-btn"
                    title={`Alineación: ${alignments[colIndex] || 'por defecto'} (Clic para cambiar)`}
                    onclick={() => cycleAlignment(colIndex)}
                  >
                    {#if alignments[colIndex] === 'center'}
                      <AlignCenter size={11} />
                    {:else if alignments[colIndex] === 'right'}
                      <AlignRight size={11} />
                    {:else}
                      <AlignLeft size={11} />
                    {/if}
                  </button>

                  <!-- Botón añadir columna después -->
                  <button
                    type="button"
                    class="column-tool-btn"
                    title="Insertar columna a la derecha"
                    onclick={() => addColumn(colIndex)}
                  >
                    <Plus size={11} />
                  </button>

                  <!-- Botón eliminar columna -->
                  {#if headers.length > 1}
                    <button
                      type="button"
                      class="column-tool-btn delete-btn"
                      title="Eliminar columna"
                      onclick={() => deleteColumn(colIndex)}
                    >
                      <Trash2 size={11} />
                    </button>
                  {/if}
                </div>
              </div>
            </th>
          {/each}
        </tr>
      </thead>

      <tbody>
        {#each rows as row, rowIndex}
          <tr class="table-data-row">
            <!-- Gutter de fila: número y acciones de fila -->
            <td class="table-row-gutter">
              <span class="row-number">{rowIndex + 1}</span>
              <div class="row-actions">
                <button
                  type="button"
                  class="row-tool-btn"
                  title="Insertar fila abajo"
                  onclick={() => addRow(rowIndex)}
                >
                  <Plus size={10} />
                </button>
                <button
                  type="button"
                  class="row-tool-btn delete-btn"
                  title="Eliminar fila"
                  onclick={() => deleteRow(rowIndex)}
                >
                  <Trash2 size={10} />
                </button>
              </div>
            </td>

            {#each headers as _, colIndex}
              <td class="table-data-cell">
                <input
                  type="text"
                  class="table-input cell-input"
                  style="text-align: {alignments[colIndex] === 'center' ? 'center' : alignments[colIndex] === 'right' ? 'right' : 'left'};"
                  value={row[colIndex] ?? ''}
                  data-row={rowIndex}
                  data-col={colIndex}
                  placeholder=""
                  oninput={(e) => updateCell(rowIndex, colIndex, (e.target as HTMLInputElement).value)}
                  onkeydown={(e) => handleCellKeyDown(e, rowIndex, colIndex)}
                />
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .table-card {
    display: block;
    margin: 14px 0 18px 0;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    overflow: hidden;
    background-color: var(--bg-primary, #ffffff);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04);
    transition: border-color 0.2s ease, box-shadow 0.2s ease;
  }

  .table-card:hover {
    border-color: rgba(9, 105, 218, 0.35);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.06);
  }

  .table-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 12px;
    background-color: var(--bg-secondary, #f6f8fa);
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    gap: 8px;
    user-select: none;
  }

  .table-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
  }

  .table-badge :global(svg) {
    color: var(--accent, #0969da);
  }

  .table-stats-pill {
    font-size: 10.5px;
    font-weight: 500;
    padding: 1px 7px;
    border-radius: 999px;
    background-color: rgba(9, 105, 218, 0.08);
    color: var(--accent, #0969da);
  }

  .table-actions {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .table-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    font-weight: 500;
    padding: 3px 8px;
    border-radius: 5px;
    border: 1px solid var(--border-primary, #d0d7de);
    background-color: var(--bg-primary, #ffffff);
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .table-btn:hover {
    color: var(--accent, #0969da);
    border-color: var(--accent, #0969da);
    background-color: rgba(9, 105, 218, 0.04);
  }

  .table-source-btn:hover {
    color: #cf222e;
    border-color: #cf222e;
    background-color: rgba(207, 34, 46, 0.04);
  }

  .table-scroll-wrapper {
    overflow-x: auto;
    max-width: 100%;
    background-color: var(--bg-primary, #ffffff);
  }

  .interactive-table {
    width: 100%;
    border-collapse: collapse;
    font-family: var(--main-font, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif);
    font-size: 13.5px;
  }

  .table-corner-cell {
    width: 44px;
    min-width: 44px;
    text-align: center;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary, #656d76);
    background-color: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    padding: 6px 4px;
    user-select: none;
  }

  .table-header-cell {
    background-color: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    padding: 2px 4px;
    font-weight: 600;
    position: relative;
    min-width: 120px;
  }

  .header-content-wrapper {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .table-header-cell:hover .header-actions,
  .table-header-cell:focus-within .header-actions {
    opacity: 1;
  }

  .column-tool-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: 1px solid transparent;
    border-radius: 3px;
    background: transparent;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    transition: all 0.12s ease;
  }

  .column-tool-btn:hover {
    color: var(--accent, #0969da);
    background-color: rgba(9, 105, 218, 0.1);
    border-color: rgba(9, 105, 218, 0.2);
  }

  .column-tool-btn.delete-btn:hover {
    color: #cf222e;
    background-color: rgba(207, 34, 46, 0.1);
    border-color: rgba(207, 34, 46, 0.2);
  }

  .table-row-gutter {
    width: 44px;
    min-width: 44px;
    text-align: center;
    background-color: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    position: relative;
    user-select: none;
    padding: 2px;
  }

  .row-number {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary, #656d76);
    display: block;
    line-height: 28px;
  }

  .row-actions {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 2px;
    background-color: var(--bg-secondary, #f6f8fa);
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .table-row-gutter:hover .row-actions {
    opacity: 1;
  }

  .row-tool-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    padding: 0;
    border: 1px solid transparent;
    border-radius: 3px;
    background: transparent;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
  }

  .row-tool-btn:hover {
    color: var(--accent, #0969da);
    background-color: rgba(9, 105, 218, 0.12);
  }

  .row-tool-btn.delete-btn:hover {
    color: #cf222e;
    background-color: rgba(207, 34, 46, 0.12);
  }

  .table-data-row:nth-child(2n) {
    background-color: rgba(246, 248, 250, 0.5);
  }

  .table-data-row:hover {
    background-color: rgba(9, 105, 218, 0.02);
  }

  .table-data-cell {
    border: 1px solid var(--border-primary, #d0d7de);
    padding: 2px 4px;
    min-width: 120px;
  }

  .table-input {
    width: 100%;
    border: 1px solid transparent;
    border-radius: 4px;
    background: transparent;
    outline: none;
    padding: 5px 8px;
    font-size: 13.5px;
    font-family: inherit;
    color: var(--text-primary, #1f2328);
    transition: background-color 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
    box-sizing: border-box;
  }

  .table-input:hover {
    background-color: rgba(0, 0, 0, 0.02);
  }

  .table-input:focus {
    background-color: var(--bg-primary, #ffffff);
    border-color: var(--accent, #0969da);
    box-shadow: 0 0 0 2px rgba(9, 105, 218, 0.15);
  }

  .header-input {
    font-weight: 600;
  }
</style>
