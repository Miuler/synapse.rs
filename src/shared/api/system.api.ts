import { invokeTauri } from './tauri-client';

export type StartupTarget =
  | { type: 'standalone'; path: string }
  | { type: 'vault'; path: string }
  | null;

export interface McpRuntimeStatus {
  running: boolean;
  port: number;
  endpoint: string;
  active_vault?: string | null;
  requests_handled: number;
  last_error?: string | null;
}

export interface McpPortRegistryEntry {
  vault_path: string;
  port: number;
  enabled: boolean;
  running: boolean;
  pid?: number | null;
  endpoint: string;
  updated_at: number;
}

export interface McpPortRegistry {
  vaults: Record<string, McpPortRegistryEntry>;
}

export const systemApi = {
  async getAppVersion(): Promise<string> {
    try {
      return await invokeTauri<string>('get_app_version');
    } catch {
      return '1.0.0-alpha.1';
    }
  },

  async openFileWithDefaultApp(relativePath: string): Promise<void> {
    if (!relativePath) return;
    try {
      await invokeTauri<void>('open_file_with_default_app', { path: relativePath });
    } catch (error) {
      console.error('Error al abrir archivo con la aplicación por defecto del sistema:', error);
      throw error;
    }
  },

  async showItemInFolder(relativePath: string): Promise<void> {
    if (!relativePath) return;
    try {
      await invokeTauri<void>('show_item_in_folder', { path: relativePath });
    } catch (error) {
      console.error('Error al mostrar en carpeta en el sistema operativo:', error);
      throw error;
    }
  },

  async openTabInNewWindow(path: string, title?: string): Promise<void> {
    if (!path) return;
    try {
      await invokeTauri('open_tab_in_new_window', {
        path,
        title: title ?? null,
      });
    } catch (error) {
      console.error('Error al abrir pestaña en nueva ventana:', error);
    }
  },

  async getDetachedWindowTab(): Promise<string | null> {
    try {
      return await invokeTauri<string | null>('get_detached_window_tab');
    } catch (error) {
      console.error('Error al obtener pestaña de ventana independiente:', error);
      return null;
    }
  },

  async getStartupTarget(): Promise<StartupTarget> {
    try {
      return await invokeTauri<StartupTarget>('get_startup_target');
    } catch (error) {
      console.error('Error al obtener startup target:', error);
      return null;
    }
  },

  async selectStandaloneFile(): Promise<string | null> {
    try {
      return await invokeTauri<string | null>('select_standalone_file');
    } catch (error) {
      console.error('Error al seleccionar archivo puntual:', error);
      return null;
    }
  },

  async getMcpServerStatus(): Promise<McpRuntimeStatus | null> {
    try {
      return await invokeTauri<McpRuntimeStatus>('get_mcp_server_status');
    } catch (error) {
      console.error('Error al obtener estado del servidor MCP:', error);
      return null;
    }
  },

  async toggleMcpServer(): Promise<McpRuntimeStatus | null> {
    try {
      return await invokeTauri<McpRuntimeStatus>('toggle_mcp_server');
    } catch (error) {
      console.error('Error al alternar servidor MCP:', error);
      return null;
    }
  },

  async startMcpServer(): Promise<McpRuntimeStatus | null> {
    try {
      return await invokeTauri<McpRuntimeStatus>('start_mcp_server');
    } catch (error) {
      console.error('Error al iniciar servidor MCP:', error);
      return null;
    }
  },

  async stopMcpServer(): Promise<McpRuntimeStatus | null> {
    try {
      return await invokeTauri<McpRuntimeStatus>('stop_mcp_server');
    } catch (error) {
      console.error('Error al detener servidor MCP:', error);
      return null;
    }
  },

  async getMcpPortRegistry(): Promise<McpPortRegistry | null> {
    try {
      return await invokeTauri<McpPortRegistry>('get_mcp_port_registry');
    } catch (error) {
      console.error('Error al obtener registro de puertos MCP:', error);
      return null;
    }
  },
};
