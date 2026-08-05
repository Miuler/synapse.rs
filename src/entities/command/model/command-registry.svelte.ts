export interface AppCommand {
  id: string;
  name: string;
  category: string;
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

  // Algoritmo de búsqueda difusa (Fuzzy Search) liviano y rápido
  search(query: string): AppCommand[] {
    if (!query || query.trim() === '') {
      return this.commands;
    }

    const cleanQuery = query.toLowerCase().trim();

    return this.commands
      .map((cmd) => {
        const text = `${cmd.category} ${cmd.name}`.toLowerCase();
        let score = 0;
        let queryIdx = 0;

        // Puntuación por coincidencia de subcadena exacta
        if (text.includes(cleanQuery)) {
          score += 100 - text.indexOf(cleanQuery);
        }

        // Puntuación por coincidencia difusa de caracteres ordenados
        for (let i = 0; i < text.length && queryIdx < cleanQuery.length; i++) {
          if (text[i] === cleanQuery[queryIdx]) {
            score += 10;
            queryIdx++;
          }
        }

        const matchesAll = queryIdx === cleanQuery.length;

        return { cmd, score: matchesAll ? score : -1 };
      })
      .filter((item) => item.score > -1)
      .sort((a, b) => b.score - a.score)
      .map((item) => item.cmd);
  }
}

export const commandRegistry = new CommandRegistry();
