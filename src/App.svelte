<script lang="ts">
  import Ribbon from './lib/components/Ribbon.svelte';
  import EditorHeader from './lib/components/EditorHeader.svelte';
  import StatusBar from './lib/components/StatusBar.svelte';
  import CommandPalette from './lib/components/CommandPalette.svelte';
  import MarkdownViewer from './lib/components/MarkdownViewer.svelte';
  import { commandRegistry } from './lib/services/commands.svelte';
  import { invokeTauri, isTauriEnvironment } from './lib/services/tauri';
  import { info, error, warn } from '@tauri-apps/plugin-log';

  interface NoteItem {
    id: string;
    title: string;
    content: string;
    relative_path: string;
  }

  // Estados reactivos con Runas de Svelte 5 (inicialmente VACÍO)
  let activeRibbonTab = $state('files');
  let isPaletteOpen = $state(false);
  let isEditing = $state(true);
  let isConnectedToRust = $state(false);
  let syncState = $state<'synced' | 'saving' | 'error'>('synced');

  // Estado reactivo dinámico sin ninguna nota predeterminada ni de relleno
  let notes = $state<NoteItem[]>([]);
  let activeNoteIndex = $state(0);

  let currentNote = $derived(
    notes[activeNoteIndex] || { id: '0', title: '', content: '', relative_path: '' }
  );

  // Contadores calculados reactivamente
  let wordCount = $derived(
    currentNote.content && currentNote.content.trim()
      ? currentNote.content.trim().split(/\s+/).length
      : 0
  );
  let charCount = $derived(currentNote.content ? currentNote.content.length : 0);

  import { onMount } from 'svelte';

  // Carga estrictamente dinámica desde Rust (Tauri IPC) al montar
  onMount(() => {
    async function fetchNotesFromBackend() {
      if (isTauriEnvironment()) {
        try {
          const realNotes = await invokeTauri<Array<{ relative_path: { 0?: string } | string; title: string; content: string }>>('get_vault_notes');

          if (realNotes && Array.isArray(realNotes)) {
            isConnectedToRust = true;
            notes = realNotes.map((n, index) => {
              let relPath = `${n.title}.md`;
              if (typeof n.relative_path === 'string') {
                relPath = n.relative_path;
              } else if (n.relative_path && typeof n.relative_path === 'object' && n.relative_path[0]) {
                relPath = n.relative_path[0];
              }

              return {
                id: String(index + 1),
                title: n.title,
                content: n.content,
                relative_path: relPath
              };
            });
            activeNoteIndex = 0;
          } else {
            notes = [];
          }
        } catch (e) {
          console.warn('Error al cargar notas de Rust:', e);
          isConnectedToRust = false;
          notes = [];
        }
      } else {
        notes = [];
      }
    }

    fetchNotesFromBackend();
  });

  // Guardar nota en tiempo real en el backend de Rust al modificar contenido o título
  async function persistNoteToRust(note: NoteItem) {
    if (!isConnectedToRust || !note.title) return;
    syncState = 'saving';
    try {
      await invokeTauri('save_note_content', {
        relativePath: note.relative_path || `${note.title}.md`,
        title: note.title,
        content: note.content
      });
      syncState = 'synced';
    } catch (e) {
      console.error('Error al guardar la nota en Rust:', e);
      syncState = 'error';
    }
  }

  async function createNewNote() {
    const newTitle = `Nueva Nota ${notes.length + 1}`;
    const newRelPath = `${newTitle}.md`;
    const newNote: NoteItem = {
      id: String(notes.length + 1),
      title: newTitle,
      content: '# Nueva Nota\n\nEscribe tu contenido aquí...',
      relative_path: newRelPath
    };
    notes.push(newNote);
    activeNoteIndex = notes.length - 1;
    await persistNoteToRust(newNote);
  }

  // Registrar comandos por defecto al iniciar
  onMount(() => {
    commandRegistry.registerMany([
      {
        id: 'cmd-new-note',
        name: 'Crear nueva nota',
        category: 'Archivo',
        shortcut: 'Ctrl+N',
        action: createNewNote
      },
      {
        id: 'cmd-open-palette',
        name: 'Abrir paleta de comandos',
        category: 'Sistema',
        shortcut: 'Ctrl+P',
        action: () => {
          isPaletteOpen = true;
        }
      },
      {
        id: 'cmd-toggle-view',
        name: 'Alternar entre modo Edición y Lectura',
        category: 'Vista',
        shortcut: 'Ctrl+E',
        action: () => {
          isEditing = !isEditing;
        }
      }
    ]);
  });

  function handleRibbonAction(actionId: string) {
    if (actionId === 'command-palette') {
      isPaletteOpen = true;
    } else if (actionId === 'new-note') {
      createNewNote();
    }
  }
</script>

