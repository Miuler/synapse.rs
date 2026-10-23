import type { EventRef } from './events';

/**
 * Clase base canónica Component de Obsidian.
 * Gestiona el ciclo de vida (load, unload) y el registro automático de eventos,
 * intervalos y listeners del DOM para evitar fugas de memoria al descargar vistas o plugins.
 */
export class Component {
  private _loaded = false;
  private _children: Component[] = [];
  private _eventRefs: EventRef[] = [];
  private _domListeners: Array<{
    el: EventTarget;
    type: string;
    callback: EventListenerOrEventListenerObject;
    options?: boolean | AddEventListenerOptions;
  }> = [];
  private _intervals: number[] = [];
  private _cleanups: Array<() => any> = [];

  get loaded(): boolean {
    return this._loaded;
  }

  /**
   * Carga el componente y activa sus subcomponentes.
   */
  load(): void {
    if (this._loaded) return;
    this._loaded = true;
    try {
      this.onload();
    } catch (e) {
      console.error('[Component] Error en onload():', e);
    }
    for (const child of this._children) {
      child.load();
    }
  }

  /**
   * Descarga el componente, cancela eventos registrados y limpia listeners DOM e intervalos.
   */
  unload(): void {
    if (!this._loaded) return;

    // 1. Descargar componentes hijos en orden inverso
    for (let i = this._children.length - 1; i >= 0; i--) {
      this._children[i].unload();
    }
    this._children = [];

    // 2. Invocar onunload del ciclo de vida
    try {
      this.onunload();
    } catch (e) {
      console.error('[Component] Error en onunload():', e);
    }

    // 3. Cancelar referencias de eventos registradas
    for (const ref of this._eventRefs) {
      if (ref.e && typeof ref.e.offref === 'function') {
        ref.e.offref(ref);
      }
    }
    this._eventRefs = [];

    // 4. Remover listeners DOM
    for (const item of this._domListeners) {
      try {
        item.el.removeEventListener(item.type, item.callback, item.options);
      } catch (e) {
        console.warn('[Component] Error al remover listener DOM:', e);
      }
    }
    this._domListeners = [];

    // 5. Limpiar intervalos
    for (const id of this._intervals) {
      clearInterval(id);
    }
    this._intervals = [];

    // 6. Ejecutar callbacks de limpieza genéricos
    for (const cleanup of this._cleanups) {
      try {
        cleanup();
      } catch (e) {
        console.warn('[Component] Error en cleanup callback:', e);
      }
    }
    this._cleanups = [];

    this._loaded = false;
  }

  /** Método de ciclo de vida invocado al cargar. Sobrescrito por vistas y plugins. */
  onload(): void {}

  /** Método de ciclo de vida invocado al descargar. Sobrescrito por vistas y plugins. */
  onunload(): void {}

  /**
   * Registra un subcomponente hijo cuyo ciclo de vida queda vinculado a este componente.
   */
  addChild<T extends Component>(component: T): T {
    this._children.push(component);
    if (this._loaded) {
      component.load();
    }
    return component;
  }

  /**
   * Remueve y descarga un componente hijo.
   */
  removeChild<T extends Component>(component: T): T {
    const idx = this._children.indexOf(component);
    if (idx !== -1) {
      this._children.splice(idx, 1);
      component.unload();
    }
    return component;
  }

  /**
   * Registra un EventRef devuelto por app.vault.on, app.workspace.on, etc.
   * Se desuscribirá automáticamente cuando este componente se descargue.
   */
  registerEvent(eventRef: EventRef): void {
    if (!eventRef) return;
    this._eventRefs.push(eventRef);
  }

  /**
   * Registra un listener de eventos del DOM en un elemento, window o document.
   * Se removerá automáticamente al descargar el componente.
   */
  registerDomEvent<K extends keyof HTMLElementEventMap>(
    el: EventTarget,
    type: K | string,
    callback: (this: HTMLElement, ev: any) => any,
    options?: boolean | AddEventListenerOptions
  ): void {
    if (!el || typeof el.addEventListener !== 'function') return;
    el.addEventListener(type, callback as EventListener, options);
    this._domListeners.push({
      el,
      type,
      callback: callback as EventListener,
      options,
    });
  }

  /**
   * Registra un intervalo creado con setInterval.
   * Se cancelará con clearInterval automáticamente al descargar el componente.
   */
  registerInterval(id: number): number {
    this._intervals.push(id);
    return id;
  }

  /**
   * Registra una función de limpieza arbitraria ejecutada al descargar.
   */
  register(cb: () => any): void {
    if (typeof cb === 'function') {
      this._cleanups.push(cb);
    }
  }
}
