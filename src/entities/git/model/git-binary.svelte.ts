// Ejecutable de Git que usa Synapse para commit, checkout, push, clone y LFS: el del PATH o el que
// elija el usuario (se guarda en config.jsonc → git.path).
import { invokeTauri, isTauriEnvironment } from '@shared/api';

export interface GitInfo {
  configuredPath?: string | null;
  /** Versión del Git en uso; `null` si no se encontró */
  version?: string | null;
}

const DISMISSED_KEY = 'synapse.gitMissing.dismissed';

class GitBinary {
  info = $state<GitInfo | null>(null);
  error = $state<string | null>(null);
  busy = $state(false);

  /** Hay que preguntar: se buscó y no hay Git que funcione */
  get missing(): boolean {
    return !!this.info && !this.info.version;
  }

  get dismissed(): boolean {
    try {
      return localStorage.getItem(DISMISSED_KEY) === '1';
    } catch {
      return false;
    }
  }

  dismiss(): void {
    try {
      localStorage.setItem(DISMISSED_KEY, '1');
    } catch {
      /* sin almacenamiento */
    }
  }

  async load(): Promise<GitInfo | null> {
    if (!isTauriEnvironment()) return null;
    this.info = await invokeTauri<GitInfo>('get_git_info');
    return this.info;
  }

  /** Abre el diálogo nativo y guarda la ruta elegida. Devuelve `true` si quedó un Git que funciona. */
  async choose(): Promise<boolean> {
    this.error = null;
    const path = await invokeTauri<string | null>('pick_git_executable');
    if (!path) return false;
    return this.setPath(path);
  }

  async setPath(path: string | null): Promise<boolean> {
    this.busy = true;
    this.error = null;
    try {
      this.info = await invokeTauri<GitInfo>('set_git_path', { path });
      if (this.info.version) {
        try {
          localStorage.removeItem(DISMISSED_KEY);
        } catch {
          /* sin almacenamiento */
        }
      }
      return !!this.info.version;
    } catch (e) {
      this.error = e === 'Cancelado' || e === 'Cancelled' ? null : e instanceof Error ? e.message : String(e);
      return false;
    } finally {
      this.busy = false;
    }
  }
}

export const gitBinary = new GitBinary();
