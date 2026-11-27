<script lang="ts">
  import { tick } from 'svelte';
  import { MessageSquare, Plus } from 'lucide-svelte';
  import { t } from '@shared/lib/i18n';
  import { clampColumnWidth, COLUMN_WIDTH, type BoardColumn, type BoardTask } from '../model/board.svelte';

  interface Props {
    tasks: BoardTask[];
    columns: BoardColumn[];
    /** Anchos propios, en píxeles, por columna. */
    widths: Record<string, number>;
    onResizeColumn: (id: string, width: number | null) => void;
    /** Cambia de columna y de posición: delante de `before`, o al final con `null`. */
    onMoveTask: (id: string, status: string, before: string | null) => void;
    onOpenTask: (id: string) => void;
    onAddTask: (status?: string) => void;
    onSelectComments: (id: string) => void;
  }

  let { tasks, columns, widths, onResizeColumn, onMoveTask, onOpenTask, onAddTask, onSelectComments }: Props =
    $props();

  // Arrastre con eventos de puntero: el de HTML5 no llega a soltar en el webview (Tauri atiende el
  // soltado nativo, para los archivos)
  /** Píxeles que hay que moverse para que sea un arrastre y no un clic. */
  const DRAG_THRESHOLD = 5;
  let root = $state<HTMLDivElement>();
  let drag = $state<{
    id: string;
    title: string;
    /** Dónde se pulsó, y a qué distancia de la esquina de la tarjeta. */
    startX: number;
    startY: number;
    offsetX: number;
    offsetY: number;
    width: number;
    x: number;
    y: number;
    moving: boolean;
    /** Dónde caería: la columna y la tarjeta que quedaría debajo (`null`, al final). */
    over: { column: string; before: string | null } | null;
  } | null>(null);

  /** Columna que se está ensanchando: el archivo se escribe una sola vez, al soltar. */
  let resize = $state<{ id: string; startX: number; startWidth: number; width: number; moved: boolean } | null>(null);

  const columnWidth = (handle: EventTarget | null) => (handle as HTMLElement).parentElement!.offsetWidth;

  function startResize(e: PointerEvent, id: string) {
    if (e.button !== 0) return;
    e.preventDefault();
    const handle = e.currentTarget as HTMLElement;
    handle.focus({ preventScroll: true });
    handle.setPointerCapture(e.pointerId);
    const width = columnWidth(handle);
    resize = { id, startX: e.clientX, startWidth: width, width, moved: false };
  }

  function moveResize(e: PointerEvent) {
    if (!resize) return;
    resize.moved ||= e.clientX !== resize.startX;
    resize.width = clampColumnWidth(resize.startWidth + e.clientX - resize.startX);
  }

  function endResize() {
    if (resize?.moved) onResizeColumn(resize.id, resize.width);
    resize = null;
  }

  /** ← / → cambian el ancho; Enter vuelve al normal. */
  function onResizerKey(e: KeyboardEvent, id: string) {
    if (e.key === 'Enter') onResizeColumn(id, null);
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
      const step = e.key === 'ArrowRight' ? 20 : -20;
      onResizeColumn(id, clampColumnWidth(columnWidth(e.currentTarget) + step));
    } else return;
    e.preventDefault();
  }

  function columnAt(x: number, y: number): HTMLElement | null {
    const col = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-column]');
    // Con la vista dividida puede haber otro tablero al lado
    return col && root?.contains(col) ? col : null;
  }

  /** La primera tarjeta, sin contar la arrastrada, cuyo centro queda por debajo del puntero. */
  function cardBelow(col: HTMLElement, y: number, dragged: string): string | null {
    const card = [...col.querySelectorAll<HTMLElement>('[data-task]')].find((card) => {
      const box = card.getBoundingClientRect();
      return card.dataset.task !== dragged && y < box.top + box.height / 2;
    });
    return card?.dataset.task ?? null;
  }

  function startDrag(e: PointerEvent, task: BoardTask) {
    if (e.button !== 0 || (e.target as HTMLElement).closest('button')) return;
    const card = e.currentTarget as HTMLElement;
    const box = card.getBoundingClientRect();
    // Sin esto, al arrastrar se selecciona el texto por donde pasa el puntero
    e.preventDefault();
    card.focus({ preventScroll: true });
    card.setPointerCapture(e.pointerId);
    drag = {
      id: task.id,
      title: task.title,
      startX: e.clientX,
      startY: e.clientY,
      offsetX: e.clientX - box.left,
      offsetY: e.clientY - box.top,
      width: box.width,
      x: e.clientX,
      y: e.clientY,
      moving: false,
      over: null,
    };
  }

  function moveDrag(e: PointerEvent) {
    if (!drag) return;
    drag.x = e.clientX;
    drag.y = e.clientY;
    drag.moving ||= Math.hypot(e.clientX - drag.startX, e.clientY - drag.startY) > DRAG_THRESHOLD;
    if (!drag.moving || !root) return;
    const col = columnAt(e.clientX, e.clientY);
    // Cerca de un borde, se desplaza para llegar a las tarjetas y columnas que no se ven
    const list = col?.querySelector<HTMLElement>('.col-cards');
    if (list) {
      const { top, bottom } = list.getBoundingClientRect();
      if (e.clientY > bottom - 30) list.scrollTop += 12;
      else if (e.clientY < top + 30) list.scrollTop -= 12;
    }
    drag.over = col && { column: col.dataset.column!, before: cardBelow(col, e.clientY, drag.id) };
    const box = root.getBoundingClientRect();
    if (e.clientX > box.right - 40) root.scrollLeft += 16;
    else if (e.clientX < box.left + 40) root.scrollLeft -= 16;
  }

  function endDrag() {
    if (drag?.moving && drag.over) onMoveTask(drag.id, drag.over.column, drag.over.before);
    drag = null;
  }

  /** Enter abre el detalle; Shift+flechas mueven la tarjeta enfocada. */
  async function onCardKey(e: KeyboardEvent, task: BoardTask) {
    if (e.ctrlKey || e.altKey || e.metaKey || e.target !== e.currentTarget) return;
    if (e.key === 'Enter') {
      e.preventDefault();
      return onOpenTask(task.id);
    }
    if (!e.shiftKey || !e.key.startsWith('Arrow')) return;
    e.preventDefault();
    if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
      const step = e.key === 'ArrowRight' ? 1 : -1;
      const next = columns[columns.findIndex((c) => c.id === task.status) + step];
      if (!next) return;
      onMoveTask(task.id, next.id, null);
    } else {
      const column = tasks.filter((t) => t.status === task.status);
      const i = column.findIndex((t) => t.id === task.id);
      const up = e.key === 'ArrowUp';
      if (up ? i === 0 : i === column.length - 1) return;
      onMoveTask(task.id, task.status, (up ? column[i - 1] : column[i + 2])?.id ?? null);
    }
    // Al moverse, la tarjeta pierde el foco: lo recupera
    await tick();
    root?.querySelector<HTMLElement>(`[data-task="${CSS.escape(task.id)}"]`)?.focus();
  }

  function openOnDoubleClick(e: MouseEvent, id: string) {
    if (!(e.target as HTMLElement).closest('button')) onOpenTask(id);
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (drag = resize = null)} />

