<script lang="ts">
  import { onMount } from 'svelte';
  import ForceGraph from 'force-graph';
  import type { VaultGraphData, GraphNode } from '@entities/vault-item';
  import { vaultApi } from '@entities/vault-item';
  import { Globe, Crosshair, RefreshCw, Layers, Search, Ghost, Filter } from 'lucide-svelte';

  interface Props {
    activeNotePath?: string | null;
    onSelectNote?: (path: string) => void;
  }

  let { activeNotePath = null, onSelectNote }: Props = $props();

  let containerEl = $state<HTMLDivElement | null>(null);
  let graphInstance: any = null;

  // Estado del Grafo
  let isGlobalMode = $state(false);
  let depth = $state(1);
  let isLoading = $state(false);
  let showGhostNodes = $state(true);
  let searchQuery = $state('');
  let rawGraphData = $state<VaultGraphData>({ nodes: [], links: [] });

  // Nodos y enlaces filtrados
  let filteredNodes = $derived(
    rawGraphData.nodes.filter((node) => {
      if (!showGhostNodes && node.is_ghost) return false;
      return true;
    })
  );

  let filteredLinks = $derived.by(() => {
    const validNodeIds = new Set(filteredNodes.map((n) => n.id));
    return rawGraphData.links.filter(
      (l) => validNodeIds.has(typeof l.source === 'object' ? (l.source as any).id : l.source) &&
             validNodeIds.has(typeof l.target === 'object' ? (l.target as any).id : l.target)
    );
  });

  let totalNodes = $derived(filteredNodes.length);
  let totalLinks = $derived(filteredLinks.length);

  async function loadGraph() {
    isLoading = true;
    try {
      const focus = isGlobalMode ? null : activeNotePath;
      const data = await vaultApi.getVaultGraph(focus, isGlobalMode ? null : depth);
      rawGraphData = data;
      syncGraphData();
    } catch (err) {
      console.error('Error cargando datos del grafo:', err);
    } finally {
      isLoading = false;
    }
  }

  function syncGraphData() {
    if (!graphInstance || !containerEl) return;

    graphInstance.graphData({
      nodes: filteredNodes.map((n) => ({ ...n })),
      links: filteredLinks.map((l) => ({ ...l })),
    });

    if (!isGlobalMode) {
      setTimeout(() => {
        if (graphInstance) graphInstance.zoomToFit(400, 40);
      }, 300);
    }
  }

  // Sincronizar datos si cambia el toggle de nodos fantasma
  $effect(() => {
    if (showGhostNodes !== undefined && rawGraphData.nodes.length > 0) {
      syncGraphData();
    }
  });

  function initForceGraph() {
    if (!containerEl) return;

    const width = containerEl.clientWidth || 800;
    const height = containerEl.clientHeight || 600;

    graphInstance = new ForceGraph(containerEl)
      .width(width)
      .height(height)
      .backgroundColor('transparent')
      .nodeId('id')
      .nodeLabel((node: any) => `${node.label}${node.is_ghost ? ' (Nota pendiente por crear)' : ` (${node.id})`}`)
      .nodeVal((node: any) => (node.is_ghost ? 2.5 : Math.max(3, Math.min(node.val * 2, 18))))
      .nodeCanvasObject((node: any, ctx: CanvasRenderingContext2D, globalScale: number) => {
        const isMatched = searchQuery.trim() &&
          (node.label.toLowerCase().includes(searchQuery.toLowerCase()) ||
           node.id.toLowerCase().includes(searchQuery.toLowerCase()));
        const isFocused = activeNotePath && node.id === activeNotePath;
        const radius = Math.max(3, Math.min((node.val || 1) * 2, 16));

        // Dibujar círculo del nodo
        ctx.beginPath();
        ctx.arc(node.x, node.y, radius, 0, 2 * Math.PI, false);

        if (node.is_ghost) {
          // Estilo fantasma: trazo punteado gris/lavanda
          ctx.setLineDash([3, 2]);
          ctx.strokeStyle = '#9ca3af';
          ctx.lineWidth = 1.5;
          ctx.stroke();
          ctx.fillStyle = 'rgba(156, 163, 175, 0.15)';
          ctx.fill();
          ctx.setLineDash([]);
        } else {
          // Color estándar según grupo o resaltado
          ctx.fillStyle = isFocused ? '#2563eb' : (isMatched ? '#ea580c' : (node.color || '#3b82f6'));
          ctx.fill();

          if (isMatched || isFocused) {
            ctx.lineWidth = 2;
            ctx.strokeStyle = '#ffffff';
            ctx.stroke();
          }
        }

        // Mostrar etiqueta de texto si hay zoom suficiente o si coincide con la búsqueda
        const showLabel = globalScale >= 1.2 || isMatched || isFocused;
        if (showLabel) {
          const fontSize = Math.max(10 / globalScale, 3);
          ctx.font = `${fontSize}px sans-serif`;
          ctx.textAlign = 'center';
          ctx.textBaseline = 'top';
          ctx.fillStyle = node.is_ghost ? '#6b7280' : (isMatched ? '#c2410c' : '#374151');
          ctx.fillText(node.label, node.x, node.y + radius + 2);
        }
      })
      .linkSource('source')
      .linkTarget('target')
      .linkColor((link: any) => {
        const targetNode = typeof link.target === 'object' ? link.target : null;
        if (targetNode?.is_ghost) {
          return 'rgba(156, 163, 175, 0.35)';
        }
        return 'rgba(150, 150, 150, 0.35)';
      })
      .linkDirectionalParticles(2)
      .linkDirectionalParticleSpeed(0.006)
      .linkDirectionalParticleWidth(1.5)
      .onNodeClick((node: any) => {
        if (node && node.id && onSelectNote) {
          onSelectNote(node.id);
        }
      })
      .d3AlphaDecay(0.02)
      .d3VelocityDecay(0.3);

    loadGraph();
  }

  // Recargar al cambiar de nota activa en modo local
  $effect(() => {
    if (!isGlobalMode && activeNotePath !== undefined) {
      loadGraph();
    }
  });

  // Re-ajustar tamaño al redimensionar ventana
  function handleResize() {
    if (graphInstance && containerEl) {
      graphInstance.width(containerEl.clientWidth).height(containerEl.clientHeight);
    }
  }

  onMount(() => {
    initForceGraph();
    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      if (graphInstance) {
        try {
          graphInstance._destructor?.();
        } catch {
          // Ignorar si ya fue desmontado
        }
        graphInstance = null;
      }
    };
  });