<div class="workspace-layout">
  <!-- 1. BARRA RIBBON IZQUIERDA -->
  <Ribbon bind:activeTab={activeRibbonTab} onAction={handleRibbonAction} />

  <!-- 2. PANEL LATERAL (EXPLORADOR DE ARCHIVOS DE LA BÓVEDA) -->
  {#if activeRibbonTab === 'files' || activeRibbonTab === 'search'}
    <aside class="sidebar-panel">
      <div class="sidebar-header">
        <span>{activeRibbonTab === 'files' ? 'Bóveda de Notas' : 'Buscar'}</span>
        {#if isConnectedToRust}
          <span class="rust-badge" title="Conectado al Backend en Rust (Bóveda real en disco)">RUST</span>
        {/if}
      </div>
      <div class="sidebar-content">
        {#each notes as note, idx}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="file-tree-item"
            class:active={idx === activeNoteIndex}
            onclick={() => (activeNoteIndex = idx)}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
              <polyline points="14 2 14 8 20 8"/>
            </svg>
            <span class="file-name">{note.relative_path || `${note.title}.md`}</span>
          </div>
        {/each}
      </div>
    </aside>
  {/if}

  <!-- 3. ÁREA DE TRABAJO PRINCIPAL -->
  <main class="main-workspace">
    <!-- BARRA SUPERIOR DE PESTAÑA Y HERRAMIENTAS -->
    <EditorHeader
      bind:isEditing
      title={notes.length > 0 ? (currentNote.relative_path || (currentNote.title ? `${currentNote.title}.md` : '')) : ''}
      onOpenCommandPalette={() => (isPaletteOpen = true)}
    />

    <!-- CONTENEDOR DEL EDITOR -->
    <div class="editor-container">
      {#if notes.length === 0}
        <div class="empty-workspace">
          <svg class="empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
            <polyline points="14 2 14 8 20 8"/>
            <line x1="9" y1="15" x2="15" y2="15"/>
          </svg>
          <h2>No hay archivos en la bóveda</h2>
          <p>Crea una nueva nota para comenzar a escribir.</p>
          <button class="create-btn" onclick={createNewNote}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="12" y1="5" x2="12" y2="19"/>
              <line x1="5" y1="12" x2="19" y2="12"/>
            </svg>
            <span>Crear nueva nota</span>
          </button>
        </div>
      {:else}
        <input
          type="text"
          class="editor-title-input"
          bind:value={currentNote.title}
          oninput={() => persistNoteToRust(currentNote)}
          placeholder="Título de la nota..."
        />

        {#if isEditing}
          <textarea
            class="editor-textarea"
            bind:value={currentNote.content}
            oninput={() => persistNoteToRust(currentNote)}
            placeholder="Comienza a escribir Markdown..."
          ></textarea>
        {:else}
          <div class="markdown-preview">
            <MarkdownViewer
              content={currentNote.content}
              isMarkdown={currentNote.relative_path ? (currentNote.relative_path.endsWith('.md') || currentNote.relative_path.endsWith('.markdown')) : true}
            />
          </div>
        {/if}
      {/if}
    </div>

    <!-- BARRA DE ESTADO INFERIOR -->
    <StatusBar
      wordCount={notes.length > 0 ? wordCount : 0}
      charCount={notes.length > 0 ? charCount : 0}
      line={notes.length > 0 ? 1 : 0}
      col={notes.length > 0 && currentNote.content ? currentNote.content.length : 0}
      syncStatus={syncState}
      onOpenCommandPalette={() => (isPaletteOpen = true)}
    />
  </main>

  <!-- 4. PALETA DE COMANDOS (OVERLAY CTRL+P) -->
  <CommandPalette bind:isOpen={isPaletteOpen} />
</div>

<style>
  .markdown-preview {
    color: #c9d1d9;
    font-size: 16px;
    line-height: 1.7;
    white-space: pre-wrap;
    background: rgba(255, 255, 255, 0.02);
    padding: 20px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .empty-workspace {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 12px;
    color: #8b949e;
    text-align: center;
    user-select: none;
  }

  .empty-icon {
    width: 56px;
    height: 56px;
    color: #484f58;
    margin-bottom: 8px;
  }

  .empty-workspace h2 {
    font-size: 20px;
    font-weight: 500;
    color: #c9d1d9;
    margin: 0;
  }

  .empty-workspace p {
    font-size: 14px;
    color: #6e7681;
    margin: 0;
  }

  .create-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
    padding: 8px 16px;
    background-color: rgba(51, 204, 255, 0.1);
    color: var(--cyan, #33ccff);
    border: 1px solid rgba(51, 204, 255, 0.3);
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .create-btn:hover {
    background-color: rgba(51, 204, 255, 0.2);
    border-color: var(--cyan, #33ccff);
    box-shadow: 0 0 12px rgba(51, 204, 255, 0.3);
  }

  .file-name {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sidebar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .rust-badge {
    font-size: 9px;
    font-weight: 700;
    background: rgba(51, 204, 255, 0.15);
    color: var(--cyan, #33ccff);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid rgba(51, 204, 255, 0.3);
  }
</style>
