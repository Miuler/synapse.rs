import { invokeTauri, isTauriEnvironment } from './tauri';
import { convertFileSrc } from '@tauri-apps/api/core';

export interface VaultNote {
  relative_path: string;
  abs_path?: string;
  title: string;
  content?: string;
  encoding?: string;
  view_mode?: string;
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
  view_mode?: string;
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

export type StartupTarget =
  | { type: 'standalone'; path: string }
  | { type: 'vault'; path: string }
  | null;

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

export interface GitDiffResponse {
  path: string;
  is_staged: boolean;
  old_file_name: string;
  new_file_name: string;
  old_content: string;
  new_content: string;
  diff: string;
}

export interface GitFileRevisionResult {
  path: string;
  revision: string;
  content: string | null;
  data_url: string | null;
  is_binary: boolean;
  exists: boolean;
}

export interface GitImageDiffResult {
  path: string;
  is_staged: boolean;
  diff_data_url: string | null;
  diff_count: number;
  width: number;
  height: number;
  total_pixels: number;
  diff_percentage: number;
  has_differences: boolean;
}

export interface GitBranchItem {
  name: string;
  ref_name: string;
  is_current: boolean;
  is_remote: boolean;
  upstream?: string | null;
  last_commit_date?: string | null;
  last_commit_message?: string | null;
}

export interface GitBranchesResult {
  current_branch?: string | null;
  local_branches: GitBranchItem[];
  remote_branches: GitBranchItem[];
}

export interface GitCommitLogItem {
  commit_hash: string;
  short_hash: string;
  author_name: string;
  author_email: string;
  date_iso: string;
  timestamp: number;
  message: string;
}

export interface GitFileLogResult {
  path: string;
  commits: GitCommitLogItem[];
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
  diff_visual_mode?: string | null;
  diff_text_mode?: string | null;
}

export interface RecentWorkspaceEntry {
  path: string;
  display_path: string;
  name: string;
  last_opened_ms: number;
  pinned: boolean;
  notes_count: number;
  git_branch?: string | null;
  is_git_repo: boolean;
  exists: boolean;
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
   * Guarda el modo de vista (edición/lectura) para una nota específica en el workspace.
   */
  setNoteViewMode(relativePath: string, viewMode: string): Promise<void>;

  /**
   * Guarda el estado de la UI del explorador (ancho de panel lateral, carpetas expandidas y preferencias de diff) en .synapse/workspace.json.
   */
  saveVaultUiState(
    sidebarWidth?: number | null,
    expandedFolders?: string[],
    diffVisualMode?: string | null,
    diffTextMode?: string | null,
  ): Promise<void>;

  /**
   * Obtiene el estado de la UI del explorador (ancho de panel lateral y carpetas expandidas) desde .synapse/workspace.json.
   */
  getVaultUiState(): Promise<VaultUiState | null>;

  /**
   * Abre una pestaña o archivo en una nueva ventana independiente de Tauri.
   */
  openTabInNewWindow(path: string, title?: string): Promise<void>;

  /**
   * Obtiene la ruta del archivo asignado a esta ventana si es una ventana flotante secundaria.
   */
  getDetachedWindowTab(): Promise<string | null>;

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
   * Obtiene los metadatos de un archivo específico en tiempo O(1) directamente desde el índice DashMap de Rust.
   */
  getNoteMeta(relativePath: string): Promise<VaultFile | null>;

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
   * Obtiene el objetivo de arranque inicial (archivo puntual o bóveda) si Synapse fue lanzado con argumentos.
   */
  getStartupTarget(): Promise<StartupTarget>;

  /**
   * Abre un selector nativo de archivos para abrir un archivo puntual (.md, .mermaid, .excalidraw) sin espacio de trabajo.
   */
  selectStandaloneFile(): Promise<string | null>;

  /**
   * Consulta el estado de Git del repositorio asociado a la bóveda.
   */
  getGitStatus(folderPath?: string): Promise<VaultGitStatus | null>;

  /**
   * Ejecuta `git add` sobre una o más rutas dentro de la bóveda.
   */
  gitAdd(paths: string[]): Promise<void>;

  /**
   * Ejecuta `git commit` sobre los archivos seleccionados con el mensaje especificado.
   * Retorna el mensaje de confirmación o salida de Git.
   */
  gitCommit(paths: string[], message: string): Promise<string>;

  /**
   * Ejecuta `git restore` para descartar cambios en el área de trabajo.
   */
  gitRestore(paths: string[]): Promise<void>;

  /**
   * Ejecuta `git restore --staged` para desmarcar cambios del stage.
   */
  gitRestoreStaged(paths: string[]): Promise<void>;

