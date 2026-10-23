/**
 * Bus canónico de eventos compatible con la clase Events de Obsidian.
 * Proporciona registro, cancelación por función o por EventRef, y disparo síncrono.
 */

export interface EventRef {
  /** Referencia a la instancia de Events emisora */
  e?: Events;
  /** Nombre del evento registrado */
  name?: string;
  /** Función callback registrada */
  fn?: (...data: any[]) => any;
  /** Contexto `this` con el que se invoca la función */
  ctx?: any;
}

interface EventListenerRecord {
  fn: (...data: any[]) => any;
  ctx?: any;
}

export class Events {
  private _events: Map<string, EventListenerRecord[]> = new Map();

  /**
   * Registra un callback para un evento específico.
   * Retorna un EventRef que puede usarse para desuscribirse con `offref` o `Plugin.prototype.registerEvent`.
   */
  on(name: string, callback: (...data: any[]) => any, ctx?: any): EventRef {
    let list = this._events.get(name);
    if (!list) {
      list = [];
      this._events.set(name, list);
    }

    list.push({ fn: callback, ctx });

    const ref: EventRef = {
      e: this,
      name,
      fn: callback,
      ctx,
    };

    return ref;
  }

  /**
   * Elimina un callback registrado para un evento específico.
   */
  off(name: string, callback: (...data: any[]) => any): void {
    const list = this._events.get(name);
    if (!list) return;

    const filtered = list.filter((item) => item.fn !== callback);
    if (filtered.length === 0) {
      this._events.delete(name);
    } else {
      this._events.set(name, filtered);
    }
  }

  /**
   * Elimina un registro de evento utilizando la referencia EventRef devuelta por `on`.
   */
  offref(ref: EventRef): void {
    if (!ref) return;
    if (ref.e && ref.e !== this) {
      ref.e.offref(ref);
      return;
    }
    if (ref.name && ref.fn) {
      const list = this._events.get(ref.name);
      if (!list) return;

      const filtered = list.filter(
        (item) => item.fn !== ref.fn || (ref.ctx !== undefined && item.ctx !== ref.ctx)
      );

      if (filtered.length === 0) {
        this._events.delete(ref.name);
      } else {
        this._events.set(ref.name, filtered);
      }
    }
  }

  /**
   * Dispara un evento por nombre con los argumentos proporcionados a todos los escuchadores.
   */
  trigger(name: string, ...data: any[]): void {
    const list = this._events.get(name);
    if (!list || list.length === 0) return;

    // Clonamos la lista para evitar problemas si algún callback añade o remueve listeners durante la ejecución
    const listeners = list.slice();
    for (const listener of listeners) {
      try {
        if (listener.ctx !== undefined) {
          listener.fn.apply(listener.ctx, data);
        } else {
          listener.fn(...data);
        }
      } catch (err) {
        console.error(`[Synapse Events] Error al ejecutar listener de "${name}":`, err);
      }
    }
  }

  /**
   * Intenta disparar de manera segura un EventRef individual con los argumentos dados.
   */
  tryTrigger(evt: EventRef, args: any[]): void {
    if (!evt || typeof evt.fn !== 'function') return;
    try {
      if (evt.ctx !== undefined) {
        evt.fn.apply(evt.ctx, args);
      } else {
        evt.fn(...args);
      }
    } catch (err) {
      console.error(`[Synapse Events] Error en tryTrigger:`, err);
    }
  }
}
