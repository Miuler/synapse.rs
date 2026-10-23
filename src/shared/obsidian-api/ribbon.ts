import { createDiv } from './dom';

export interface RibbonIconItem {
  id: string;
  icon: string;
  title: string;
  callback: (evt: MouseEvent) => any;
  element: HTMLElement;
}

/**
 * Gestor de iconos de la barra lateral Ribbon de Synapse.
 */
export class SynapseRibbonManager {
  readonly icons: RibbonIconItem[] = [];

  addRibbonIcon(icon: string, title: string, callback: (evt: MouseEvent) => any): HTMLElement {
    const el = createDiv({ cls: 'side-dock-ribbon-action clickable-icon', title });
    el.setAttribute('aria-label', title);
    el.addEventListener('click', callback as any);

    const item: RibbonIconItem = {
      id: `ribbon-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
      icon,
      title,
      callback,
      element: el,
    };

    this.icons.push(item);
    return el;
  }

  removeRibbonIcon(el: HTMLElement): void {
    const idx = this.icons.findIndex((i) => i.element === el);
    if (idx !== -1) {
      this.icons.splice(idx, 1);
      el.remove();
    }
  }
}

/**
 * Gestor de elementos de la barra de estado inferior (StatusBar) de Synapse.
 */
export class SynapseStatusBarManager {
  readonly items: HTMLElement[] = [];

  addStatusBarItem(): HTMLElement {
    const el = createDiv({ cls: 'status-bar-item plugin-status-item' });
    this.items.push(el);
    return el;
  }

  removeStatusBarItem(el: HTMLElement): void {
    const idx = this.items.indexOf(el);
    if (idx !== -1) {
      this.items.splice(idx, 1);
      el.remove();
    }
  }
}
