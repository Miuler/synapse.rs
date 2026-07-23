<script lang="ts">
  import Ribbon from './lib/components/Ribbon.svelte';
  import EditorHeader from './lib/components/EditorHeader.svelte';
  import StatusBar from './lib/components/StatusBar.svelte';
  import CommandPalette from './lib/components/CommandPalette.svelte';
  import { commandRegistry } from './lib/services/commands.svelte';

  // Estados reactivos con Runas de Svelte 5
  let activeRibbonTab = $state('files');
  let isPaletteOpen = $state(false);
  let isEditing = $state(true);

  // Datos dummy de archivos de notas
  const notes = $state([
    { id: '1', title: 'Bienvenida a Synapse.rs', content: '# Synapse.rs\n\nBienvenido a tu entorno de notas hiper-rápido impulsado por Rust y Svelte 5.\n\nPresiona **Ctrl+P** para abrir la paleta de comandos flotante.' },
    { id: '2', title: 'Arquitectura Cebolla en Rust.md', content: '# Arquitectura Cebolla\n\nEstructura en capas:\n- **Domain**: Entidades y traits de repositorios.\n- **Application**: Casos de uso.\n- **Infrastructure**: Implementación física y comandos Tauri.' },
    { id: '3', title: 'Ideas y Proyectos 2026.md', content: '# Proyectos 2026\n\n1. Motor de búsqueda difusa con Skim/Nucleo en Rust.\n2. Visualización de grafo en WebGL.' }
  ]);

  let activeNoteIndex = $state(0);
  let currentNote = $derived(notes[activeNoteIndex]);

  // Contadores calculados reactivamente
  let wordCount = $derived(
    currentNote.content.trim() ? currentNote.content.trim().split(/\s+/).length : 0
  );
  let charCount = $derived(currentNote.content.length);

  // Registrar comandos por defecto al iniciar
  $effect(() => {
    commandRegistry.registerMany([
      {
        id: 'cmd-new-note',
        name: 'Crear nueva nota',
        category: 'Archivo',
        shortcut: 'Ctrl+N',
        action: () => {
          notes.push({
            id: String(notes.length + 1),
            title: `Nueva Nota ${notes.length + 1}.md`,
            content: '# Nueva Nota\n\nEscribe tu contenido aquí...'
          });
          activeNoteIndex = notes.length - 1;
        }
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
      },
      {
        id: 'cmd-graph-view',
        name: 'Abrir vista de gráfico de conexiones',
        category: 'Visualización',
        action: () => {
          alert('Próximamente: Vista de gráfico interactiva impulsada por Rust');
        }
      },
      {
        id: 'cmd-vault-settings',
        name: 'Abrir preferencias y configuración',
        category: 'Ajustes',
        shortcut: 'Ctrl+,',
        action: () => {
          alert('Configuración de la bóveda de notas');
        }
      }
    ]);
  });

  function handleRibbonAction(actionId: string) {
    if (actionId === 'command-palette') {
      isPaletteOpen = true;
    } else if (actionId === 'new-note') {
      const newCmd = commandRegistry.all.find((c) => c.id === 'cmd-new-note');
      if (newCmd) newCmd.action();
    }
  }
</script>

<div class="workspace-layout">
  <!-- 1. BARRA RIBBON IZQUIERDA -->
  <Ribbon bind:activeTab={activeRibbonTab} onAction={handleRibbonAction} />

  <!-- 2. PANEL LATERAL (EXPLORADOR DUMMY) -->
  {#if activeRibbonTab === 'files' || activeRibbonTab === 'search'}
    <aside class="sidebar-panel">
      <div class="sidebar-header">
        <span>{activeRibbonTab === 'files' ? 'Bóveda de Notas' : 'Buscar'}</span>
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
            <span>{note.title}</span>
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
      title={currentNote.title}
      onOpenCommandPalette={() => (isPaletteOpen = true)}
    />

    <!-- CONTENEDOR DEL EDITOR -->
    <div class="editor-container">
      <input
        type="text"
        class="editor-title-input"
        bind:value={currentNote.title}
        placeholder="Título de la nota..."
      />

      {#if isEditing}
        <textarea
          class="editor-textarea"
          bind:value={currentNote.content}
          placeholder="Comienza a escribir Markdown..."
        ></textarea>
      {:else}
        <div class="markdown-preview">
          <p>{currentNote.content}</p>
        </div>
      {/if}
    </div>

    <!-- BARRA DE ESTADO INFERIOR -->
    <StatusBar
      {wordCount}
      {charCount}
      line={1}
      col={currentNote.content.length}
      syncStatus="synced"
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
</style>
