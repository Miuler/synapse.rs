import { tauriFileTypeRepository, type FileTypeRepository } from './file-type.repository';
import { fileTypesManager } from '../model/file-types.svelte';

/**
 * Cargar los tipos de archivo soportados desde el repositorio
 * y actualizar el estado de la entidad FileTypesManager.
 */
export async function loadSupportedFileTypes(
  repository: FileTypeRepository = tauriFileTypeRepository
): Promise<void> {
  const types = await repository.getSupportedFileTypes();
  if (types && Array.isArray(types.images)) {
    fileTypesManager.setSupportedFileTypes(types);
  }
}

export const loadSupportedFileTypesUseCase = loadSupportedFileTypes;
