<script lang="ts">
  import { tick } from 'svelte';
  import { MessageSquare, Plus } from 'lucide-svelte';
  import type { BoardColumn, BoardTask } from '../model/board.svelte';

  interface Props {
    tasks: BoardTask[];
    columns: BoardColumn[];
    onUpdateTask: (id: string, patch: Partial<BoardTask>) => void;
    onAddTask: (status?: string) => void;
    onSelectComments: (id: string) => void;
  }

  let { tasks, columns, onUpdateTask, onAddTask, onSelectComments }: Props = $props();

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
    over: string | null;
  } | null>(null);

  function columnAt(x: number, y: number): string | null {
    const col = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-column]');
    // Con la vista dividida puede haber otro tablero al lado
    return col && root?.contains(col) ? col.dataset.column! : null;
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
    drag.over = columnAt(e.clientX, e.clientY);
    // Cerca de un borde, se desplaza para llegar a las columnas que no se ven
    const box = root.getBoundingClientRect();
    if (e.clientX > box.right - 40) root.scrollLeft += 16;
    else if (e.clientX < box.left + 40) root.scrollLeft -= 16;
  }

  function endDrag() {
    if (drag?.moving && drag.over) moveTask(drag.id, drag.over);
    drag = null;
  }

  function moveTask(id: string, column: string) {
    if (tasks.find((t) => t.id === id)?.status !== column) onUpdateTask(id, { status: column });
  }

  /** Shift+← / Shift+→ sobre la tarjeta enfocada. */
  async function moveByKey(e: KeyboardEvent, task: BoardTask) {
    const step = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
    if (!step || !e.shiftKey || e.ctrlKey || e.altKey || e.metaKey || e.target !== e.currentTarget) return;
    e.preventDefault();
    const next = columns[columns.findIndex((c) => c.id === task.status) + step];
    if (!next) return;
    moveTask(task.id, next.id);
    // La tarjeta se vuelve a crear en la otra columna: recupera el foco
    await tick();
    root?.querySelector<HTMLElement>(`[data-task="${CSS.escape(task.id)}"]`)?.focus();
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (drag = null)} />

<div class="kanban-view" bind:this={root}>
  {#each columns as col (col.id)}
    {@const colTasks = tasks.filter((t) => t.status === col.id)}
    <div class="kanban-col" class:drag-over={drag?.over === col.id} data-column={col.id} role="list">
      <div class="col-header">
        <span class="dot" style="background: {col.color}"></span>
        <span class="col-title">{col.label}</span>
        <span class="col-count">{colTasks.length}</span>
      </div>

      <div class="col-cards">
        {#each colTasks as task (task.id)}
          <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
          <div
            class="card"
            class:dragging={drag?.moving && drag.id === task.id}
            role="listitem"
            tabindex="0"
            data-task={task.id}
            onpointerdown={(e) => startDrag(e, task)}
            onpointermove={moveDrag}
            onpointerup={endDrag}
            onpointercancel={() => (drag = null)}
            onkeydown={(e) => moveByKey(e, task)}
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
  }

  .card {
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
