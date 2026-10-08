import { invokeTauri, isTauriEnvironment } from './tauri';
import { convertFileSrc } from '@tauri-apps/api/core';

export interface VaultNote {
  relative_path: string;
  abs_path?: string;
  title: string;
  content?: string;
  encoding?: string;
}

export interface VaultFileStat {
  ctime: number | null;
  mtime: number;
  size: number;
}

export interface VaultFile {
  path: string;
  name: string;
  basename: string;
  extension: string;
  parent?: string;
  stat: VaultFileStat;
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

export interface GitFileStatus {
  index?: string | null;
  worktree?: string | null;
  is_stashed?: boolean;
}

export type GitFileStatusKind = GitFileStatus;

export interface VaultGitStatus {
  is_repo: boolean;
  branch?: string | null;
  statuses: Record<string, GitFileStatus>;
}

export interface VaultEntryNode {
  name: string;
  relative_path: string;
  is_folder: boolean;
  title?: string;
}

export interface SearchResult {
  text: string;
  score: number;
  match_indices: number[];
  note_path?: string;
  is_recent?: boolean;
  last_opened_nanos?: number;
}

export interface QuickOpenSearchResult {
  results: SearchResult[];
  total_files: number;
  matched_files: number;
}

export interface OpenTabDto {
  path: string;
  view_mode?: string;
}

export interface WorkspaceOpenTabsState {
  open_tabs: OpenTabDto[];
  active_tab?: string | null;
}

export interface VaultUiState {
  sidebar_width?: number | null;
  expanded_folders: string[];
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
   * Guarda el estado de pestañas abiertas, orden, pestaña activa y modo de vista en DashMap / .synapse/cache.bin.
   */
  saveOpenTabsState(tabs: OpenTabDto[], activeTab?: string | null): Promise<void>;

  /**
   * Obtiene el estado restaurado de pestañas abiertas desde DashMap / .synapse/cache.bin.
   */
  getOpenTabsState(): Promise<WorkspaceOpenTabsState | null>;

  /**
   * Guarda el estado de la UI del explorador (ancho de panel lateral y carpetas expandidas) en .synapse/workspace.json.
   */
  saveVaultUiState(sidebarWidth?: number | null, expandedFolders?: string[]): Promise<void>;

  /**
   * Obtiene el estado de la UI del explorador (ancho de panel lateral y carpetas expandidas) desde .synapse/workspace.json.
   */
  getVaultUiState(): Promise<VaultUiState | null>;

  /**
   * Registra que una nota o archivo fue abierto para consulta/edición.
   */
  recordNoteOpened(relativePath: string): Promise<void>;

  /**
   * Obtiene la lista de rutas relativas de las notas abiertas recientemente en Synapse.
   */
  getRecentNotes(limit?: number): Promise<string[]>;

  /**
   * Obtiene la lista de notas/archivos presentes en la bóveda actual.
   */
  getNotes(): Promise<VaultNote[]>;

  /** Obtiene únicamente metadatos desde el índice DashMap, sin contenido. */
  getVaultFiles(): Promise<VaultFile[]>;

  /**
   * Obtiene los elementos directos (hijos) de una carpeta o de la raíz de la bóveda
   * consultando el caché en memoria de Rust de forma inmediata (sub-miligramo/sub-ms).
   */
  getDirectoryChildren(parentPath?: string): Promise<VaultEntryNode[]>;

  /**
   * Obtiene la cantidad total de archivos indexados en la bóveda actual desde la memoria de Rust.
   */
  getVaultFilesCount(): Promise<number>;

  /**
   * Realiza una búsqueda difusa interactiva utilizando el motor nucleo en Rust en memoria.
   */
  searchNotes(query: string): Promise<QuickOpenSearchResult>;

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
   * Consulta el estado de Git del repositorio asociado a la bóveda.
   */
  getGitStatus(folderPath?: string): Promise<VaultGitStatus | null>;

