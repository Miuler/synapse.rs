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

  function config(spec: string, isDark: boolean) {
    return {
      content: spec,
      forceDarkModeState: (isDark ? 'dark' : 'light') as 'dark' | 'light',
      hideDarkModeToggle: true,
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
  });

  onDestroy(() => {
    destroyed = true;
    observer?.disconnect();
    instance?.destroy();
    instance = null;
  });
</script>

<div class="openapi-viewer" bind:this={host}></div>

<style>
  .openapi-viewer {
    height: 100%;
    overflow: auto;
    /* Scalar usa su propio tipo de letra por defecto; se hereda el de la app */
    --scalar-font: var(--font-interface, inherit);
    --scalar-font-code: var(--code-font, monospace);
  }
</style>