</script>

<div class="graph-viewer-wrapper">
  <!-- Barra de Controles Superior -->
  <div class="graph-toolbar">
    <div class="mode-toggles">
      <button
        class="toggle-btn"
        class:active={!isGlobalMode}
        onclick={() => {
          isGlobalMode = false;
          loadGraph();
        }}
        title="Mostrar sólo conexiones directas de la nota actual"
      >
        <Crosshair size={14} />
        <span>Local</span>
      </button>

      <button
        class="toggle-btn"
        class:active={isGlobalMode}
        onclick={() => {
          isGlobalMode = true;
          loadGraph();
        }}
        title="Mostrar la galaxia completa de notas de la bóveda"
      >
        <Globe size={14} />
        <span>Todo el Vault</span>
      </button>
    </div>

    {#if !isGlobalMode}
      <div class="depth-selector">
        <Layers size={13} class="icon-muted" />
        <span class="label">Saltos:</span>
        <select
          value={depth}
          onchange={(e) => {
            depth = Number((e.target as HTMLSelectElement).value);
            loadGraph();
          }}
          class="depth-select"
        >
          <option value={1}>1 nivel</option>
          <option value={2}>2 niveles</option>
          <option value={3}>3 niveles</option>
        </select>
      </div>
    {/if}

    <!-- Filtro de Nodos Fantasma -->
    <button
      class="toggle-ghost-btn"
      class:active={showGhostNodes}
      onclick={() => (showGhostNodes = !showGhostNodes)}
      title={showGhostNodes ? 'Ocultar notas pendientes/fantasma' : 'Mostrar notas pendientes/fantasma'}
    >
      <Ghost size={13} />
      <span>Fantasmas</span>
    </button>

    <!-- Barra de Búsqueda Rápida de Nodos -->
    <div class="graph-search-bar">
      <Search size={13} class="search-icon" />
      <input
        type="text"
        placeholder="Buscar nodo..."
        bind:value={searchQuery}
        class="search-input"
      />
    </div>

    <div class="stats-badge">
      <span>{totalNodes} nodos</span>
      <span class="dot">•</span>
      <span>{totalLinks} enlaces</span>
    </div>

    <button
      class="refresh-btn"
      onclick={loadGraph}
      title="Recargar Grafo"
      disabled={isLoading}
    >
      <span class={isLoading ? 'spinning' : ''} style="display: inline-flex;">
        <RefreshCw size={13} />
      </span>
    </button>
  </div>

  <!-- Contenedor del Canvas de force-graph -->
  <div class="graph-canvas-container" bind:this={containerEl}></div>
</div>

<style>
  .graph-viewer-wrapper {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background-color: var(--bg-primary, #ffffff);
  }

  .graph-toolbar {
    position: absolute;
    top: 12px;
    left: 12px;
    right: 12px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 12px;
    border-radius: 8px;
    background-color: var(--bg-secondary, rgba(246, 248, 250, 0.85));
    backdrop-filter: blur(8px);
    border: 1px solid var(--border-primary, rgba(208, 215, 222, 0.8));
    z-index: 10;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
  }

  .mode-toggles {
    display: inline-flex;
    background-color: var(--bg-tertiary, #eaeef2);
    border-radius: 6px;
    padding: 2px;
  }

  .toggle-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    border: none;
    background: transparent;
    padding: 4px 10px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary, #656d76);
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .toggle-btn.active {
    background-color: var(--bg-primary, #ffffff);
    color: var(--accent, #0969da);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .depth-selector {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-secondary, #656d76);
  }

  .depth-select {
    border: 1px solid var(--border-primary, #d0d7de);
    background-color: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
    font-size: 11px;
    border-radius: 4px;
    padding: 2px 6px;
    cursor: pointer;
  }

  .toggle-ghost-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 8px;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary, #656d76);
    background: transparent;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .toggle-ghost-btn:hover {
    background-color: var(--bg-tertiary, #eaeef2);
  }

  .toggle-ghost-btn.active {
    background-color: var(--bg-primary, #ffffff);
    color: var(--accent, #0969da);
    border-color: var(--accent, #0969da);
  }

  .graph-search-bar {
    position: relative;
    display: flex;
    align-items: center;
  }

  :global(.search-icon) {
    position: absolute;
    left: 8px;
    color: var(--text-muted, #8c959f);
  }

  .search-input {
    width: 130px;
    padding: 2px 6px 2px 24px;
    font-size: 11px;
    background-color: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 4px;
    color: var(--text-primary, #1f2328);
    outline: none;
    transition: width 0.2s ease;
  }

  .search-input:focus {
    width: 170px;
    border-color: var(--accent, #0969da);
  }

  .stats-badge {
    margin-left: auto;
    font-size: 11px;
    color: var(--text-muted, #8c959f);
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .dot {
    opacity: 0.5;
  }

  .refresh-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--text-secondary, #656d76);
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .refresh-btn:hover {
    background-color: var(--bg-tertiary, #eaeef2);
    color: var(--text-primary, #1f2328);
  }

  .spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .graph-canvas-container {
    flex: 1;
    width: 100%;
    height: 100%;
  }
</style>
