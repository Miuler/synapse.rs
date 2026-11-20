// Imágenes del chat del agente (pegadas por el usuario o devueltas por el agente) que se abren en una
// pestaña con el visor de imágenes. No son archivos de la bóveda: viven en memoria mientras dure la sesión.
import { SvelteMap } from 'svelte/reactivity';

export interface AgentImage {
  name: string;
  mimeType: string;
  /** base64 sin prefijo */
  data: string;
}

export const AGENT_IMAGE_TAB_PREFIX = 'synapse://agent-image/';

class AgentImages {
  readonly items = new SvelteMap<string, AgentImage>();
  /** Pestaña que el espacio de trabajo debe abrir (la consume y la deja en `null`) */
  request = $state<string | null>(null);
  #next = 1;

  /** Guarda la imagen (una sola vez aunque se abra varias veces) y pide abrir su pestaña */
  open(img: AgentImage): string {
    let id = [...this.items].find(([, v]) => v.data === img.data)?.[0];
    if (!id) {
      id = String(this.#next++);
      this.items.set(id, { ...img });
    }
    const tab = `${AGENT_IMAGE_TAB_PREFIX}${id}`;
    this.request = tab;
    return tab;
  }

  /** Imagen de una pestaña `synapse://agent-image/<id>` */
  get(tab: string): AgentImage | undefined {
    return tab.startsWith(AGENT_IMAGE_TAB_PREFIX) ? this.items.get(tab.slice(AGENT_IMAGE_TAB_PREFIX.length)) : undefined;
  }
}

export const agentImages = new AgentImages();
