import { StreamLanguage } from '@codemirror/language';

export const d2Language = StreamLanguage.define({
  name: 'd2',
  startState() {
    return { inString: false, inBlockString: false };
  },
  token(stream, state) {
    if (stream.eatSpace()) return null;

    // Bloque multilínea """ ... """ o |' ... '|
    if (state.inBlockString) {
      if (stream.match(/^"""/) || stream.match(/^\|'/)) {
        state.inBlockString = false;
        return 'string';
      }
      stream.skipToEnd();
      return 'string';
    }

    if (stream.match(/^"""/) || stream.match(/^\|'/)) {
      state.inBlockString = true;
      return 'string';
    }

    // Comentarios # ...
    if (stream.match(/^#.*/)) {
      return 'comment';
    }

    // Strings de una línea "..." o '...'
    if (stream.match(/^"[^"]*"/)) {
      return 'string';
    }
    if (stream.match(/^'[^']*'/)) {
      return 'string';
    }

    // Flechas y conexiones: ->, <-, <->, --
    if (stream.match(/^(-->|<--|<->|->|<-|--)/)) {
      return 'operator';
    }

    // Palabras clave principales de D2
    if (stream.match(/^(vars|d2-config|layout-engine|theme-id|dark-theme-id|sketch|pad|center|direction|shape|style|icon|tooltip|link|near|width|height|grid-rows|grid-columns|grid-gap|horizontal-gap|vertical-gap|classes|layers|scenarios|steps|top|left)\b/i)) {
      return 'keyword';
    }

    // Formas nativas
    if (stream.match(/^(rectangle|square|page|parallelogram|document|cylinder|queue|package|step|callout|stored_data|person|diamond|oval|circle|hexagon|cloud|sql_table|class|sequence_diagram|text|code|image)\b/i)) {
      return 'typeName';
    }

    // Booleanos y direcciones
    if (stream.match(/^(true|false|right|down|left|up)\b/i)) {
      return 'atom';
    }

    // Números o colores hex
    if (stream.match(/^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})\b/)) {
      return 'number';
    }
    if (stream.match(/^[0-9]+(\.[0-9]+)?\b/)) {
      return 'number';
    }

    // Identificadores de nodo o contenedor
    if (stream.match(/^[a-zA-Z_][a-zA-Z0-9_\-]*/)) {
      return 'variableName';
    }

    stream.next();
    return null;
  },
});
