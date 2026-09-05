import { invokeTauri, isTauriEnvironment } from './tauri';
import { convertFileSrc } from '@tauri-apps/api/core';

export interface VaultNote {
  relative_path: string;
  abs_path?: string;
  title: string;
  content?: string;
  encoding?: string;
}

export interface SaveNoteParams {
  relativePath: string;
  title: string;
  content: string;
  encoding?: string;
}

export interface SelectVaultFolderResult {
  folder_path: string;
  notes: VaultNote[];
}

/**
 * Contrato de repositorio para el acceso y manipulación de archivos
 * y notas dentro de las carpetas que representan una bóveda (Vault).
 */
export interface VaultRepository {
  /**
   * Indica si la fuente de datos / backend subyacente está disponible.
   */
  isConnected(): boolean;

  /**
   * Obtiene la lista de notas/archivos presentes en la bóveda actual.
   */
  getNotes(): Promise<VaultNote[]>;

  /**
   * Obtiene la ruta absoluta de la carpeta de la bóveda activa.
   */
  getActiveVaultPath(): Promise<string | null>;

  /**
   * Establece la ruta absoluta de la carpeta de la bóveda activa.
   */
  setActiveVaultPath(path: string): Promise<void>;

  /**
   * Lee el contenido completo y metadatos de un archivo específico de la bóveda.
   */
  readNote(relativePath: string): Promise<VaultNote | null>;

  /**
   * Guarda o persiste el contenido de un archivo en la bóveda.
   */
  saveNote(params: SaveNoteParams): Promise<void>;

  /**
   * Abre un diálogo de selección para abrir una nueva carpeta de bóveda.
   * Opcionalmente inicia en la ruta especificada por startingDirectory.
   */
  selectVaultFolder(startingDirectory?: string): Promise<SelectVaultFolderResult | null>;

  /**
   * Resuelve una ruta absoluta del sistema de archivos a una URL segura para el WebView.
   */
  resolveAssetUrl(path: string): string;
}

/**
 * Implementación de infraestructura del VaultRepository que interactúa
 * con el backend Rust mediante comandos IPC de Tauri y convertFileSrc.
 */
export class TauriVaultRepository implements VaultRepository {
  isConnected(): boolean {
    return isTauriEnvironment();
  }

  async getNotes(): Promise<VaultNote[]> {
    if (!this.isConnected()) {
      return [];
    }

    try {
      const notes = await invokeTauri<VaultNote[]>('get_vault_notes');
      return Array.isArray(notes) ? notes : [];
    } catch (error) {
      console.warn('Error en TauriVaultRepository al obtener get_vault_notes:', error);
      return [];
    }
  }

  async getActiveVaultPath(): Promise<string | null> {
    if (!this.isConnected()) {
      return null;
    }

    try {
      return await invokeTauri<string>('get_active_vault_path');
    } catch (error) {
      console.warn('Error en TauriVaultRepository al obtener get_active_vault_path:', error);
      return null;
    }
  }

  async setActiveVaultPath(path: string): Promise<void> {
    if (!this.isConnected()) {
      return;
    }

    try {
      await invokeTauri('set_active_vault_path', {
        newPath: path,
        new_path: path,
      });
    } catch (error) {
      console.error('Error en TauriVaultRepository al cambiar set_active_vault_path:', error);
    }
  }

  async readNote(relativePath: string): Promise<VaultNote | null> {
    if (!this.isConnected()) {
      return null;
    }

    try {
      return await invokeTauri<VaultNote>('read_note_content', {
        relativePath,
        relative_path: relativePath,
      });
    } catch (error) {
      console.error(`Error en TauriVaultRepository al leer ${relativePath}:`, error);
      return null;
    }
  }

  async saveNote(params: SaveNoteParams): Promise<void> {
    if (!this.isConnected()) {
      return;
    }

    await invokeTauri('save_note_content', {
      relativePath: params.relativePath,
      relative_path: params.relativePath,
      title: params.title,
      content: params.content,
      encoding: params.encoding,
    });
  }

  async selectVaultFolder(startingDirectory?: string): Promise<SelectVaultFolderResult | null> {
    if (!this.isConnected()) {
      return null;
    }

    try {
      return await invokeTauri<SelectVaultFolderResult | null>('select_vault_folder', {
        startingDirectory: startingDirectory ?? null,
        starting_directory: startingDirectory ?? null,
      });
    } catch (error) {
      console.error('Error en TauriVaultRepository al seleccionar carpeta de bóveda:', error);
      return null;
    }
  }

  resolveAssetUrl(path: string): string {
    if (!path) return '';
    try {
      return convertFileSrc(path);
    } catch {
      return path;
    }
  }
}

export const vaultRepository: VaultRepository = new TauriVaultRepository();
