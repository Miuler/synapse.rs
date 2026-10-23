import { Component } from './component';
import { createDiv, createEl, empty } from './dom';
import type { SynapseApp } from './app';

/**
 * Notificación flotante en pantalla (Notice de Obsidian).
 */
export class Notice {
  readonly noticeEl: HTMLElement;
  private hideTimer?: ReturnType<typeof setTimeout>;

  constructor(message: string | DocumentFragment, duration = 5000) {
    let container = typeof document !== 'undefined' ? document.querySelector('.notice-container') as HTMLElement : null;
    if (!container && typeof document !== 'undefined' && document.body) {
      container = createDiv({ cls: 'notice-container' });
      document.body.appendChild(container);
    }

    this.noticeEl = createDiv({ cls: 'notice', parent: container || undefined });
    this.setMessage(message);

    this.noticeEl.addEventListener('click', () => {
      this.hide();
    });

    if (duration > 0) {
      this.hideTimer = setTimeout(() => {
        this.hide();
      }, duration);
    }
  }

  setMessage(message: string | DocumentFragment): this {
    if (typeof message === 'string') {
      this.noticeEl.textContent = message;
    } else if (message && (message as any).cloneNode) {
      empty(this.noticeEl);
      this.noticeEl.appendChild(message);
    }
    return this;
  }

  hide(): void {
    if (this.hideTimer) {
      clearTimeout(this.hideTimer);
      this.hideTimer = undefined;
    }
    this.noticeEl.remove();
  }
}

/**
 * Diálogo modal accesible de Obsidian (Modal).
 */
export class Modal extends Component {
  readonly app: SynapseApp;
  readonly containerEl: HTMLElement;
  readonly modalEl: HTMLElement;
  readonly titleEl: HTMLElement;
  readonly contentEl: HTMLElement;
  readonly closeButtonEl: HTMLElement;
  isOpen = false;

  constructor(app: SynapseApp) {
    super();
    this.app = app;

    this.containerEl = createDiv({ cls: 'modal-container mod-dim' });
    const bg = createDiv({ cls: 'modal-bg', parent: this.containerEl });
    bg.addEventListener('click', () => this.close());

    this.modalEl = createDiv({ cls: 'modal', parent: this.containerEl });
    this.closeButtonEl = createDiv({ cls: 'modal-close-button', parent: this.modalEl });
    this.closeButtonEl.setAttribute('aria-label', 'Cerrar');
    this.closeButtonEl.addEventListener('click', () => this.close());

    this.titleEl = createDiv({ cls: 'modal-title', parent: this.modalEl });
    this.contentEl = createDiv({ cls: 'modal-content', parent: this.modalEl });

    // Escuchar Escape para cerrar
    this.registerDomEvent(this.containerEl, 'keydown', (evt: KeyboardEvent) => {
      if (evt.key === 'Escape') {
        this.close();
      }
    });
  }

  open(): void {
    if (this.isOpen) return;
    this.isOpen = true;
    if (typeof document !== 'undefined' && document.body) {
      document.body.appendChild(this.containerEl);
    }
    this.load();
    this.onOpen();
  }

  close(): void {
    if (!this.isOpen) return;
    this.isOpen = false;
    this.onClose();
    this.containerEl.remove();
    this.unload();
  }

  onOpen(): void {}
  onClose(): void {}

  setTitle(title: string): this {
    this.titleEl.textContent = title;
    return this;
  }

  setContent(content: string | DocumentFragment): this {
    if (typeof content === 'string') {
      this.contentEl.textContent = content;
    } else if (content && (content as any).cloneNode) {
      empty(this.contentEl);
      this.contentEl.appendChild(content);
    }
    return this;
  }
}

/**
 * Elemento de menú contextual (MenuItem).
 */
export class MenuItem {
  title = '';
  icon = '';
  active = false;
  disabled = false;
  checked: boolean | null = null;
  section = '';
  private clickCallback?: (evt: MouseEvent | KeyboardEvent) => any;
  readonly dom: HTMLElement;