  /**
   * Obtiene el diff (unificado) de un archivo respecto al índice o a HEAD (o una revisión específica).
   */
  getGitFileDiff(path: string, staged: boolean, revision?: string): Promise<GitDiffResponse>;

  /**
   * Obtiene el contenido o data_url de un archivo en una revisión histórica (HEAD, Staged o commit específico).
   */
  getGitFileRevision(path: string, staged: boolean, revision?: string): Promise<GitFileRevisionResult>;

  /**
   * Obtiene el diff perceptual en memoria de una imagen (Disco vs Git).
   */
  getGitImageDiff(path: string, staged: boolean, revision?: string): Promise<GitImageDiffResult>;

  /**
   * Obtiene el historial de commits de un archivo.
   */
  getGitFileLog(path: string, maxCount?: number): Promise<GitFileLogResult>;

  /**
   * Obtiene la lista de ramas locales y remotas del repositorio Git de la bóveda.
   */
  getGitBranches(): Promise<GitBranchesResult>;

  /**
   * Cambia de rama en el repositorio Git de la bóveda (git checkout).
   */
  gitCheckout(branchName: string): Promise<string>;

  /**
   * Crea una nueva rama en el repositorio Git de la bóveda.
   */
  gitCreateBranch(params: {
    newBranch: string;
    baseBranch?: string;
    checkout?: boolean;
  }): Promise<string>;

  /**
   * Elimina un archivo o carpeta dentro de la bóveda.
   */
  deleteItem(relativePath: string): Promise<void>;

  /**
   * Abre un archivo usando la aplicación predeterminada del sistema operativo (Word, Excel, etc.).
   */
  openFileWithDefaultApp(relativePath: string): Promise<void>;

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
   * Si no incluye extensión, se autocompleta con .md. Si existe más de un archivo con el mismo
   * nombre, prioriza el que esté en la misma carpeta o subcarpeta más cercana a `baseFile`.
   */
  resolveVaultLink(link: string, baseFile?: string | null): Promise<string | null>;

  /**
   * Resuelve múltiples enlaces simultáneamente utilizando el DashMap de Rust, priorizando
   * archivos en la misma carpeta o subcarpetas de `baseFile`.
   */
  resolveVaultLinks(links: string[], baseFile?: string | null): Promise<Record<string, string>>;

  /**
   * Transforma contenido Markdown sustituyendo los WikiLinks ([[...]]) por enlaces estándar
   * con las rutas resueltas desde el DashMap en memoria de Rust, con proximidad al archivo actual `baseFile`.
   */
  renderMarkdownWikilinks(content: string, baseFile?: string | null): Promise<string>;

  /**
   * Resuelve una referencia de imagen o asset (por DashMap, relativa o absoluta) a su ruta absoluta en disco.
   */
  resolveAssetFilePath(src: string, baseFile?: string | null): Promise<string | null>;

  /**
   * Resuelve una ruta absoluta del sistema de archivos a una URL segura para el WebView.
   */
  resolveAssetUrl(path: string): string;

  /**
   * Lee el archivo de imagen o documento como una Data URL (data:...;base64,...).
   * Compatible al 100% con WebView nativo, Tauri, Web y Android sin problemas de CORS ni protocolos bloqueados.
   */
  readAssetDataUrl(src: string, baseFile?: string | null): Promise<string | null>;

  /**
   * Guarda un archivo binario en la bóveda codificado en Base64 (útil para PDFs exportados con anotaciones, etc.).
   */
  saveBinaryAsset(relativePath: string, base64Data: string): Promise<void>;

  /**
   * Lee el contenido binario crudo de un archivo dentro de la bóveda como ArrayBuffer.
   */
  readBinary(relativePath: string): Promise<ArrayBuffer>;

  /**
   * Escribe contenido binario crudo directamente en un archivo de la bóveda sin sobrecoste de Base64.
   */
  writeBinary(relativePath: string, data: ArrayBuffer | Uint8Array): Promise<void>;

  /**
   * Crea una carpeta y sus subcarpetas dentro de la bóveda.
   */
  createFolder(relativePath: string): Promise<void>;

  /**
   * Suscribe a los eventos de cambio de archivos emitidos por el watcher de la bóveda (Rust notify).
   * Retorna una función para cancelar la suscripción.
   */
  onFilesChanged?(callback: (payload: { paths: string[]; deleted: string[] }) => void): Promise<() => void> | (() => void);

  /**
   * Obtiene la lista de workspaces recientes desde ~/.config/synapsers/workspaces.json.
   */
  getRecentWorkspaces(): Promise<RecentWorkspaceEntry[]>;