<div class="kanban-view" bind:this={root}>
  {#each columns as col (col.id)}
    {@const colTasks = tasks.filter((t) => t.status === col.id)}
    {@const dropHere = drag?.moving && drag.over?.column === col.id ? drag.over : null}
    {@const width = resize?.id === col.id ? resize.width : widths[col.id]}
    <div
      class="kanban-col"
      class:drag-over={dropHere}
      class:resized={width !== undefined}
      style:--col-width={width === undefined ? null : `${width}px`}
      data-column={col.id}
      role="list"
    >
      <div class="col-header">
        <span class="dot" style="background: {col.color}"></span>
        <span class="col-title">{col.label}</span>
        <span class="col-count">{colTasks.length}</span>
      </div>

      <div class="col-cards" class:drop-end={dropHere && dropHere.before === null}>
        {#each colTasks as task (task.id)}
          <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
          <div
            class="card"
            class:dragging={drag?.moving && drag.id === task.id}
            class:drop-before={dropHere?.before === task.id}
            role="listitem"
            tabindex="0"
            data-task={task.id}
            onpointerdown={(e) => startDrag(e, task)}
            onpointermove={moveDrag}
            onpointerup={endDrag}
            onpointercancel={() => (drag = null)}
            onkeydown={(e) => onCardKey(e, task)}
            ondblclick={(e) => openOnDoubleClick(e, task.id)}
          >
            <div class="card-top">
              <span class="card-id">{task.id}</span>
              {#if task.dependsOn !== '-'}
                <span class="dep-badge">↳ {task.dependsOn}</span>
              {/if}
            </div>
            <div class="card-title">{task.title}</div>
            <div class="progress-bar">
              <div class="progress-fill" style="width: {task.progress}%"></div>
            </div>
            <div class="card-footer">
              <span class="dates">{task.start} · {task.duration}d</span>
              <button
                type="button"
                class="comment-btn"
                onclick={() => onSelectComments(task.id)}
              >
                <MessageSquare size={12} />
                <span>{task.comments.length}</span>
              </button>
            </div>
          </div>
        {/each}
      </div>

      <button type="button" class="add-card-btn" onclick={() => onAddTask(col.id)}>
        <Plus size={13} /> Añadir
      </button>
      <!-- Un separador que se puede enfocar es un control (WAI-ARIA «window splitter») -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
      <div
        class="col-resizer"
        class:active={resize?.id === col.id}
        role="separator"
        aria-orientation="vertical"
        aria-label={t('board.resizeColumn', { column: col.label })}
        aria-valuemin={COLUMN_WIDTH.min}
        aria-valuemax={COLUMN_WIDTH.max}
        aria-valuenow={width}
        title={t('board.resizeColumnHint')}
        tabindex="0"
        onpointerdown={(e) => startResize(e, col.id)}
        onpointermove={moveResize}
        onpointerup={endResize}
        onpointercancel={() => (resize = null)}
        ondblclick={() => onResizeColumn(col.id, null)}
        onkeydown={(e) => onResizerKey(e, col.id)}
      ></div>
    </div>
  {/each}
</div>

{#if drag?.moving}
  <div
    class="kanban-ghost"
    style="left: {drag.x - drag.offsetX}px; top: {drag.y - drag.offsetY}px; width: {drag.width}px"
  >
    {drag.title}
  </div>
{/if}

<style>
  .kanban-view {
    display: flex;
    gap: 14px;
    padding: 16px;
    height: 100%;
    overflow-x: auto;
    align-items: flex-start;
  }

  .kanban-col {
    position: relative;
    flex-shrink: 0;
    min-width: 260px;
    max-width: 280px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 8px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 100%;
  }

  .kanban-col.resized {
    width: var(--col-width);
    min-width: var(--col-width);
    max-width: var(--col-width);
  }

  /* En el hueco que sigue a la columna */
  .col-resizer {
    position: absolute;
    top: 0;
    bottom: 0;
    right: -9px;
    width: 8px;
    border-radius: 4px;
    cursor: col-resize;
    touch-action: none;
  }

  .col-resizer:hover,
  .col-resizer:focus-visible,
  .col-resizer.active {
    background: var(--accent);
    outline: none;
  }

  .kanban-col.drag-over {
    border-color: var(--accent);
    background: var(--bg-tertiary);
  }

  .card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .card.dragging {
    opacity: 0.4;
  }

  /* No estorba al buscar la columna que hay bajo el puntero */
  .kanban-ghost {
    position: fixed;
    z-index: 1000;
    pointer-events: none;
    padding: 10px;
    border: 1px solid var(--accent);
    border-radius: 6px;
    background: var(--bg-primary);
    box-shadow: var(--modal-shadow);
    font-size: 13px;
    color: var(--text-primary);
    transform: rotate(1.5deg);
    /* Deja ver debajo la marca de dónde caerá */
    opacity: 0.75;
  }

  .col-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 4px;
    font-size: 13px;
    font-weight: 600;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .col-title {
    flex: 1;
    color: var(--text-primary);
  }

  .col-count {
    font-size: 11px;
    color: var(--text-muted);
    background: var(--bg-tertiary);
    padding: 1px 6px;
    border-radius: 10px;
  }

  .col-cards {
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
    /* Sitio para la marca de destino encima de la primera tarjeta, sin mover nada */
    padding-block: 6px;
    margin-block: -6px;
  }

  .card.drop-before::before {
    content: '';
    position: absolute;
    top: -6px;
    left: 0;
    right: 0;
    height: 2px;
    border-radius: 1px;
    background: var(--accent);
  }

  .col-cards.drop-end::after {
    content: '';
    flex-shrink: 0;
    height: 2px;
    border-radius: 1px;
    background: var(--accent);
  }

  .card {
    position: relative;
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    padding: 10px;
    cursor: grab;
    user-select: none;
    touch-action: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .card:hover {
    border-color: var(--accent);
  }

  .card-top {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--text-muted);
  }

  .dep-badge {
    background: var(--bg-tertiary);
    padding: 0 5px;
    border-radius: 4px;
  }

  .card-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .progress-bar {
    height: 4px;
    background: var(--bg-tertiary);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent);
  }

  .card-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 11px;
    color: var(--text-muted);
  }

  .comment-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 4px;
  }

  .comment-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .add-card-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 6px;
    font-size: 12px;
    background: transparent;
    color: var(--text-secondary);
    border: 1px dashed var(--border-primary);
    border-radius: 6px;
    cursor: pointer;
  }

  .add-card-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }
</style>