  constructor(containerEl: HTMLElement) {
    this.dom = createDiv({ cls: 'menu-item', parent: containerEl });
    this.dom.addEventListener('click', (e) => {
      if (this.disabled) return;
      this.clickCallback?.(e);
    });
  }

  setTitle(title: string | DocumentFragment): this {
    this.title = typeof title === 'string' ? title : '';
    if (typeof title === 'string') {
      this.dom.textContent = title;
    } else if (title && (title as any).cloneNode) {
      empty(this.dom);
      this.dom.appendChild(title);
    }
    return this;
  }

  setIcon(icon: string): this {
    this.icon = icon;
    return this;
  }

  setActive(active: boolean): this {
    this.active = active;
    if (active) this.dom.classList.add('is-active');
    else this.dom.classList.remove('is-active');
    return this;
  }

  setDisabled(disabled: boolean): this {
    this.disabled = disabled;
    if (disabled) this.dom.classList.add('is-disabled');
    else this.dom.classList.remove('is-disabled');
    return this;
  }

  setChecked(checked: boolean | null): this {
    this.checked = checked;
    return this;
  }

  setSection(section: string): this {
    this.section = section;
    return this;
  }

  onClick(callback: (evt: MouseEvent | KeyboardEvent) => any): this {
    this.clickCallback = callback;
    return this;
  }
}

/**
 * Menú contextual nativo de Obsidian (Menu).
 */
export class Menu extends Component {
  readonly items: MenuItem[] = [];
  readonly dom: HTMLElement;

  constructor() {
    super();
    this.dom = createDiv({ cls: 'menu' });
  }

  addItem(cb: (item: MenuItem) => any): this {
    const item = new MenuItem(this.dom);
    cb(item);
    this.items.push(item);
    return this;
  }

  addSeparator(): this {
    createEl('hr', { cls: 'menu-separator', parent: this.dom });
    return this;
  }

  showAtPosition(pos: { x: number; y: number }): this {
    this.dom.style.position = 'fixed';
    this.dom.style.left = `${pos.x}px`;
    this.dom.style.top = `${pos.y}px`;
    this.dom.style.zIndex = '9999';
    if (typeof document !== 'undefined' && document.body) {
      document.body.appendChild(this.dom);
    }
    return this;
  }

  showAtMouseEvent(evt: MouseEvent): this {
    return this.showAtPosition({ x: evt.clientX, y: evt.clientY });
  }

  hide(): void {
    this.dom.remove();
  }
}

/**
 * Modal de sugerencias con búsqueda difusa (FuzzySuggestModal).
 */
export abstract class FuzzySuggestModal<T> extends Modal {
  abstract getItems(): T[];
  abstract getItemText(item: T): string;
  abstract onChooseItem(item: T, evt: MouseEvent | KeyboardEvent): void;
}

/**
 * Gestor de atajos de teclado y alcance de eventos de Obsidian.
 */
export class Scope {
  register(
    _modifiers: string[],
    _key: string | null,
    _func: (evt: KeyboardEvent) => any
  ): any {
    return {};
  }
  unregister(_key: any): void {}
}

/**
 * Clase base para sugerencias de autocompletado en campos de entrada.
 */
export abstract class AbstractInputSuggest<T> {
  constructor(protected app: any, protected textInputEl: HTMLInputElement | HTMLTextAreaElement) {}
  abstract getSuggestions(inputStr: string): T[] | Promise<T[]>;
  abstract renderSuggestion(item: T, el: HTMLElement): void;
  abstract selectSuggestion(item: T, evt: MouseEvent | KeyboardEvent): void;
}

/**
 * Clase base para autocompletado contextual en el editor de Markdown.
 */
export abstract class EditorSuggest<T> extends Component {
  constructor(public app: any) {
    super();
  }
  abstract onTrigger(cursor: any, editor: any, file: any): any;
  abstract getSuggestions(context: any): T[] | Promise<T[]>;
  abstract renderSuggestion(value: T, el: HTMLElement): void;
  abstract selectSuggestion(value: T, evt: MouseEvent | KeyboardEvent): void;
}
