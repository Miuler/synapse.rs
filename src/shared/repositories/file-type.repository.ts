import { invokeTauri, isTauriEnvironment } from './tauri';
import type { SupportedFileTypes } from '@entities/file-type';

/**
 * Contrato de repositorio para el acceso a la configuración y metadatos
 * de tipos de archivo provistos por la fuente de datos.
 */
export interface FileTypeRepository {
  getSupportedFileTypes(): Promise<SupportedFileTypes | null>;
}

/**
 * Implementación de infraestructura del repositorio de tipos de archivo
 * basada en la comunicación IPC con el backend Tauri (Rust).
 */
export class TauriFileTypeRepository implements FileTypeRepository {
  async getSupportedFileTypes(): Promise<SupportedFileTypes | null> {
    if (!isTauriEnvironment()) {
      return null;
    }

    try {
      return await invokeTauri<SupportedFileTypes>('get_supported_file_types');
    } catch (error) {
      console.warn('Error en TauriFileTypeRepository al invocar get_supported_file_types:', error);
      return null;
    }
  }
}

export const tauriFileTypeRepository = new TauriFileTypeRepository();