  /**
   * Fija o desfija un workspace en la lista de recientes.
   */
  togglePinWorkspace(path: string): Promise<RecentWorkspaceEntry[]>;

  /**
   * Elimina un workspace del registro de recientes.
   */
  removeRecentWorkspace(path: string): Promise<RecentWorkspaceEntry[]>;

  /**
   * Crea o inicializa un nuevo directorio como workspace de Synapse sin advertencia.
   */
  createNewWorkspace(startingDirectory?: string): Promise<SelectVaultFolderResult | null>;

  /**
   * Clona un repositorio Git e inicializa un nuevo workspace de Synapse.
   */
  cloneGitWorkspace(repoUrl: string, startingDirectory?: string): Promise<SelectVaultFolderResult | null>;

  /**
   * Abre un workspace en una nueva ventana independiente de Tauri.
   */
  openWorkspaceInNewWindow(path: string): Promise<void>;
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

  async setNoteViewMode(relativePath: string, viewMode: string): Promise<void> {
    if (!this.isConnected()) return;

    try {
      await invokeTauri('set_note_view_mode', {
        relativePath,
        relative_path: relativePath,
        viewMode,
        view_mode: viewMode,
      });
    } catch (error) {
      console.warn('Error en TauriVaultRepository al guardar set_note_view_mode:', error);
    }
  }

