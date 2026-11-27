// Atajos del puntero láser en un visor: `K` lo activa o desactiva y `Escape` lo desactiva.
// Solo actúan si el puntero está sobre el visor o el foco está dentro, y nunca mientras se
// escribe (un editor de código al lado, un buscador…).
import { isTyping } from '@shared/lib/navigation';

export interface LaserShortcutsOptions {
  isActive: () => boolean;
  setActive: (active: boolean) => void;
}

/** Acción de Svelte para el contenedor del visor: `use:laserShortcuts={{ isActive, setActive }}`. */
export function laserShortcuts(node: HTMLElement, options: LaserShortcutsOptions) {
  let opts = options;
  const onKeydown = (e: KeyboardEvent) => {
    if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey || isTyping(e.target)) return;
    const here = node.matches(':hover') || node.contains(document.activeElement);
    if (!here) return;
    if (e.key === 'k' || e.key === 'K') {
      opts.setActive(!opts.isActive());
      e.preventDefault();
    } else if (e.key === 'Escape' && opts.isActive()) {
      opts.setActive(false);
      e.preventDefault();
    }
  };
  window.addEventListener('keydown', onKeydown);
  return {
    update(next: LaserShortcutsOptions) {
      opts = next;
    },
    destroy() {
      window.removeEventListener('keydown', onKeydown);
    },
  };
}
