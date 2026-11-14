/** Pregunta pendiente al usuario (permisos entre bóvedas, por ejemplo). */
export interface ConfirmAsk {
  id: number;
  title: string;
  message: string;
  resolve: (allowed: boolean) => void;
}

class ConfirmQueue {
  items = $state<ConfirmAsk[]>([]);
  #next = 1;

  /** Pregunta y espera la respuesta; las preguntas se muestran de a una, en orden. */
  ask(title: string, message: string): Promise<boolean> {
    return new Promise((resolve) => {
      this.items.push({ id: this.#next++, title, message, resolve });
    });
  }

  answer(allowed: boolean) {
    const [first, ...rest] = this.items;
    if (!first) return;
    this.items = rest;
    first.resolve(allowed);
  }
}

export const confirmQueue = new ConfirmQueue();
export const askConfirm = (title: string, message: string) => confirmQueue.ask(title, message);
