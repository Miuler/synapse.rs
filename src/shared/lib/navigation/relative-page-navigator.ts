export interface RelativePageOptions {
  /** Contenedor scrollable (listContainerEl, sidebarContentEl, etc.) */
  containerEl: HTMLElement | null;
  /** Dirección del salto */
  direction: 'pagedown' | 'pageup';
  /** Colección o array de elementos navegables en el DOM */
  items: ArrayLike<HTMLElement>;
  /** Altura estándar estimada del ítem en px (fallback) */
  fallbackItemHeight?: number;
  /** Callback para sincronizar el nuevo valor o nodo seleccionado */
  onSelect: (targetItem: HTMLElement, targetIndex: number) => void;
  /** Callback opcional cuando el salto se acerca al final de los elementos montados */
  onNearEnd?: (targetIndex: number) => void;
}

/**
 * Ejecuta un salto de página (PageDown / PageUp) conservando la posición vertical
 * relativa (misma altura en pantalla o fila visual) del elemento seleccionado.
 *
 * Si en el destino hay menos elementos que la fila relativa actual, aterriza en el
 * elemento disponible más cercano (el último o el primero).
 */
export function handleRelativePageJump(options: RelativePageOptions): boolean {
  const {
    containerEl,
    direction,
    items,
    fallbackItemHeight = 40,
    onSelect,
    onNearEnd,
  } = options;

  if (!containerEl || items.length === 0) return false;

  const count = items.length;

  // 1. Obtener altura visible y salto en número de ítems
  const firstItem = items[0];
  const itemHeight = firstItem?.getBoundingClientRect().height || fallbackItemHeight;
  const visibleHeight = containerEl.clientHeight || 400;
  const pageJump = Math.max(1, Math.floor(visibleHeight / itemHeight));

  // 2. Localizar el índice actualmente seleccionado en la lista
  let currentIndex = -1;
  for (let i = 0; i < count; i++) {
    const el = items[i];
    if (
      el.hasAttribute('data-highlighted') ||
      el.hasAttribute('data-selected') ||
      el.classList.contains('selected')
    ) {
      currentIndex = i;
      break;
    }
  }
  const baseIndex = currentIndex === -1 ? 0 : currentIndex;
  const currentItem = items[baseIndex];

  // 3. Calcular el índice objetivo respetando los límites (clamping)
  let targetIndex: number;
  if (direction === 'pagedown') {
    targetIndex = Math.min(baseIndex + pageJump, count - 1);
    if (onNearEnd && targetIndex >= count - pageJump) {
      onNearEnd(targetIndex);
    }
  } else {
    targetIndex = Math.max(baseIndex - pageJump, 0);
  }

  const targetItem = items[targetIndex];
  if (!targetItem) return false;

  // 4. Notificar selección al consumidor
  onSelect(targetItem, targetIndex);

  // 5. Ajustar scroll conservando exactamente la posición relativa en pantalla (deltaY)
  if (currentItem && currentItem !== targetItem) {
    const currentRect = currentItem.getBoundingClientRect();
    const targetRect = targetItem.getBoundingClientRect();
    const deltaY = targetRect.top - currentRect.top;
    containerEl.scrollTop += deltaY;
  } else {
    targetItem.scrollIntoView({ block: 'nearest', behavior: 'auto' });
  }

  // Sincronizar foco y eventos de hover
  targetItem.dispatchEvent(new PointerEvent('pointermove', { bubbles: true }));

  return true;
}
