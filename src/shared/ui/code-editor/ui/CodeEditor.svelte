<script lang="ts">
  import { onMount, untrack, tick } from 'svelte';
  import { basicSetup } from 'codemirror';
  import { EditorView, keymap, tooltips } from '@codemirror/view';
  import { EditorState, Compartment, Prec } from '@codemirror/state';
  import { markdown } from '@codemirror/lang-markdown';
  import { yaml } from '@codemirror/lang-yaml';
  import { json } from '@codemirror/lang-json';
  import { sql } from '@codemirror/lang-sql';
  import { StreamLanguage, syntaxHighlighting, HighlightStyle } from '@codemirror/language';
  import { tags } from '@lezer/highlight';
  import { searchKeymap, search, openSearchPanel, setSearchQuery, SearchQuery } from '@codemirror/search';
  import { autocompletion } from '@codemirror/autocomplete';
  import { linter, lintGutter, type Diagnostic } from '@codemirror/lint';
  import { vim } from '@replit/codemirror-vim';
  import { undoRedoExtension } from '@shared/lib/editor-history';
  import { commentExtension, withCommentTokens } from '@shared/lib/editor-comment';
  import { d2Language } from '../lib/d2-language';
  import { plantumlLanguage } from '../lib/plantuml-language';
  import { braceFolding } from '../lib/brace-folding';
  import { registerVimFolding } from '../lib/vim-folding';
  import { renameOccurrencesKeymap } from '../lib/rename-occurrences';
  import { lineKeymap } from '../lib/line-keymap';
  import { minimalChange } from '../lib/minimal-change';
  import { jsonRefLinks, jumpToPointer } from '../lib/json-ref';
  import { gitDiffGutter, setGitBaseContent } from '../lib/git-diff-gutter';

  registerVimFolding();
  import { d2Autocomplete, mermaidAutocomplete, plantumlAutocomplete } from '../lib/diagram-autocompletion';
  import { documentWordsCompletion, withDocumentWords } from '../lib/document-words';

  export interface SelectionInfo {
    hasSelection: boolean;
    selectedWords: number;
    selectedChars: number;
    selectedLines: number;
    selectedCols: number;
    cursorLine: number;
    cursorCol: number;
  }

  export interface EditorDiagnostic {
    line: number;
    column: number;
    endLine?: number;
    endColumn?: number;
    message: string;
    severity?: 'error' | 'warning' | 'info';
  }

  interface Props {
    content: string;
    readOnly?: boolean;
    vimMode?: boolean;
    /** Pestaña activa: al volver a ella, el editor recupera el foco (y su cursor). */
    isActive?: boolean;
    /** Ctrl+Enter: acción del editor (en los diagramas, redibujar ya). */
    onRun?: () => void;
    mode?: 'markdown' | 'mermaid' | 'd2' | 'plantuml' | 'yaml' | 'json' | 'sql';
    diagnostics?: EditorDiagnostic[];
    onChange?: (content: string) => void;
    onSelectionChange?: (info: SelectionInfo) => void;
    /** Ruta del archivo (relativa a la bóveda): con ella los `$ref` de YAML/JSON se pueden seguir. */
    filePath?: string | null;
    /** Abre el archivo de un `$ref` y salta a su puntero `#/...`. */
    onOpenRef?: (path: string, pointer: string | null) => void;
    onNotify?: (message: string) => void;
    /** Puntero `#/...` al que saltar cuando el contenido esté cargado (se consume con `onJumped`). */
    jumpPointer?: string | null;
    onJumped?: () => void;
    /** Contenido base de Git (HEAD) para mostrar indicadores de líneas nuevas/modificadas en el gutter. */
    gitBaseContent?: string | null;
    /** Términos de una búsqueda global: se abre el buscador del editor con el primero cargado. */
    scrollToTerms?: string[];
  }

  let {
    content = '',
    readOnly = false,
    vimMode = false,
    isActive = false,
    onRun,
    mode = 'markdown',
    diagnostics = [],
    onChange,
    onSelectionChange,
    filePath = null,
    onOpenRef,
    onNotify,
    jumpPointer = null,
    onJumped,
    gitBaseContent = null,
    scrollToTerms = [],
  }: Props = $props();

  let containerRef = $state<HTMLDivElement | null>(null);
  let editorView: EditorView | null = null;
  let isInternalUpdate = false;
  const vimCompartment = new Compartment();
  const lintCompartment = new Compartment();
  // Lectura/Edición cambia sin volver a crear el editor. En lectura se usa `readOnly` y no
  // `editable`: el editor sigue recibiendo el foco, con cursor, selección y Ctrl+F para buscar.
  const editableCompartment = new Compartment();

  let lastContent = $state(untrack(() => content));

  let lastAppliedTerms: string[] | null = null;
  $effect(() => {
    const terms = scrollToTerms;
    if (!terms || terms.length === 0) return;
    tick().then(() => {
      if (!editorView || lastAppliedTerms === terms) return;
      const doc = editorView.state.doc.toString().toLowerCase();
      const term = terms.map((t) => t.trim()).find((t) => t && doc.includes(t.toLowerCase()));
      if (!term) return;
      lastAppliedTerms = terms;
      const index = doc.indexOf(term.toLowerCase());
      editorView.dispatch({
        selection: { anchor: index, head: index + term.length },
        scrollIntoView: true,
        effects: setSearchQuery.of(new SearchQuery({ search: term })),
      });
      openSearchPanel(editorView as any);
    });
  });

  export function triggerSearch() {
    if (editorView) {
      openSearchPanel(editorView as any);
    }
  }

  function emitSelectionInfo(state: EditorState) {
    if (!onSelectionChange) return;

    const mainSel = state.selection.main;
    const hasSelection = !mainSel.empty;
    const head = mainSel.head;
    const cursorLineObj = state.doc.lineAt(head);
    const cursorLine = cursorLineObj.number;
    const cursorCol = head - cursorLineObj.from + 1;

    if (hasSelection) {
      const from = mainSel.from;
      const to = mainSel.to;
      const selectedText = state.sliceDoc(from, to);
      const fromLine = state.doc.lineAt(from);
      const toLine = state.doc.lineAt(to);
      const selectedLines = toLine.number - fromLine.number + 1;
      const selectedChars = selectedText.length;
      const selectedCols = selectedLines === 1 ? to - from : selectedChars;
      const trimmed = selectedText.trim();
      const selectedWords = trimmed ? trimmed.split(/\s+/).length : 0;

      onSelectionChange({
        hasSelection: true,
        selectedWords,
        selectedChars,
        selectedLines,
        selectedCols,
        cursorLine,
        cursorCol,
      });
    } else {
      onSelectionChange({
        hasSelection: false,
        selectedWords: 0,
        selectedChars: 0,
        selectedLines: 1,
        selectedCols: 0,
        cursorLine,
        cursorCol,
      });
    }
  }

  // Lenguaje Mermaid personalizado con StreamLanguage
  const mermaidLanguage = StreamLanguage.define({
    name: 'mermaid',
    startState() {
      return {};
    },
    token(stream) {
      if (stream.eatSpace()) return null;

      // Comentarios %% ...
      if (stream.match(/^%%.*/)) {
        return 'comment';
      }

      // Palabras clave de tipos de diagrama
      if (
        stream.match(
          /^(flowchart|graph|sequenceDiagram|classDiagram|stateDiagram(-v2)?|erDiagram|gantt|pie|gitGraph|mindmap|timeline|architecture-beta|xychart-beta|quadrantChart|kanban|block-beta)\b/i
        )
      ) {
        return 'keyword';
      }

      // Estructuras y palabras clave del flujo
      if (
        stream.match(
          /^(TD|TB|BT|RL|LR|subgraph|end|click|call|style|classDef|class|linkStyle|participant|actor|boundary|control|entity|database|collections|queue|box|autonumber|activate|deactivate|loop|alt|else|opt|par|critical|option|break|rect|note|over)\b/i
        )
      ) {
        return 'typeName';
      }

      // Flechas y conectores
      if (stream.match(/^(-->|---|==>|-.->|->>|-->>|x--x|o--o|--|->|<-)/)) {
        return 'operator';
      }

      // Texto entre comillas o corchetes
      if (stream.match(/^"[^"]*"/)) {
        return 'string';
      }

      // Nombres de nodos e identificadores
      if (stream.match(/^[a-zA-Z_][a-zA-Z0-9_]*/)) {
        return 'variableName';
      }

      stream.next();
      return null;
    },
  });

  // Tema de colores de sintaxis (Syntax Highlighting)
  const customHighlightStyle = HighlightStyle.define([
    { tag: tags.heading, color: 'var(--accent, #0969da)', fontWeight: 'bold' },
    { tag: tags.emphasis, fontStyle: 'italic' },
    { tag: tags.strong, fontWeight: 'bold' },
    { tag: tags.keyword, color: 'var(--syntax-keyword, #cf222e)', fontWeight: 'bold' },
    { tag: tags.comment, color: 'var(--syntax-comment, #6e7781)', fontStyle: 'italic' },
    { tag: tags.string, color: 'var(--syntax-string, #0a3069)' },
    { tag: tags.variableName, color: 'var(--syntax-variable, #953800)' },
    { tag: tags.typeName, color: 'var(--syntax-type, #116329)', fontWeight: 'bold' },
    { tag: tags.link, color: 'var(--accent, #0969da)', textDecoration: 'underline' },
    { tag: tags.url, color: 'var(--accent, #0969da)' },
    { tag: tags.number, color: 'var(--syntax-number, #0550ae)' },
    { tag: tags.operator, color: 'var(--syntax-number, #0550ae)', fontWeight: 'bold' },
    { tag: tags.meta, color: 'var(--text-secondary, #8c959f)' },
    // Claves de YAML/JSON y literales
    { tag: [tags.propertyName, tags.definition(tags.propertyName)], color: 'var(--syntax-variable, #953800)' },
    { tag: [tags.bool, tags.null, tags.atom], color: 'var(--syntax-keyword, #cf222e)' },
  ]);

  function buildDiagnostics(view: any, diagList: EditorDiagnostic[]): Diagnostic[] {
    const doc = view.state.doc;
    const result: Diagnostic[] = [];

    for (const d of diagList) {
      const lineNum = Math.min(Math.max(1, d.line), doc.lines);
      const lineObj = doc.line(lineNum);
      const from = Math.min(lineObj.from + Math.max(0, d.column - 1), lineObj.to);

      let to = from + 1;
      if (d.endLine && d.endColumn) {
        const endLineNum = Math.min(Math.max(1, d.endLine), doc.lines);
        const endLineObj = doc.line(endLineNum);
        to = Math.min(endLineObj.from + Math.max(0, d.endColumn - 1), endLineObj.to);
      } else {
        to = Math.max(from + 1, lineObj.to);
      }

      if (to <= from) to = Math.min(from + 1, doc.length);

      result.push({
        from,
        to,
        severity: d.severity || 'error',
        message: d.message,
      });
    }

    return result;
  }

  onMount(() => {
    if (!containerRef) return;

    const updateListener = EditorView.updateListener.of((update) => {
      if (update.docChanged && !isInternalUpdate) {
        const newText = update.state.doc.toString();
        lastContent = newText;
        if (onChange) onChange(newText);
      }
      if (update.selectionSet || update.docChanged) {
        emitSelectionInfo(update.state);
      }
    });

    const theme = EditorView.theme({
      '&': {
        fontSize: 'calc(13px * var(--content-zoom, 1))',
        fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace',
        color: 'var(--text-primary, #1f2328)',
      },
      // Solo en el editor: `&` también se aplica al contenedor de tooltips del body, que taparía la app
      '&.cm-editor': {
        height: '100%',
        width: '100%',
        backgroundColor: 'var(--bg-primary, #ffffff)',
      },
      '.cm-scroller': {
        overflow: 'auto',
        fontFamily: 'inherit',
      },
      '.cm-content': {
        padding: '8px 0',
      },
      '.cm-line': {
        padding: '0 8px',
      },
      '.cm-gutters': {
        backgroundColor: 'var(--bg-secondary, #f6f8fa)',
        color: 'var(--text-secondary, #656d76)',
        borderRight: '1px solid var(--border-primary, #d0d7de)',
      },
      '.cm-activeLine': {
        backgroundColor: 'rgba(9, 105, 218, 0.05)',
      },
      '.cm-activeLineGutter': {
        backgroundColor: 'rgba(9, 105, 218, 0.1)',
        color: 'var(--accent, #0969da)',
      },
      // Sin esto el cursor es negro (valor por defecto de CodeMirror) y no se ve en tema oscuro.
      '.cm-cursor, .cm-dropCursor': {
        borderLeftColor: 'var(--text-primary, #1f2328)',
      },
      '.cm-panels': {
        backgroundColor: 'var(--bg-secondary, #f6f8fa)',
        color: 'var(--text-primary, #1f2328)',
      },
      '.cm-panels-bottom': {
        borderTop: '1px solid var(--border-primary, #d0d7de)',
      },
      // Búsqueda de Vim (`/`): sin esto el campo hereda el fondo claro por defecto de CodeMirror y
      // en tema oscuro el texto (blanco) no se ve.
      '.cm-vim-panel': {
        backgroundColor: 'var(--bg-secondary, #f6f8fa)',
        color: 'var(--text-primary, #1f2328)',
        fontFamily: 'var(--code-font, monospace)',
        padding: '2px 10px',
      },
      '.cm-vim-panel input': {
        color: 'inherit',
        font: 'inherit',
        caretColor: 'var(--accent, #0969da)',
        flex: '1',
        minWidth: '0',
      },
      '.cm-panel.cm-search': {
        backgroundColor: 'var(--bg-secondary, #f6f8fa)',
        color: 'var(--text-primary, #1f2328)',
        borderBottom: '1px solid var(--border-primary, #d0d7de)',
        padding: '6px 10px',
      },
      '.cm-textfield': {
        backgroundColor: 'var(--bg-primary, #ffffff)',
        color: 'var(--text-primary, #1f2328)',
        border: '1px solid var(--border-primary, #d0d7de)',
        borderRadius: '4px',
        padding: '2px 6px',
      },
      '.cm-button': {
        backgroundColor: 'var(--bg-primary, #ffffff)',
        color: 'var(--text-primary, #1f2328)',
        border: '1px solid var(--border-primary, #d0d7de)',
        borderRadius: '4px',
        backgroundImage: 'none',
        padding: '2px 8px',
        cursor: 'pointer',
      },
      '.cm-button:hover': {
        backgroundColor: 'rgba(9, 105, 218, 0.1)',
        borderColor: 'var(--accent, #0969da)',
      },
      '&.cm-focused': {
        outline: 'none',
      },
    });

    const langExtension =
      mode === 'd2'
        ? [withCommentTokens(d2Language, { line: '#' }), braceFolding({ lineComment: '#' })]
        : mode === 'mermaid'
          ? [withCommentTokens(mermaidLanguage, { line: '%%' }), braceFolding({ lineComment: '%%' })]
          : mode === 'plantuml'
            ? [withCommentTokens(plantumlLanguage, { line: "'" }), braceFolding({ lineComment: "'" })]
            : mode === 'yaml'
              ? yaml()
              : mode === 'json'
                ? json()
                : mode === 'sql'
                  ? sql()
                  : markdown();

    const autocompleteExtension =
      mode === 'd2'
        ? autocompletion({ override: [withDocumentWords(d2Autocomplete)] })
        : mode === 'mermaid'
          ? autocompletion({ override: [withDocumentWords(mermaidAutocomplete)] })
          : mode === 'plantuml'
            ? autocompletion({ override: [withDocumentWords(plantumlAutocomplete)] })
            : autocompletion({ override: [documentWordsCompletion] });

    const currentDiag = diagnostics;
    const linterExtension = linter((view) => buildDiagnostics(view, currentDiag));

    const state = EditorState.create({
      doc: content,
      extensions: [
        vimCompartment.of(vimMode ? vim() : []),
        lintCompartment.of([linterExtension, lintGutter()]),
        // En el body: dentro del editor los contenedores de la vista recortan o tapan los tooltips
        tooltips({ parent: document.body }),
        basicSetup,
        gitDiffGutter(gitBaseContent),
        undoRedoExtension(),
        commentExtension(),
        langExtension,
        autocompleteExtension,
        syntaxHighlighting(customHighlightStyle),
        // Textos del plegado en español
        EditorState.phrases.of({
          'Fold line': 'Plegar',
          'Unfold line': 'Desplegar',
          'folded code': 'código plegado',
          unfold: 'desplegar',
        }),
        search({ top: true }),
        keymap.of(searchKeymap as any),
        renameOccurrencesKeymap(),
        lineKeymap(),
        mode === 'yaml' || mode === 'json'
          ? jsonRefLinks({
              getPath: () => filePath,
              openRef: (path, pointer) => onOpenRef?.(path, pointer),
              notify: (m) => onNotify?.(m),
            })
          : [],
        Prec.highest(keymap.of([{ key: 'Mod-Enter', run: () => (onRun ? (onRun(), true) : false) }])),
        editableCompartment.of(EditorState.readOnly.of(readOnly)),
        updateListener,
        theme,
      ],
    });

    editorView = new EditorView({
      state,
      parent: containerRef,
    });

    emitSelectionInfo(editorView.state);

    return () => {
      if (editorView) {
        editorView.destroy();
        editorView = null;
      }
    };
  });

  // Salto pendiente a un `$ref` de otro archivo: cuando el contenido ya está en el editor
  $effect(() => {
    const ptr = jumpPointer;
    const text = content;
    if (!ptr || !editorView || !text) return;
    queueMicrotask(() => {
      if (!editorView) return;
      if (!jumpToPointer(editorView, ptr)) onNotify?.(`No se encontró «#${ptr}» en este archivo`);
      onJumped?.();
    });
  });

  $effect(() => {
    const base = gitBaseContent;
    editorView?.dispatch({ effects: setGitBaseContent.of(base) });
  });

  $effect(() => {
    const ro = readOnly;
    editorView?.dispatch({ effects: editableCompartment.reconfigure(EditorState.readOnly.of(ro)) });
  });

  $effect(() => {
    const isVim = vimMode;
    if (editorView) {
      editorView.dispatch({
        effects: vimCompartment.reconfigure(isVim ? vim() : []),
      });
    }
  });

  $effect(() => {
    const diagList = diagnostics;
    if (editorView) {
      const newLinter = linter((view) => buildDiagnostics(view, diagList));
      editorView.dispatch({
        effects: lintCompartment.reconfigure([newLinter, lintGutter()]),
      });
    }
  });

  // Al volver a la pestaña, el foco vuelve al editor; CodeMirror restaura su selección.
  $effect(() => {
    if (!isActive || readOnly) return;
    requestAnimationFrame(() => {
      if (editorView && !editorView.hasFocus && containerRef?.offsetParent) editorView.focus();
    });
  });

  $effect(() => {
    const c = content;
    if (editorView && c !== untrack(() => lastContent)) {
      lastContent = c;
      if (c !== editorView.state.doc.toString()) {
        isInternalUpdate = true;
        editorView.dispatch({ changes: minimalChange(editorView.state.doc.toString(), c) });
        isInternalUpdate = false;
        emitSelectionInfo(editorView.state);
      }
    }
  });
</script>

<div class="codemirror-container" bind:this={containerRef}></div>

<style>
  .codemirror-container {
    width: 100%;
    height: 100%;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
