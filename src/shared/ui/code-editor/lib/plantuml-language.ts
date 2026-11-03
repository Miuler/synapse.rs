import { StreamLanguage } from '@codemirror/language';

export const plantumlLanguage = StreamLanguage.define<{ inBlockComment: boolean }>({
  name: 'plantuml',
  startState() {
    return { inBlockComment: false };
  },
  token(stream, state) {
    if (state.inBlockComment) {
      if (stream.match(/^.*?'\//)) {
        state.inBlockComment = false;
      } else {
        stream.skipToEnd();
      }
      return 'comment';
    }

    if (stream.eatSpace()) return null;

    // Comentarios /' ... '/ y ' ...
    if (stream.match(/^\/'/)) {
      state.inBlockComment = true;
      return 'comment';
    }
    if (stream.match(/^'.*/)) {
      return 'comment';
    }

    // Delimitadores @startuml / @enduml y directivas del preprocesador
    if (stream.match(/^@(start|end)\w+/i) || stream.match(/^![a-z]+/i)) {
      return 'meta';
    }

    if (stream.match(/^"[^"]*"/)) {
      return 'string';
    }

    // Flechas: ->, -->, <-, <--, ->>, ..>, -[#red]->, <|--, *--, o--
    if (stream.match(/^[<*o|\\/]*(-+|\.+)(\[[^\]]*\]-*)?[>|\\/*o]*(?=\s|$|\w)/)) {
      return 'operator';
    }

    if (
      stream.match(
        /^(participant|actor|boundary|control|entity|database|collections|queue|class|interface|abstract|enum|annotation|package|namespace|component|node|folder|frame|cloud|rectangle|usecase|state|object|map|json)\b/i
      )
    ) {
      return 'typeName';
    }

    if (
      stream.match(
        /^(note|end|as|title|header|footer|legend|caption|skinparam|hide|show|autonumber|activate|deactivate|destroy|create|return|newpage|alt|else|opt|loop|par|break|critical|group|box|start|stop|if|then|elseif|endif|while|endwhile|repeat|fork|endfork|partition|left|right|top|bottom|over|of|on|to|direction)\b/i
      )
    ) {
      return 'keyword';
    }

    // Colores #fff o #Nombre
    if (stream.match(/^#\w+/)) {
      return 'atom';
    }
    if (stream.match(/^\d+(\.\d+)?\b/)) {
      return 'number';
    }

    stream.match(/^\w+/) || stream.next();
    return null;
  },
});