  async saveVaultUiState(
    sidebarWidth?: number | null,
    expandedFolders?: string[],
    diffVisualMode?: string | null,
    diffTextMode?: string | null,
  ): Promise<void> {
    if (!this.isConnected()) return;

    try {
      await invokeTauri('save_vault_ui_state', {
        sidebarWidth: sidebarWidth ?? null,
        sidebar_width: sidebarWidth ?? null,
        expandedFolders: expandedFolders ?? null,
        expanded_folders: expandedFolders ?? null,
        diffVisualMode: diffVisualMode ?? null,
        diff_visual_mode: diffVisualMode ?? null,
        diffTextMode: diffTextMode ?? null,
        diff_text_mode: diffTextMode ?? null,
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

  async openTabInNewWindow(path: string, title?: string): Promise<void> {
    if (!this.isConnected() || !path) return;

    try {
      await invokeTauri('open_tab_in_new_window', {
        path,
        title: title ?? null,
      });
    } catch (error) {
      console.error('Error en TauriVaultRepository al abrir pestaña en nueva ventana:', error);
    }
  }

  async getDetachedWindowTab(): Promise<string | null> {
    if (!this.isConnected()) return null;

    try {
      return await invokeTauri<string | null>('get_detached_window_tab');
    } catch (error) {
      console.warn('Error en TauriVaultRepository al consultar get_detached_window_tab:', error);
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

  async getNoteMeta(relativePath: string): Promise<VaultFile | null> {
    if (!this.isConnected() || !relativePath) return null;

    try {
      return await invokeTauri<VaultFile | null>('get_note_meta', {
        relativePath,
      });
    } catch (error) {
      console.warn(`Error en TauriVaultRepository al obtener get_note_meta para ${relativePath}:`, error);
      return null;
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

  async getStartupTarget(): Promise<StartupTarget> {
    if (!this.isConnected()) {
      return null;
    }
    try {
      return await invokeTauri<StartupTarget>('get_startup_target');
    } catch (error) {
      console.error('Error al obtener startup target:', error);
      return null;
    }
  }

  async selectStandaloneFile(): Promise<string | null> {
    if (!this.isConnected()) {
      return null;
    }
    try {
      return await invokeTauri<string | null>('select_standalone_file');
    } catch (error) {
      console.error('Error al seleccionar archivo puntual:', error);
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

  async gitCommit(paths: string[], message: string): Promise<string> {
    if (!this.isConnected() || paths.length === 0) {
      throw new Error('No hay archivos seleccionados para confirmar');
    }

    return await invokeTauri<string>('git_commit_paths', {
      paths,
      message,
    });
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

  async getGitFileDiff(path: string, staged: boolean, revision?: string): Promise<GitDiffResponse> {
    if (!this.isConnected()) {
      return {
        path,
        is_staged: staged,
        old_file_name: path,
        new_file_name: path,
        old_content: '',
        new_content: '',
        diff: '',
      };
    }

    return await invokeTauri<GitDiffResponse>('get_git_file_diff', {
      path,
      staged,
      revision: revision ?? null,
    });
  }

  async getGitFileRevision(path: string, staged: boolean, revision?: string): Promise<GitFileRevisionResult> {
    if (!this.isConnected()) {
      return {
        path,
        revision: revision || (staged ? 'staged' : 'HEAD'),
        content: null,
        data_url: null,
        is_binary: false,
        exists: false,
      };
    }

    return await invokeTauri<GitFileRevisionResult>('get_git_file_revision', {
      path,
      staged,
      revision: revision ?? null,
    });
  }

  async getGitImageDiff(path: string, staged: boolean, revision?: string): Promise<GitImageDiffResult> {
    if (!this.isConnected()) {
      return {
        path,
        is_staged: staged,
        diff_data_url: null,
        diff_count: 0,
        width: 0,
        height: 0,
        total_pixels: 0,
        diff_percentage: 0,
        has_differences: false,
      };
    }

    return await invokeTauri<GitImageDiffResult>('get_git_image_diff', {
      path,
      staged,
      revision: revision ?? null,
    });
  }

  async getGitFileLog(path: string, maxCount = 50): Promise<GitFileLogResult> {
    if (!this.isConnected()) {
      return {
        path,
        commits: [],
      };
    }

    return await invokeTauri<GitFileLogResult>('get_git_file_log', {
      path,
      limit: maxCount,
    });
  }

  async getGitBranches(): Promise<GitBranchesResult> {
    if (!this.isConnected()) {
      return {
        current_branch: null,
        local_branches: [],
        remote_branches: [],
      };
    }

    return await invokeTauri<GitBranchesResult>('get_git_branches');
  }

  async gitCheckout(branchName: string): Promise<string> {
    if (!this.isConnected()) {
      throw new Error('No hay conexión con la bóveda');
    }

    return await invokeTauri<string>('git_checkout_branch', {
      branchName,
      branch_name: branchName,
    });
  }

  async gitCreateBranch(params: {
    newBranch: string;
    baseBranch?: string;
    checkout?: boolean;
  }): Promise<string> {
    if (!this.isConnected()) {
      throw new Error('No hay conexión con la bóveda');
    }

    return await invokeTauri<string>('git_create_branch', {
      newBranch: params.newBranch,
      new_branch: params.newBranch,
      baseBranch: params.baseBranch || '',
      base_branch: params.baseBranch || '',
      checkout: params.checkout ?? false,
    });
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

  async resolveVaultLink(link: string, baseFile?: string | null): Promise<string | null> {
    if (!this.isConnected() || !link) return null;
    try {
      return await invokeTauri<string | null>('resolve_vault_link', {
        link,
        baseFile: baseFile ?? null,
        base_file: baseFile ?? null,
      });
    } catch (error) {
      console.warn('Error al resolver enlace de bóveda:', error);
      return null;
    }
  }

  async resolveVaultLinks(links: string[], baseFile?: string | null): Promise<Record<string, string>> {
    if (!this.isConnected() || !links || links.length === 0) return {};
    try {
      const res = await invokeTauri<Record<string, string>>('resolve_vault_links', {
        links,
        baseFile: baseFile ?? null,
        base_file: baseFile ?? null,
      });
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

  async saveBinaryAsset(relativePath: string, base64Data: string): Promise<void> {
    if (!this.isConnected() || !relativePath) return;
    try {
      await invokeTauri<void>('save_binary_asset', {
        relativePath,
        relative_path: relativePath,
        base64Data,
        base64_data: base64Data,
      });
    } catch (error) {
      console.error('Error al guardar archivo binario en la bóveda:', { relativePath, error });
      throw error;
    }
  }

  async readBinary(relativePath: string): Promise<ArrayBuffer> {
    if (!this.isConnected() || !relativePath) return new ArrayBuffer(0);
    try {
      const raw = await invokeTauri<ArrayBuffer | Uint8Array | number[]>('read_binary_file', {
        relativePath,
        relative_path: relativePath,
      });
      if (raw instanceof ArrayBuffer) return raw;
      if (raw instanceof Uint8Array) {
        return raw.buffer.slice(raw.byteOffset, raw.byteOffset + raw.byteLength) as ArrayBuffer;
      }
      if (Array.isArray(raw)) return new Uint8Array(raw).buffer as ArrayBuffer;
      return new ArrayBuffer(0);
    } catch (error) {
      console.error('Error al leer archivo binario en la bóveda:', { relativePath, error });
      throw error;
    }
  }

  async writeBinary(relativePath: string, data: ArrayBuffer | Uint8Array): Promise<void> {
    if (!this.isConnected() || !relativePath) return;
    try {
      const bytes = data instanceof Uint8Array ? Array.from(data) : Array.from(new Uint8Array(data));
      await invokeTauri<void>('write_binary_file', {
        relativePath,
        relative_path: relativePath,
        data: bytes,
      });
    } catch (error) {
      console.error('Error al escribir archivo binario en la bóveda:', { relativePath, error });
      throw error;
    }
  }

  async createFolder(relativePath: string): Promise<void> {
    if (!this.isConnected() || !relativePath) return;
    try {
      await invokeTauri<void>('create_vault_folder', {
        relativePath,
        relative_path: relativePath,
      });
    } catch (error) {
      console.error('Error al crear carpeta en la bóveda:', { relativePath, error });
      throw error;
    }
  }

  async onFilesChanged(callback: (payload: { paths: string[]; deleted: string[] }) => void): Promise<() => void> {
    if (!this.isConnected()) {
      return () => {};
    }
    try {
      const { listen } = await import('@tauri-apps/api/event');
      return await listen<{ paths: string[]; deleted: string[] }>('vault:files-changed', (event) => {
        callback(event.payload);
      });
    } catch (error) {
      console.warn('No se pudo suscribir a vault:files-changed:', error);
      return () => {};
    }
  }

  async renderMarkdownWikilinks(content: string, baseFile?: string | null): Promise<string> {
    if (!this.isConnected() || !content) return content;
    try {
      const res = await invokeTauri<string>('render_markdown_wikilinks', {
        content,
        baseFile: baseFile ?? null,
        base_file: baseFile ?? null,
      });
      return typeof res === 'string' ? res : content;
    } catch (error) {
      console.warn('Error al renderizar wikilinks en markdown:', error);
      return content;
    }
  }

  async getRecentWorkspaces(): Promise<RecentWorkspaceEntry[]> {
    if (!this.isConnected()) return [];
    try {
      const list = await invokeTauri<RecentWorkspaceEntry[]>('get_recent_workspaces');
      return Array.isArray(list) ? list : [];
    } catch (error) {
      console.warn('Error al obtener get_recent_workspaces:', error);
      return [];
    }
  }

  async togglePinWorkspace(path: string): Promise<RecentWorkspaceEntry[]> {
    if (!this.isConnected() || !path) return [];
    try {
      const list = await invokeTauri<RecentWorkspaceEntry[]>('toggle_pin_workspace', { path });
      return Array.isArray(list) ? list : [];
    } catch (error) {
      console.warn('Error al alternar toggle_pin_workspace:', error);
      return [];
    }
  }

  async removeRecentWorkspace(path: string): Promise<RecentWorkspaceEntry[]> {
    if (!this.isConnected() || !path) return [];
    try {
      const list = await invokeTauri<RecentWorkspaceEntry[]>('remove_recent_workspace', { path });
      return Array.isArray(list) ? list : [];
    } catch (error) {
      console.warn('Error al eliminar remove_recent_workspace:', error);
      return [];
    }
  }

  async createNewWorkspace(startingDirectory?: string): Promise<SelectVaultFolderResult | null> {
    if (!this.isConnected()) return null;
    try {
      return await invokeTauri<SelectVaultFolderResult | null>('create_new_workspace', {
        startingDirectory: startingDirectory ?? null,
        starting_directory: startingDirectory ?? null,
      });
    } catch (error) {
      console.error('Error al crear nuevo workspace:', error);
      return null;
    }
  }

  async cloneGitWorkspace(repoUrl: string, startingDirectory?: string): Promise<SelectVaultFolderResult | null> {
    if (!this.isConnected() || !repoUrl.trim()) return null;
    try {
      return await invokeTauri<SelectVaultFolderResult | null>('clone_git_workspace', {
        repoUrl,
        repo_url: repoUrl,
        startingDirectory: startingDirectory ?? null,
        starting_directory: startingDirectory ?? null,
      });
    } catch (error) {
      console.error('Error al clonar repositorio Git como workspace:', error);
      throw error;
    }
  }

  async openWorkspaceInNewWindow(path: string): Promise<void> {
    if (!this.isConnected() || !path) return;
    try {
      await invokeTauri<void>('open_workspace_in_new_window', { path });
    } catch (error) {
      console.error('Error al abrir workspace en nueva ventana:', error);
    }
  }

  async openFileWithDefaultApp(relativePath: string): Promise<void> {
    if (!this.isConnected() || !relativePath) return;
    try {
      await invokeTauri<void>('open_file_with_default_app', { path: relativePath });
    } catch (error) {
      console.error('Error al abrir archivo con la aplicación por defecto del sistema:', error);
      throw error;
    }
  }
}

export const vaultRepository: VaultRepository = new TauriVaultRepository();
