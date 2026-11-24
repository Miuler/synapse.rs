import type { CommandCategory, CommandId } from '../lib/texts';

export interface AppCommand {
  id: CommandId;
  category: CommandCategory;
  icon?: string; // Nombre del icono SVG
  shortcut?: string;
  action: () => void;
}

class CommandRegistry {
  // Estado reactivo de Svelte 5 para almacenar comandos registrados
  commands = $state<AppCommand[]>([]);

  get all(): AppCommand[] {
    return this.commands;
  }

  register(command: AppCommand) {
    // Evitar duplicados por id
    const index = this.commands.findIndex((c) => c.id === command.id);
    if (index >= 0) {
      this.commands[index] = command;
    } else {
      this.commands.push(command);
    }
  }

  registerMany(newCommands: AppCommand[]) {
    newCommands.forEach((cmd) => this.register(cmd));
  }

  unregister(commandId: string) {
    const index = this.commands.findIndex((c) => c.id === commandId);
    if (index >= 0) {
      this.commands.splice(index, 1);
    }
  }
}

export const commandRegistry = new CommandRegistry();
