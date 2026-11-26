<script lang="ts">
  import { X, Send } from 'lucide-svelte';
  import type { BoardTask } from '../model/board.svelte';

  interface Props {
    task: BoardTask;
    onClose: () => void;
    onAddComment: (taskId: string, text: string) => void;
  }

  let { task, onClose, onAddComment }: Props = $props();
  let draft = $state('');

  function submit(e: Event) {
    e.preventDefault();
    const text = draft.trim();
    if (!text) return;
    onAddComment(task.id, text);
    draft = '';
  }
</script>

<aside class="comments-panel">
  <div class="panel-header">
    <div>
      <div class="panel-id">{task.id}</div>
      <div class="panel-title">{task.title}</div>
    </div>
    <button type="button" class="close-btn" onclick={onClose} aria-label="Cerrar">
      <X size={16} />
    </button>
  </div>

  <div class="comments-list">
    {#if task.comments.length === 0}
      <p class="empty">Sin comentarios aún.</p>
    {:else}
      {#each task.comments as comment, i (i)}
        <div class="comment-item">
          <div class="comment-ts">{comment.timestamp.replace('T', ' ').slice(0, 19)}</div>
          <div class="comment-text">{comment.text}</div>
        </div>
      {/each}
    {/if}
  </div>

  <form class="comment-form" onsubmit={submit}>
    <input
      type="text"
      placeholder="Escribe un comentario..."
      bind:value={draft}
    />
    <button type="submit" aria-label="Enviar">
      <Send size={14} />
    </button>
  </form>
</aside>

<style>
  .comments-panel {
    width: 300px;
    border-left: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .panel-header {
    padding: 12px;
    border-bottom: 1px solid var(--border-primary);
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 8px;
  }

  .panel-id {
    font-size: 11px;
    color: var(--text-muted);
  }

  .panel-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 2px;
  }

  .comments-list {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .empty {
    font-size: 12px;
    color: var(--text-muted);
    margin: 0;
  }

  .comment-item {
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    padding: 8px;
  }

  .comment-ts {
    font-size: 10px;
    color: var(--text-muted);
    margin-bottom: 4px;
  }

  .comment-text {
    font-size: 12px;
    color: var(--text-primary);
    white-space: pre-wrap;
  }

  .comment-form {
    padding: 10px;
    border-top: 1px solid var(--border-primary);
    display: flex;
    gap: 6px;
  }

  .comment-form input {
    flex: 1;
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    padding: 6px 8px;
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
  }

  .comment-form input:focus {
    border-color: var(--accent);
  }

  .comment-form button {
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 6px;
    padding: 0 10px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
  }
</style>
