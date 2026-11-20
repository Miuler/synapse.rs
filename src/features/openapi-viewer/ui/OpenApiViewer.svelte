<script lang="ts">
  // Visor de especificaciones OpenAPI/Swagger con Scalar (@scalar/api-reference, empaquetado por Vite).
  // Sin CDN, sin fuentes externas, sin telemetría ni cliente HTTP integrado.
  // Con `path` y `readFile`, los `$ref` a otros archivos se resuelven antes (ver lib/bundle.ts).
  import { onDestroy, onMount } from 'svelte';
  import { bundleOpenApi, hasExternalRefs, type ReadFile } from '../lib/bundle';
  type ApiReferenceInstance = ReturnType<typeof import('@scalar/api-reference').createApiReference>;

  interface Props {
    content: string;
    /** Ruta del archivo (relativa a la bóveda), para resolver `$ref` relativos */
    path?: string;
    /** Lector de los archivos referenciados; sin él no se siguen referencias externas */
    readFile?: ReadFile;
    /** Re-resolver cuando cambie en disco un archivo referenciado (no para revisiones de Git) */
    watchFiles?: boolean;
  }
  let { content, path, readFile, watchFiles = false }: Props = $props();

  let resolved = $state<string | null>(null);
  let problems = $state<string[]>([]);
  let refFiles: string[] = [];
  let bumped = $state(0);
  let unlisten: (() => void) | null = null;

  let host: HTMLDivElement;
  let instance: ApiReferenceInstance | null = null;
  let destroyed = false;
  let dark = $state(document.documentElement.getAttribute('data-theme') === 'dark');
  let observer: MutationObserver | null = null;

  // Colores de Scalar tomados del tema activo de la app (claro/oscuro y temas como Papel o Neón)
  const SYNAPSE_THEME_CSS = `
.openapi-viewer .light-mode, .openapi-viewer .dark-mode, .openapi-viewer .scalar-app {
  --scalar-background-1: var(--bg-primary);
  --scalar-background-2: var(--bg-secondary);
  --scalar-background-3: var(--bg-tertiary);
  --scalar-background-accent: var(--accent-bg);
  --scalar-color-1: var(--text-primary);
  --scalar-color-2: var(--text-secondary);
  --scalar-color-3: var(--text-secondary);
  --scalar-color-accent: var(--accent);
  --scalar-border-color: var(--border-primary);
  --scalar-link-color: var(--accent);
  --scalar-link-color-hover: var(--accent);
  --scalar-button-1: var(--accent);
  --scalar-button-1-color: var(--bg-primary);
  --scalar-sidebar-background-1: var(--bg-secondary);
  --scalar-sidebar-color-1: var(--text-primary);
  --scalar-sidebar-color-2: var(--text-secondary);
  --scalar-sidebar-border-color: var(--border-primary);
  --scalar-sidebar-item-hover-background: var(--hover-bg);
  --scalar-sidebar-item-active-background: var(--active-bg);
  --scalar-sidebar-color-active: var(--accent);
  --scalar-sidebar-search-background: var(--bg-primary);
  --scalar-sidebar-search-border-color: var(--border-primary);
  --scalar-color-red: var(--syntax-keyword);
  --scalar-color-green: var(--syntax-type);
  --scalar-color-blue: var(--syntax-number);
  --scalar-color-orange: var(--syntax-variable);
  --scalar-color-purple: var(--syntax-function);
  --scalar-font: var(--font-interface, inherit);
  --scalar-font-code: var(--code-font, monospace);
}`;

  function config(spec: string, isDark: boolean) {
    return {
      content: spec,
      forceDarkModeState: (isDark ? 'dark' : 'light') as 'dark' | 'light',
      hideDarkModeToggle: true,
      theme: 'none' as const,
      customCss: SYNAPSE_THEME_CSS,
      withDefaultFonts: false,
      telemetry: false,
      hideClientButton: true,
      showDeveloperTools: 'never' as const,
      showToolbar: 'never' as const,
      // Sin «Ask AI»: el chat de Scalar envía la especificación a sus servidores
      agent: { disabled: true },
      // Que Scalar no busque nada fuera (fuentes, proxy, «Open in client»)
      proxyUrl: undefined,
    };
  }

  onMount(() => {
    // Carga diferida: Scalar (Vue + estilos) solo entra cuando se abre un OpenAPI
    void (async () => {
      const [{ createApiReference }] = await Promise.all([
        import('@scalar/api-reference'),
        import('@scalar/api-reference/style.css'),
      ]);
      if (destroyed) return;
      instance = createApiReference(host, config(resolved ?? content, dark));
    })();
    observer = new MutationObserver(() => {
      dark = document.documentElement.getAttribute('data-theme') === 'dark';
    });
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
    if (watchFiles) {
      void import('@tauri-apps/api/event')
        .then(({ listen }) =>
          listen<{ paths: string[]; deleted: string[] }>('vault:files-changed', (e) => {
            const norm = (x: string) => x.replace(/\\/g, '/').replace(/^\.\//, '');
            const changed = [...e.payload.paths, ...e.payload.deleted].map(norm);
            if (changed.some((c) => refFiles.includes(c))) bumped++;
          }),
        )
        .then((u) => {
          if (destroyed) u();
          else unlisten = u;
        })
        .catch(() => {});
    }
  });

  // Junta los archivos referenciados (y otra vez si alguno cambia en disco)
  $effect(() => {
    const src = content;
    const p = path;
    const reader = readFile;
    void bumped;
    if (!p || !reader || !hasExternalRefs(src)) {
      resolved = null;
      problems = [];
      refFiles = [];
      return;
    }
    let stale = false;
    bundleOpenApi(src, p, reader)
      .then((r) => {
        if (stale) return;
        resolved = r.content;
        problems = r.problems;
        refFiles = r.files;
      })
      .catch((e) => {
        if (!stale) problems = [String(e)];
      });
    return () => {
      stale = true;
    };
  });

  $effect(() => {
    const c = config(resolved ?? content, dark);
    instance?.updateConfiguration(c);
    // Scalar solo lee forceDarkModeState al montarse: el cambio de tema se aplica a mano en <body>
    if (instance) {
      document.body.classList.toggle('dark-mode', dark);
      document.body.classList.toggle('light-mode', !dark);
    }
  });

  onDestroy(() => {
    destroyed = true;
    observer?.disconnect();
    unlisten?.();
    instance?.destroy();
    instance = null;
    // Scalar marca el modo en <body>; si no queda otro visor abierto, se limpia
    if (!document.querySelector('.openapi-viewer .scalar-app')) document.body.classList.remove('dark-mode', 'light-mode');
  });
</script>

<div class="openapi-wrap">
  {#if problems.length}
    <details class="openapi-problems" role="alert">
      <summary>⚠ {problems.length === 1 ? 'Una referencia' : `${problems.length} referencias`} sin resolver</summary>
      <ul>
        {#each problems as p}<li>{p}</li>{/each}
      </ul>
    </details>
  {/if}
  <div class="openapi-viewer" bind:this={host}></div>
</div>

<style>
  .openapi-wrap {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .openapi-viewer {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .openapi-problems {
    padding: 6px 12px;
    font-size: 12px;
    color: var(--warning);
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-primary);
  }
  .openapi-problems ul {
    margin: 4px 0 0;
    padding-left: 18px;
    color: var(--text-secondary);
  }
</style>
