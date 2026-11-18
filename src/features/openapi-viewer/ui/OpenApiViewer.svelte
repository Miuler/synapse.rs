<script lang="ts">
  // Visor de especificaciones OpenAPI/Swagger con Scalar (@scalar/api-reference, empaquetado por Vite).
  // Sin CDN, sin fuentes externas, sin telemetría ni cliente HTTP integrado.
  import { onDestroy, onMount } from 'svelte';
  type ApiReferenceInstance = ReturnType<typeof import('@scalar/api-reference').createApiReference>;

  interface Props {
    content: string;
  }
  let { content }: Props = $props();

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
      instance = createApiReference(host, config(content, dark));
    })();
    observer = new MutationObserver(() => {
      dark = document.documentElement.getAttribute('data-theme') === 'dark';
    });
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
  });

  $effect(() => {
    const c = config(content, dark);
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
    instance?.destroy();
    instance = null;
    // Scalar marca el modo en <body>; si no queda otro visor abierto, se limpia
    if (!document.querySelector('.openapi-viewer .scalar-app')) document.body.classList.remove('dark-mode', 'light-mode');
  });
</script>

<div class="openapi-viewer" bind:this={host}></div>

<style>
  .openapi-viewer {
    height: 100%;
    overflow: auto;
  }
</style>