  /**
   * Ejecuta `git add` sobre una o más rutas dentro de la bóveda.
   */
  gitAdd(paths: string[]): Promise<void>;

  /**
   * Ejecuta `git restore` para descartar cambios en el área de trabajo.
   */
  gitRestore(paths: string[]): Promise<void>;

  /**
   * Ejecuta `git restore --staged` para desmarcar cambios del stage.
   */
  gitRestoreStaged(paths: string[]): Promise<void>;

  /**
   * Elimina un archivo o carpeta dentro de la bóveda.
   */
  deleteItem(relativePath: string): Promise<void>;

  /** Renombra un archivo o carpeta conservándolo en su carpeta actual. */
  renameItem(relativePath: string, newName: string): Promise<string>;

  /**
   * Copia (pega) archivos o carpetas dentro de `destDir`. Si el destino es la misma
   * carpeta de origen se agrega `.copy` al nombre. Retorna las rutas creadas.
   */
  copyItems(paths: string[], destDir: string): Promise<string[]>;

  /**
   * Fuerza la recarga en memoria y en DashMap de uno o más archivos o directorios
   * (incluyendo todos sus subdirectorios y metadatos).
   * Retorna las rutas relativas de los archivos afectados.
   */
  reloadVaultItems(paths: string[]): Promise<string[]>;

  /**
   * Resuelve la ruta relativa completa de un enlace o WikiLink ([[...]]) desde el DashMap de Rust.
   * Si no incluye extensión, se autocompleta con .md.
   */
  resolveVaultLink(link: string): Promise<string | null>;

  /**
   * Resuelve múltiples enlaces simultáneamente utilizando el DashMap de Rust.
   */
  resolveVaultLinks(links: string[]): Promise<Record<string, string>>;

  /**
   * Transforma contenido Markdown sustituyendo los WikiLinks ([[...]]) por enlaces estándar
   * con las rutas resueltas desde el DashMap en memoria de Rust.
   */
  renderMarkdownWikilinks(content: string): Promise<string>;

  /**
   * Resuelve una referencia de imagen o asset (por DashMap, relativa o absoluta) a su ruta absoluta en disco.
   */
  resolveAssetFilePath(src: string, baseFile?: string | null): Promise<string | null>;

  /**
   * Resuelve una ruta absoluta del sistema de archivos a una URL segura para el WebView.
   */
  resolveAssetUrl(path: string): string;

  /**
   * Lee el archivo de imagen como una Data URL (data:image/...;base64,...).
   * Compatible al 100% con WebView nativo, Tauri, Web y Android sin problemas de CORS ni protocolos bloqueados.
   */
  readAssetDataUrl(src: string, baseFile?: string | null): Promise<string | null>;
}

/**
 * Implementación de infraestructura del VaultRepository que interactúa
 * con el backend Rust mediante comandos IPC de Tauri y convertFileSrc.
 */
export class TauriVaultRepository implements VaultRepository {
  isConnected(): boolean {
    return isTauriEnvironment();
  }

  async saveOpenTabsState(tabs: OpenTabDto[], activeTab?: string | null): Promise<void> {
    if (!this.isConnected()) return;

    try {
      await invokeTauri('save_open_tabs_state', {
        tabs,
        activeTab: activeTab ?? null,
        active_tab: activeTab ?? null,
      });
    } catch (error) {
      console.warn('Error en TauriVaultRepository al guardar save_open_tabs_state:', error);
    }
  }

  async getOpenTabsState(): Promise<WorkspaceOpenTabsState | null> {
    if (!this.isConnected()) return null;

    try {
      return await invokeTauri<WorkspaceOpenTabsState>('get_open_tabs_state');
    } catch (error) {
      console.warn('Error en TauriVaultRepository al obtener get_open_tabs_state:', error);
      return null;
    }
  }

