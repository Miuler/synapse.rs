<script lang="ts">
  import { onMount } from 'svelte';
  import ForceGraph from 'force-graph';
  import type { VaultGraphData, GraphNode } from '@entities/vault-item';
  import { vaultApi } from '@entities/vault-item';
  import { Globe, Crosshair, RefreshCw, Layers } from 'lucide-svelte';

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
  let graphData = $state<VaultGraphData>({ nodes: [], links: [] });

  let totalNodes = $derived(graphData.nodes.length);
  let totalLinks = $derived(graphData.links.length);

  async function loadGraph() {
    isLoading = true;
    try {
      const focus = isGlobalMode ? null : activeNotePath;
      const data = await vaultApi.getVaultGraph(focus, isGlobalMode ? null : depth);
      graphData = data;

      if (graphInstance && containerEl) {
        graphInstance
          .graphData({
            nodes: data.nodes.map((n) => ({ ...n })),
            links: data.links.map((l) => ({ ...l })),
          });

        if (!isGlobalMode) {
          // Zoom suave centrado en el nodo activo
          setTimeout(() => {
            if (graphInstance) graphInstance.zoomToFit(400, 40);
          }, 300);
        }
      }
    } catch (err) {
      console.error('Error cargando datos del grafo:', err);
    } finally {
      isLoading = false;
    }
  }

  function initForceGraph() {
    if (!containerEl) return;

    const width = containerEl.clientWidth || 800;
    const height = containerEl.clientHeight || 600;

    graphInstance = new ForceGraph(containerEl)
      .width(width)
      .height(height)
      .backgroundColor('transparent')
      .nodeId('id')
      .nodeLabel((node: any) => `${node.label} (${node.id})`)
      .nodeVal((node: any) => Math.max(3, Math.min(node.val * 2, 18)))
      .nodeAutoColorBy('group')
      .linkSource('source')
      .linkTarget('target')
      .linkColor(() => 'rgba(150, 150, 150, 0.35)')
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
