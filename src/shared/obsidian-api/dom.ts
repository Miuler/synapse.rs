/**
 * Utilidades del DOM compatibles con Obsidian.
 * Obsidian enriquece Node y HTMLElement con métodos de conveniencia como
 * `.createDiv()`, `.createSpan()`, `.createEl()` y `.empty()`.
 */

export interface DomElementOptions {
  cls?: string | string[];
  text?: string;
  attr?: Record<string, string | number | boolean>;
  title?: string;
  parent?: HTMLElement | Node;
}

/**
 * Crea un elemento HTML con clases, texto y atributos opcionales.
 * Si se ejecuta en un entorno sin DOM nativo (ej. pruebas aisladas), provee un mock ligero.
 */
export function createEl<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  options?: DomElementOptions
): HTMLElementTagNameMap[K] {
  if (typeof document !== 'undefined' && typeof document.createElement === 'function' && !(document as any).__isSynapseMock) {
    const el = document.createElement(tag);
    if (options?.cls) {
      const classes = Array.isArray(options.cls) ? options.cls : options.cls.split(/\s+/).filter(Boolean);
      el.classList.add(...classes);
    }
    if (options?.text !== undefined) {
      el.textContent = options.text;
    }
    if (options?.title) {
      el.title = options.title;
    }
    if (options?.attr) {
      for (const [k, v] of Object.entries(options.attr)) {
        el.setAttribute(k, String(v));
      }
    }
    if (options?.parent) {
      options.parent.appendChild(el);
    }
    return el;
  }

  // Mock funcional para entornos de prueba sin DOM nativo
  const children: any[] = [];
  const classListSet = new Set<string>();
  const attributes = new Map<string, string>();
  const eventListeners = new Map<string, Function[]>();

  if (options?.cls) {
    const classes = Array.isArray(options.cls) ? options.cls : options.cls.split(/\s+/).filter(Boolean);
    for (const c of classes) classListSet.add(c);
  }

  if (options?.attr) {
    for (const [k, v] of Object.entries(options.attr)) {
      attributes.set(k, String(v));
    }
  }

  const mock: any = {
    tagName: tag.toUpperCase(),
    className: Array.from(classListSet).join(' '),
    title: options?.title || '',
    textContent: options?.text || '',
    value: options?.attr?.value !== undefined ? String(options.attr.value) : '',
    innerHTML: '',
    style: {},
    children,
    classList: {
      add: (...classes: string[]) => {
        for (const c of classes) classListSet.add(c);
        mock.className = Array.from(classListSet).join(' ');
      },
      remove: (...classes: string[]) => {
        for (const c of classes) classListSet.delete(c);
        mock.className = Array.from(classListSet).join(' ');
      },
      contains: (c: string) => classListSet.has(c),
      toggle: (c: string) => {
        if (classListSet.has(c)) {
          classListSet.delete(c);
          mock.className = Array.from(classListSet).join(' ');
          return false;
        }
        classListSet.add(c);
        mock.className = Array.from(classListSet).join(' ');
        return true;
      },
    },
    appendChild: (child: any) => {
      children.push(child);
      return child;
    },
    removeChild: (child: any) => {
      const idx = children.indexOf(child);
      if (idx !== -1) children.splice(idx, 1);
      return child;
    },
    remove: () => {},
    addEventListener: (type: string, cb: Function) => {
      let list = eventListeners.get(type);
      if (!list) {
        list = [];
        eventListeners.set(type, list);
      }
      list.push(cb);
    },
    removeEventListener: (type: string, cb: Function) => {
      const list = eventListeners.get(type);
      if (list) {
        eventListeners.set(type, list.filter((fn) => fn !== cb));
      }
    },
    dispatchEvent: (ev: any) => {
      const type = ev?.type || String(ev);
      const list = eventListeners.get(type);
      if (list) {
        for (const fn of list) {
          fn.call(mock, ev);
        }
      }
      return true;
    },
    setAttribute: (name: string, val: any) => {
      attributes.set(name, String(val));
    },
    getAttribute: (name: string) => {
      return attributes.get(name) ?? null;
    },
    querySelector: () => null,
    querySelectorAll: () => [],
    addClass: (c: string) => {
      classListSet.add(c);
      mock.className = Array.from(classListSet).join(' ');
    },
    removeClass: (c: string) => {
      classListSet.delete(c);
      mock.className = Array.from(classListSet).join(' ');
    },
    hasClass: (c: string) => classListSet.has(c),
    toggleClass: (c: string) => {
      if (classListSet.has(c)) {
        classListSet.delete(c);
      } else {
        classListSet.add(c);
      }
      mock.className = Array.from(classListSet).join(' ');
    },
    setText: (t: string) => {
      mock.textContent = String(t);
      return mock;
    },
    getText: () => mock.textContent,
    createDiv: (o?: DomElementOptions) => createDiv(o, mock),
    createSpan: (o?: DomElementOptions) => createSpan(o, mock),
    createEl: (t: any, o?: DomElementOptions) => createEl(t, { ...o, parent: mock }),
    empty: () => {
      children.length = 0;
      mock.textContent = '';
      mock.innerHTML = '';
    },
  };

  if (options?.parent && typeof (options.parent as any).appendChild === 'function') {
    (options.parent as any).appendChild(mock);
  }

  return mock as HTMLElementTagNameMap[K];
}

