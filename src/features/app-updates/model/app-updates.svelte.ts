// Estado de las actualizaciones: comprobar (al arrancar o a mano), descargar/instalar y reiniciar.
import { invokeTauri, isTauriEnvironment } from '@shared/repositories/tauri';
import { listen } from '@tauri-apps/api/event';

export interface UpdateInfo {
  version: string;
  currentVersion: string;
  notes?: string | null;
  date?: string | null;
}

type Status = 'idle' | 'checking' | 'up-to-date' | 'available' | 'installing' | 'installed' | 'error';

const DISMISSED_KEY = 'synapse.update.dismissed';

function readDismissed(): string | null {
  try {
    return localStorage.getItem(DISMISSED_KEY);
  } catch {
    return null;
  }
}

class AppUpdates {
  status = $state<Status>('idle');
  available = $state<UpdateInfo | null>(null);
  error = $state<string | null>(null);
  progress = $state<{ downloaded: number; total?: number | null } | null>(null);
  /** El aviso flotante está visible */
  noticeOpen = $state(false);

  constructor() {
    if (isTauriEnvironment()) {
      void listen<{ downloaded: number; total?: number | null }>('app-update-progress', (e) => {
        this.progress = e.payload;
      });
    }
  }

  /** `silent`: comprobación automática; no avisa si no hay nada ni si falla, ni repite una versión descartada. */
  async check(silent = false): Promise<void> {
    if (!isTauriEnvironment() || this.status === 'checking' || this.status === 'installing') return;
    this.status = 'checking';
    this.error = null;
    try {
      const info = await invokeTauri<UpdateInfo | null>('check_for_update');
      this.available = info;
      this.status = info ? 'available' : 'up-to-date';
      this.noticeOpen = !!info && !(silent && readDismissed() === info.version);
      if (!info && !silent) this.noticeOpen = true;
    } catch (e) {
      this.status = 'error';
      this.error = e instanceof Error ? e.message : String(e);
      this.noticeOpen = !silent;
    }
  }

  async install(): Promise<void> {
    if (this.status === 'installing') return;
    this.status = 'installing';
    this.error = null;
    this.progress = null;
    this.noticeOpen = true;
    try {
      await invokeTauri<string>('install_update');
      this.status = 'installed';
    } catch (e) {
      this.status = 'error';
      this.error = e instanceof Error ? e.message : String(e);
    }
  }

  restart(): void {
    void invokeTauri('restart_app');
  }

  /** «Más tarde»: no vuelve a avisar de esta versión al arrancar (sí al buscar a mano). */
  dismiss(): void {
    if (this.available && this.status === 'available') {
      try {
        localStorage.setItem(DISMISSED_KEY, this.available.version);
      } catch {
        /* sin almacenamiento */
      }
    }
    this.noticeOpen = false;
  }
}

export const appUpdates = new AppUpdates();
