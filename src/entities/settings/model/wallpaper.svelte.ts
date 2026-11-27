// Imagen de fondo de la bóveda: la de la pestaña vacía y la marca de agua de los index.md
import { convertFileSrc } from '@tauri-apps/api/core';
// Fondos por defecto según el tema; Vite los empaqueta y van dentro del binario
import defaultWallpaperDark from '@shared/assets/synapsers_w_dark.webp';
import defaultWallpaperLight from '@shared/assets/synapsers_w_light.webp';
import { appSettings, NO_WALLPAPER } from './settings.svelte';

/**
 * El fondo como variables CSS (`undefined` si se eligió «Ninguno»): `--wallpaper` con una imagen
 * propia; si no, `--wallpaper-light` y `--wallpaper-dark`, y el CSS elige según el tema.
 * Se llama al inicializar un componente (usa `$effect`).
 */
export function useWallpaper() {
  // Si el fondo ya no se puede leer (se borró o no está autorizado), queda el de por defecto
  let broken = $state<string | null>(null);
  const none = $derived(appSettings.wallpaper === NO_WALLPAPER);
  const own = $derived(none ? undefined : appSettings.wallpaper);
  $effect(() => {
    const path = own;
    if (!path) return;
    broken = null;
    const probe = new Image();
    probe.onerror = () => (broken = path);
    probe.src = convertFileSrc(path);
    return () => (probe.onerror = null);
  });
  const style = $derived(
    none
      ? undefined
      : own && own !== broken
        ? `--wallpaper: url("${convertFileSrc(own)}")`
        : `--wallpaper-light: url("${defaultWallpaperLight}"); --wallpaper-dark: url("${defaultWallpaperDark}")`,
  );
  return {
    get style(): string | undefined {
      return style;
    },
  };
}