export function createDiv(options?: DomElementOptions, parent?: HTMLElement | Node): HTMLDivElement {
  return createEl('div', { ...options, parent: parent || options?.parent });
}

export function createSpan(options?: DomElementOptions, parent?: HTMLElement | Node): HTMLSpanElement {
  return createEl('span', { ...options, parent: parent || options?.parent });
}

export function empty(node: Node | HTMLElement): void {
  if (typeof (node as any)?.empty === 'function') {
    (node as any).empty();
  } else if (node) {
    while (node.firstChild) {
      node.removeChild(node.firstChild);
    }
  }
}

/**
 * Extiende los prototipos de Node y HTMLElement si estamos en un entorno con DOM activo,
 * tal y como hace Obsidian internamente.
 */
export function setupObsidianDomExtensions(): void {
  if (typeof window === 'undefined' || typeof HTMLElement === 'undefined') return;

  const proto = HTMLElement.prototype as any;
  if (!proto.createEl) {
    proto.createEl = function (tag: any, options?: DomElementOptions) {
      return createEl(tag, { ...options, parent: this });
    };
  }
  if (!proto.createDiv) {
    proto.createDiv = function (options?: DomElementOptions) {
      return createDiv(options, this);
    };
  }
  if (!proto.createSpan) {
    proto.createSpan = function (options?: DomElementOptions) {
      return createSpan(options, this);
    };
  }
  if (!proto.setText) {
    proto.setText = function (val: any) {
      this.textContent = val !== undefined && val !== null ? String(val) : '';
      return this;
    };
  }
  if (!proto.getText) {
    proto.getText = function () {
      return this.textContent || '';
    };
  }
  if (!proto.addClass) {
    proto.addClass = function (...cls: string[]) {
      for (const c of cls) {
        if (c) this.classList.add(...c.split(/\s+/).filter(Boolean));
      }
      return this;
    };
  }
  if (!proto.removeClass) {
    proto.removeClass = function (...cls: string[]) {
      for (const c of cls) {
        if (c) this.classList.remove(...c.split(/\s+/).filter(Boolean));
      }
      return this;
    };
  }
  if (!proto.removeClasses) {
    proto.removeClasses = function (cls: string[] | string) {
      const arr = Array.isArray(cls) ? cls : [cls];
      for (const c of arr) {
        if (c) this.classList.remove(...c.split(/\s+/).filter(Boolean));
      }
      return this;
    };
  }
  if (!proto.addClasses) {
    proto.addClasses = function (cls: string[] | string) {
      const arr = Array.isArray(cls) ? cls : [cls];
      for (const c of arr) {
        if (c) this.classList.add(...c.split(/\s+/).filter(Boolean));
      }
      return this;
    };
  }
  if (!proto.toggleClass) {
    proto.toggleClass = function (cls: string, force?: boolean) {
      return this.classList.toggle(cls, force);
    };
  }
  if (!proto.hasClass) {
    proto.hasClass = function (cls: string) {
      return this.classList.contains(cls);
    };
  }
  if (!proto.setAttr) {
    proto.setAttr = function (qualifiedName: string, value: any) {
      this.setAttribute(qualifiedName, String(value));
      return this;
    };
  }
  if (!proto.getAttr) {
    proto.getAttr = function (qualifiedName: string) {
      return this.getAttribute(qualifiedName);
    };
  }
  if (!proto.empty) {
    proto.empty = function () {
      while (this.firstChild) {
        this.removeChild(this.firstChild);
      }
    };
  }
}

// Ejecutar automáticamente si el entorno tiene DOM
setupObsidianDomExtensions();
