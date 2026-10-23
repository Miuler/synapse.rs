/**
 * Definición canónica de un comando de Obsidian.
 */
export interface Command {
  id: string;
  name: string;
  icon?: string;
  mobileOnly?: boolean;
  callback?: () => any;
  checkCallback?: (checking: boolean) => boolean | void;
  editorCallback?: (editor: any, view: any) => any;
  editorCheckCallback?: (checking: boolean, editor: any, view: any) => boolean | void;
  hotkeys?: Array<{ modifiers: string[]; key: string }>;
}

/**
 * Gestor central de comandos de SynapseApp (app.commands).
 */
export class SynapseCommandManager {
  private readonly commands = new Map<string, Command>();

  /**
   * Registra un nuevo comando en la paleta de comandos.
   */
  addCommand(command: Command): Command {
    this.commands.set(command.id, command);
    return command;
  }

  /**
   * Remueve un comando por su identificador.
   */
  removeCommand(id: string): void {
    this.commands.delete(id);
  }

  /**
   * Obtiene la lista completa de comandos registrados.
   */
  listCommands(): Command[] {
    return Array.from(this.commands.values());
  }

  /**
   * Busca un comando por su ID.
   */
  findCommand(id: string): Command | undefined {
    return this.commands.get(id);
  }

  /**
   * Ejecuta un comando registrado por su ID. Retorna true si se ejecutó con éxito.
   */
  executeCommandById(id: string): boolean {
    const cmd = this.commands.get(id);
    if (!cmd) return false;

    if (typeof cmd.callback === 'function') {
      cmd.callback();
      return true;
    }

    if (typeof cmd.checkCallback === 'function') {
      const allowed = cmd.checkCallback(true);
      if (allowed !== false) {
        cmd.checkCallback(false);
        return true;
      }
    }

    return false;
  }
}
