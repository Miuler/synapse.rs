import type { Completion, CompletionContext, CompletionResult } from '@codemirror/autocomplete';
import {
  D2_ENGINES,
  D2_THEMES,
  D2_SHAPES,
  D2_DIRECTIONS,
  D2_STYLES,
  MERMAID_THEMES,
  MERMAID_LOOKS,
  MERMAID_LAYOUTS,
  MERMAID_CURVES,
  MERMAID_DIAGRAM_TYPES,
} from '@shared/lib/diagrams';

/**
 * Autocompletado inteligente para el lenguaje D2.
 */
export function d2Autocomplete(context: CompletionContext): CompletionResult | null {
  const line = context.state.doc.lineAt(context.pos);
  const textBefore = line.text.slice(0, context.pos - line.from);
  const word = context.matchBefore(/[\w\-:]*/) ?? { from: context.pos, to: context.pos, text: '' };

  // 1. Si está tras 'layout-engine:'
  if (/(?:layout-engine|layout)\s*:\s*[\w\-]*$/i.test(textBefore)) {
    const from = line.from + textBefore.lastIndexOf(':') + 1;
    return {
      from: from + (textBefore.slice(from - line.from).match(/^\s*/)?.[0]?.length || 0),
      options: D2_ENGINES.map((e) => ({
        label: e.label,
        type: 'keyword',
        detail: e.detail,
        boost: 2,
      })),
    };
  }

  // 2. Si está tras 'theme-id:' o 'dark-theme-id:'
  if (/(?:theme-id|dark-theme-id)\s*:\s*[\w\-]*$/i.test(textBefore)) {
    const from = line.from + textBefore.lastIndexOf(':') + 1;
    return {
      from: from + (textBefore.slice(from - line.from).match(/^\s*/)?.[0]?.length || 0),
      options: D2_THEMES.map((t) => ({
        label: t.label,
        type: 'constant',
        detail: t.detail,
        boost: t.id === 0 || t.id === 200 ? 5 : 1,
      })),
    };
  }

  // 3. Si está tras 'shape:'
  if (/shape\s*:\s*[\w\-]*$/i.test(textBefore)) {
    const from = line.from + textBefore.lastIndexOf(':') + 1;
    return {
      from: from + (textBefore.slice(from - line.from).match(/^\s*/)?.[0]?.length || 0),
      options: D2_SHAPES.map((s) => ({
        label: s,
        type: 'type',
        detail: 'Forma geométrica D2',
        boost: 2,
      })),
    };
  }

  // 4. Si está tras 'direction:'
  if (/direction\s*:\s*[\w\-]*$/i.test(textBefore)) {
    const from = line.from + textBefore.lastIndexOf(':') + 1;
    return {
      from: from + (textBefore.slice(from - line.from).match(/^\s*/)?.[0]?.length || 0),
      options: D2_DIRECTIONS.map((d) => ({
        label: d,
        type: 'keyword',
        detail: 'Dirección del flujo',
        boost: 2,
      })),
    };
  }

  // 5. Si está tras 'sketch:' o valores booleanos
  if (/(?:sketch|animated|shadow|3d|multiple|double-border)\s*:\s*[\w\-]*$/i.test(textBefore)) {
    const from = line.from + textBefore.lastIndexOf(':') + 1;
    return {
      from: from + (textBefore.slice(from - line.from).match(/^\s*/)?.[0]?.length || 0),
      options: [
        { label: 'true', type: 'keyword', boost: 2 },
        { label: 'false', type: 'keyword', boost: 2 },
      ],
    };
  }

  // 6. Si está tras 'style.'
  if (/style\.\s*[\w\-]*$/i.test(textBefore)) {
    const dotPos = textBefore.lastIndexOf('.');
    return {
      from: line.from + dotPos + 1,
      options: D2_STYLES.map((st) => ({
        label: st,
        type: 'property',
        detail: 'Propiedad de estilo D2',
        boost: 2,
      })),
    };
  }

  // 7. Snippets generales y propiedades de primer nivel
  const trimmed = textBefore.trim();
  const options: Completion[] = [
    {
      label: 'd2-config',
      detail: 'Snippet de configuración declarativa del diagrama',
      type: 'snippet',
      apply: 'vars: {\n  d2-config: {\n    layout-engine: dagre\n    theme-id: 0\n    sketch: false\n  }\n}\n',
      boost: 10,
    },
    {
      label: 'direction: right',
      detail: 'Flujo horizontal de izquierda a derecha',
      type: 'keyword',
      boost: 5,
    },
    {
      label: 'direction: down',
      detail: 'Flujo vertical de arriba hacia abajo',
      type: 'keyword',
      boost: 5,
    },
    {
      label: 'shape: ',
      detail: 'Asignar forma al nodo',
      type: 'property',
      boost: 4,
    },
    {
      label: 'style: ',
      detail: 'Bloque de estilos del nodo/conexión',
      type: 'property',
      boost: 4,
    },
    {
      label: 'tooltip: ',
      detail: 'Texto emergente al pasar el cursor',
      type: 'property',
      boost: 3,
    },
    {
      label: 'link: ',
      detail: 'Enlace web al hacer clic en el nodo',
      type: 'property',
      boost: 3,
    },
  ];

  if (!word && !context.explicit) return null;

  return {
    from: word.from,
    options,
  };
}

