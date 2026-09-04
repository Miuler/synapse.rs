import { tauriFileTypeRepository, type FileTypeRepository } from '@shared/repositories';
import { fileTypesManager } from '@entities/file-type';

/**
 * Caso de uso: Cargar los tipos de archivo soportados desde el repositorio
 * y actualizar el estado de la entidad FileTypesManager.
 */
export async function loadSupportedFileTypesUseCase(
  repository: FileTypeRepository = tauriFileTypeRepository
): Promise<void> {
  const types = await repository.getSupportedFileTypes();
  if (types && Array.isArray(types.images)) {
    fileTypesManager.setSupportedFileTypes(types);
  }
}
