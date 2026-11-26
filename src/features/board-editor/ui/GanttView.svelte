<script lang="ts">
  import type { BoardTask } from '../model/board.svelte';

  interface Props {
    tasks: BoardTask[];
    onUpdateTask: (id: string, patch: Partial<BoardTask>) => void;
    onSelectComments: (id: string) => void;
  }

  let { tasks, onUpdateTask, onSelectComments }: Props = $props();

  const DAY_WIDTH = 36;
  const ROW_HEIGHT = 36;

  function parseDate(iso: string): Date {
    const [y, m, d] = iso.split('-').map(Number);
    return new Date(y, (m || 1) - 1, d || 1);
  }

  function formatDate(d: Date): string {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }

  function addDays(iso: string, delta: number): string {
    const d = parseDate(iso);
    d.setDate(d.getDate() + delta);
    return formatDate(d);
  }

  function diffDays(a: string, b: string): number {
    const ms = parseDate(b).getTime() - parseDate(a).getTime();
    return Math.round(ms / 86400000);
  }

  let minDate = $derived.by(() => {
    if (tasks.length === 0) return formatDate(new Date());
    return tasks.reduce((min, t) => (t.start < min ? t.start : min), tasks[0].start);
  });

  let totalDays = $derived.by(() => {
    if (tasks.length === 0) return 14;
    let maxEnd = 14;
    for (const t of tasks) {
      const endOffset = diffDays(minDate, t.start) + t.duration + 3;
      if (endOffset > maxEnd) maxEnd = endOffset;
    }
    return maxEnd;
  });

  let days = $derived(Array.from({ length: totalDays }, (_, i) => addDays(minDate, i)));

  let dragging = $state<{ id: string; startX: number; origStart: string } | null>(null);

  function onBarPointerDown(e: PointerEvent, task: BoardTask) {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    dragging = { id: task.id, startX: e.clientX, origStart: task.start };
  }

  function onBarPointerMove(e: PointerEvent) {
    if (!dragging) return;
    const deltaDays = Math.round((e.clientX - dragging.startX) / DAY_WIDTH);
    const nextStart = addDays(dragging.origStart, deltaDays);
    onUpdateTask(dragging.id, { start: nextStart });
  }

  function onBarPointerUp() {
    dragging = null;
  }
</script>

<div class="gantt-view">
  <div class="gantt-sidebar">
    <div class="gantt-header-cell">Tarea</div>
    {#each tasks as task (task.id)}
      <button type="button" class="gantt-row-label" onclick={() => onSelectComments(task.id)}>
        <span class="task-id">{task.id}</span>
        <span class="task-title">{task.title}</span>
      </button>
    {/each}
  </div>

  <div class="gantt-timeline" style="min-width: {totalDays * DAY_WIDTH}px">
    <div class="timeline-header">
      {#each days as day (day)}
        <div class="day-cell" style="width: {DAY_WIDTH}px">{day.slice(5)}</div>
      {/each}
    </div>

    <div class="timeline-body" style="height: {Math.max(tasks.length, 1) * ROW_HEIGHT}px">
      <svg class="dep-layer" width={totalDays * DAY_WIDTH} height={tasks.length * ROW_HEIGHT}>
        {#each tasks as task, idx (task.id)}
          {#if task.dependsOn && task.dependsOn !== '-'}
            {@const depIdx = tasks.findIndex((t) => t.id === task.dependsOn)}
            {#if depIdx !== -1}
              {@const depTask = tasks[depIdx]}
              {@const x1 = (diffDays(minDate, depTask.start) + depTask.duration) * DAY_WIDTH}
              {@const y1 = depIdx * ROW_HEIGHT + ROW_HEIGHT / 2}
              {@const x2 = diffDays(minDate, task.start) * DAY_WIDTH}
              {@const y2 = idx * ROW_HEIGHT + ROW_HEIGHT / 2}
              <path
                d="M {x1} {y1} C {x1 + 16} {y1}, {x2 - 16} {y2}, {x2} {y2}"
                fill="none"
                stroke="var(--text-muted)"
                stroke-width="1.5"
              />
            {/if}
          {/if}
        {/each}
      </svg>

      {#each tasks as task, idx (task.id)}
        {@const left = diffDays(minDate, task.start) * DAY_WIDTH}
        {@const width = Math.max(DAY_WIDTH, task.duration * DAY_WIDTH)}
        <div
          class="gantt-bar"
          role="slider"
          tabindex="0"
          aria-label={task.title}
          aria-valuenow={task.progress}
          style="left: {left}px; top: {idx * ROW_HEIGHT + 6}px; width: {width}px;"
          onpointerdown={(e) => onBarPointerDown(e, task)}
          onpointermove={onBarPointerMove}
          onpointerup={onBarPointerUp}
        >
          <div class="gantt-bar-progress" style="width: {task.progress}%"></div>
          <span class="gantt-bar-label">{task.title}</span>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .gantt-view {
    display: flex;
    height: 100%;
    overflow: auto;
    font-size: 12px;
  }

  .gantt-sidebar {
    min-width: 200px;
    max-width: 220px;
    border-right: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    position: sticky;
    left: 0;
    z-index: 3;
  }

  .gantt-header-cell {
    height: 32px;
    display: flex;
    align-items: center;
    padding: 0 10px;
    font-weight: 600;
    border-bottom: 1px solid var(--border-primary);
    color: var(--text-secondary);
  }

  .gantt-row-label {
    height: 36px;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    border: none;
    border-bottom: 1px solid var(--border-secondary);
    background: transparent;
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
  }

  .gantt-row-label:hover {
    background: var(--bg-tertiary);
  }

  .task-id {
    color: var(--text-muted);
    font-size: 11px;
  }

  .task-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .gantt-timeline {
    flex: 1;
    position: relative;
  }

  .timeline-header {
    display: flex;
    height: 32px;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
  }

  .day-cell {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-right: 1px solid var(--border-secondary);
    color: var(--text-muted);
    font-size: 11px;
  }

  .timeline-body {
    position: relative;
  }

  .dep-layer {
    position: absolute;
    top: 0;
    left: 0;
    pointer-events: none;
  }

  .gantt-bar {
    position: absolute;
    height: 24px;
    background: var(--bg-tertiary);
    border: 1px solid var(--accent);
    border-radius: 5px;
    cursor: grab;
    overflow: hidden;
    display: flex;
    align-items: center;
    padding: 0 6px;
    user-select: none;
  }

  .gantt-bar-progress {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    background: var(--accent);
    opacity: 0.25;
    pointer-events: none;
  }

  .gantt-bar-label {
    position: relative;
    z-index: 1;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 11px;
  }
</style>