  async saveVaultUiState(sidebarWidth?: number | null, expandedFolders?: string[]): Promise<void> {
    if (!this.isConnected()) return;

    try {
      await invokeTauri('save_vault_ui_state', {
        sidebarWidth: sidebarWidth ?? null,
        sidebar_width: sidebarWidth ?? null,
        expandedFolders: expandedFolders ?? null,
        expanded_folders: expandedFolders ?? null,
      });
    } catch (error) {
      console.warn('Error en TauriVaultRepository al guardar save_vault_ui_state:', error);
    }
  }

  async getVaultUiState(): Promise<VaultUiState | null> {
    if (!this.isConnected()) return null;

    try {
      return await invokeTauri<VaultUiState>('get_vault_ui_state');
    } catch (error) {
      console.warn('Error en TauriVaultRepository al obtener get_vault_ui_state:', error);
      return null;
    }
  }

  async recordNoteOpened(relativePath: string): Promise<void> {
    if (!this.isConnected() || !relativePath || relativePath.startsWith('empty:')) {
      return;
    }

    try {
      await invokeTauri('record_note_opened', {
        relativePath,
        relative_path: relativePath,
      });
    } catch (error) {
      console.warn('Error en TauriVaultRepository al registrar record_note_opened:', error);
    }
  }