/**
 * Autocompletado inteligente para el lenguaje Mermaid.
 */
export function mermaidAutocomplete(context: CompletionContext): CompletionResult | null {
  const line = context.state.doc.lineAt(context.pos);
  const textBefore = line.text.slice(0, context.pos - line.from);
  const word = context.matchBefore(/[\w\-:]*/) ?? { from: context.pos, to: context.pos, text: '' };

  // 1. Si está completando 'theme:'
  if (/(?:["']?theme["']?)\s*:\s*["']?[\w\-]*$/i.test(textBefore)) {
    const from = line.from + textBefore.lastIndexOf(':') + 1;
    return {
      from: from + (textBefore.slice(from - line.from).match(/^\s*["']?/)?.[0]?.length || 0),
      options: MERMAID_THEMES.map((t) => ({
        label: t.label,
        type: 'constant',
        detail: t.detail,
        boost: 2,
      })),
    };
  }

  // 2. Si está completando 'look:'
  if (/(?:["']?look["']?)\s*:\s*["']?[\w\-]*$/i.test(textBefore)) {
    const from = line.from + textBefore.lastIndexOf(':') + 1;
    return {
      from: from + (textBefore.slice(from - line.from).match(/^\s*["']?/)?.[0]?.length || 0),
      options: MERMAID_LOOKS.map((l) => ({
        label: l.label,
        type: 'keyword',
        detail: l.detail,
        boost: 2,
      })),
    };
  }

  // 3. Si está completando 'layout:'
  if (/(?:["']?layout["']?)\s*:\s*["']?[\w\-]*$/i.test(textBefore)) {
    const from = line.from + textBefore.lastIndexOf(':') + 1;
    return {
      from: from + (textBefore.slice(from - line.from).match(/^\s*["']?/)?.[0]?.length || 0),
      options: MERMAID_LAYOUTS.map((ly) => ({
        label: ly.label,
        type: 'keyword',
        detail: ly.detail,
        boost: 2,
      })),
    };
  }

  // 4. Si está completando 'curve:'
  if (/(?:["']?curve["']?)\s*:\s*["']?[\w\-]*$/i.test(textBefore)) {
    const from = line.from + textBefore.lastIndexOf(':') + 1;
    return {
      from: from + (textBefore.slice(from - line.from).match(/^\s*["']?/)?.[0]?.length || 0),
      options: MERMAID_CURVES.map((c) => ({
        label: c,
        type: 'keyword',
        detail: 'Tipo de curva de conexión',
        boost: 2,
      })),
    };
  }

  // 5. Si está al inicio del documento (línea 1 o vacía antes) sugerir tipos de diagrama o snippets init
  if (line.number <= 2) {
    const options: Completion[] = [
      {
        label: '%%{init: ...}%%',
        detail: 'Directiva de configuración in-file de Mermaid',
        type: 'snippet',
        apply: '%%{init: {"theme": "default", "look": "classic"}}%%\n',
        boost: 10,
      },
      {
        label: '--- (Frontmatter config)',
        detail: 'Bloque frontmatter YAML para configurar Mermaid',
        type: 'snippet',
        apply: '---\nconfig:\n  theme: default\n  look: handDrawn\n---\n',
        boost: 9,
      },
      ...MERMAID_DIAGRAM_TYPES.map((dt) => ({
        label: dt.label,
        type: 'keyword',
        detail: dt.detail,
        boost: 5,
      })),
    ];

    return {
      from: word.from,
      options,
    };
  }

  // 6. Palabras clave generales de Mermaid
  const generalOptions: Completion[] = [
    { label: 'subgraph', detail: 'Subgrafo / agrupación', type: 'keyword', apply: 'subgraph Titulo\n  \nend', boost: 3 },
    { label: 'participant', detail: 'Participante en diagrama de secuencia', type: 'keyword', boost: 2 },
    { label: 'actor', detail: 'Actor en diagrama de secuencia', type: 'keyword', boost: 2 },
    { label: 'autonumber', detail: 'Numeración automática de pasos', type: 'keyword', boost: 2 },
    { label: 'classDef', detail: 'Definición de estilo de clase', type: 'keyword', boost: 2 },
    { label: 'style', detail: 'Estilo directo para un nodo', type: 'keyword', boost: 2 },
  ];

  if (!word && !context.explicit) return null;

  return {
    from: word.from,
    options: generalOptions,
  };
}
