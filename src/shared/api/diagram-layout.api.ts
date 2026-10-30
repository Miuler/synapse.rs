import { invokeTauri } from './tauri-client';

/**
 * Proporción (porcentaje del ancho) del panel de código de un diagrama.
 * El backend la guarda solo en memoria; `null` indica que el usuario no la ha modificado.
 */
export const diagramLayoutApi = {
  async getSplitRatio(path: string): Promise<number | null> {
    try {
      return (await invokeTauri<number | null>('get_diagram_split_ratio', { path })) ?? null;
    } catch {
      return null;
    }
  },

  async setSplitRatio(path: string, ratio: number): Promise<void> {
    try {
      await invokeTauri('set_diagram_split_ratio', { path, ratio });
    } catch {
      // Sin backend (entorno web) la proporción solo vive en el componente
    }
  },
};