  async getRecentNotes(limit = 15): Promise<string[]> {
    if (!this.isConnected()) {
      return [];
    }

    try {
      const results = await invokeTauri<string[]>('get_recent_notes_command', {
        limit,
      });
      return Array.isArray(results) ? results : [];
    } catch (error) {
      console.warn('Error en TauriVaultRepository al obtener get_recent_notes:', error);
      return [];
    }
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

  async getVaultFiles(): Promise<VaultFile[]> {
    if (!this.isConnected()) return [];

    try {
      const files = await invokeTauri<VaultFile[]>('get_vault_files');
      return Array.isArray(files) ? files : [];
    } catch (error) {
      console.warn('Error en TauriVaultRepository al obtener get_vault_files:', error);
      return [];
    }
  }

  async getDirectoryChildren(parentPath?: string): Promise<VaultEntryNode[]> {
    if (!this.isConnected()) {
      return [];
    }

    try {
      const children = await invokeTauri<VaultEntryNode[]>('get_vault_directory_children', {
        parentPath: parentPath ?? null,
        parent_path: parentPath ?? null,
      });
      return Array.isArray(children) ? children : [];
    } catch (error) {
      console.warn('Error en TauriVaultRepository al obtener get_vault_directory_children:', error);
      return [];
    }
  }

  async getVaultFilesCount(): Promise<number> {
    if (!this.isConnected()) return 0;
    try {
      const count = await invokeTauri<number>('get_vault_files_count');
      return typeof count === 'number' ? count : 0;
    } catch {
      return 0;
    }
  }

  async searchNotes(query: string): Promise<QuickOpenSearchResult> {
    if (!this.isConnected()) {
      return { results: [], total_files: 0, matched_files: 0 };
    }

    try {
      const resp = await invokeTauri<QuickOpenSearchResult>('search_notes_command', {
        query,
      });
      return resp || { results: [], total_files: 0, matched_files: 0 };
    } catch (error) {
      console.warn('Error en TauriVaultRepository al buscar notas con nucleo:', error);
      return { results: [], total_files: 0, matched_files: 0 };
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

  async getGitStatus(folderPath?: string): Promise<VaultGitStatus | null> {
    if (!this.isConnected()) {
      return null;
    }

    try {
      return await invokeTauri<VaultGitStatus>('get_vault_git_status', {
        folderPath: folderPath ?? null,
        folder_path: folderPath ?? null,
      });
    } catch (error) {
      console.warn('Error en TauriVaultRepository al obtener get_vault_git_status:', error);
      return null;
    }
  }

  async gitAdd(paths: string[]): Promise<void> {
    if (!this.isConnected() || paths.length === 0) {
      return;
    }

    await invokeTauri('git_add_paths', { paths });
  }

  async gitRestore(paths: string[]): Promise<void> {
    if (!this.isConnected() || paths.length === 0) {
      return;
    }

    await invokeTauri('git_restore_paths', { paths });
  }

  async gitRestoreStaged(paths: string[]): Promise<void> {
    if (!this.isConnected() || paths.length === 0) {
      return;
    }

    await invokeTauri('git_restore_staged_paths', { paths });
  }

  async deleteItem(relativePath: string): Promise<void> {
    if (!this.isConnected()) {
      return;
    }

    await invokeTauri('delete_vault_item', {
      relativePath,
      relative_path: relativePath,
    });
  }

  async renameItem(relativePath: string, newName: string): Promise<string> {
    if (!this.isConnected()) {
      throw new Error('No hay conexión con la bóveda');
    }

    return await invokeTauri<string>('rename_vault_item', {
      relativePath,
      relative_path: relativePath,
      newName,
      new_name: newName,
    });
  }

  async copyItems(paths: string[], destDir: string): Promise<string[]> {
    if (!this.isConnected() || paths.length === 0) {
      return [];
    }

    const created = await invokeTauri<string[]>('copy_vault_items', {
      paths,
      destDir,
      dest_dir: destDir,
    });
    return Array.isArray(created) ? created : [];
  }

  async reloadVaultItems(paths: string[]): Promise<string[]> {
    if (!this.isConnected()) {
      return [];
    }

    try {
      const reloaded = await invokeTauri<string[]>('reload_vault_items', {
        paths,
      });
      return Array.isArray(reloaded) ? reloaded : [];
    } catch (error) {
      console.warn('Error en TauriVaultRepository al recargar elementos de la bóveda:', error);
      return [];
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

  async resolveVaultLink(link: string): Promise<string | null> {
    if (!this.isConnected() || !link) return null;
    try {
      return await invokeTauri<string | null>('resolve_vault_link', { link });
    } catch (error) {
      console.warn('Error al resolver enlace de bóveda:', error);
      return null;
    }
  }

  async resolveVaultLinks(links: string[]): Promise<Record<string, string>> {
    if (!this.isConnected() || !links || links.length === 0) return {};
    try {
      const res = await invokeTauri<Record<string, string>>('resolve_vault_links', { links });
      return res || {};
    } catch (error) {
      console.warn('Error al resolver enlaces de bóveda:', error);
      return {};
    }
  }

  async resolveAssetFilePath(src: string, baseFile?: string | null): Promise<string | null> {
    if (!this.isConnected() || !src) return null;
    try {
      const resolved = await invokeTauri<string | null>('resolve_asset_file_path', {
        src,
        baseFile: baseFile ?? null,
        base_file: baseFile ?? null,
      });
      if (!resolved) {
        console.error('El backend no pudo resolver la ruta del asset.', { src, baseFile });
      }
      return resolved;
    } catch (error) {
      console.error('Error al resolver ruta de asset en bóveda:', { src, baseFile, error });
      return null;
    }
  }

  async readAssetDataUrl(src: string, baseFile?: string | null): Promise<string | null> {
    if (!this.isConnected() || !src) return null;
    try {
      return await invokeTauri<string | null>('read_asset_data_url', {
        src,
        baseFile: baseFile ?? null,
        base_file: baseFile ?? null,
      });
    } catch (error) {
      console.error('Error al leer asset data URL:', { src, baseFile, error });
      return null;
    }
  }

  async renderMarkdownWikilinks(content: string): Promise<string> {
    if (!this.isConnected() || !content) return content;
    try {
      const res = await invokeTauri<string>('render_markdown_wikilinks', { content });
      return typeof res === 'string' ? res : content;
    } catch (error) {
      console.warn('Error al renderizar wikilinks en markdown:', error);
      return content;
    }
  }
}

export const vaultRepository: VaultRepository = new TauriVaultRepository();
