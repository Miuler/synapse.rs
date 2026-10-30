<script lang="ts">
  import { onMount } from 'svelte';
  import { DiagramSplitViewer } from '@shared/ui/diagram-viewer';
  import { initMerman, renderMermaidSvg, validate, parseMermanErrors } from '@shared/lib/diagrams';
  import type { EditorDiagnostic } from '@shared/ui/code-editor';

  interface Props {
    content: string;
    filePath?: string;
    readOnly?: boolean;
    showEditor?: boolean;
    vimMode?: boolean;
    onChange?: (content: string) => void;
    onSelectionChange?: (info: any) => void;
  }

  let {
    content = '',
    filePath,
    readOnly = false,
    showEditor = false,
    vimMode = false,
    onChange,
    onSelectionChange,
  }: Props = $props();

  let svgContent = $state('');
  let isReady = $state(false);
  let error = $state('');
  let diagnostics = $state<EditorDiagnostic[]>([]);

  onMount(async () => {
    try {
      await initMerman();
      isReady = true;
    } catch (e) {
      error = 'Error al cargar merman: ' + String(e);
    }
  });

  $effect(() => {
    if (isReady && content && content.trim()) {
      try {
        const valRes = validate(content);
        if (!valRes.valid) {
          error = valRes.error || `Error de sintaxis (${valRes.code_name || 'MERMAN_PARSE_ERROR'})`;
          const errs = parseMermanErrors(valRes);
          diagnostics = errs.map((e) => ({
            line: e.line,
            column: e.column,
            message: e.message,
            severity: 'error' as const,
          }));
        } else {
          const res = renderMermaidSvg(content);
          if (res.error) {
            error = res.error;
            diagnostics = res.errors.map((e) => ({
              line: e.line,
              column: e.column,
              message: e.message,
              severity: 'error' as const,
            }));
          } else {
            error = '';
            diagnostics = [];
            svgContent = res.svg || '';
          }
        }
      } catch (e: any) {
        error = String(e?.message || e);
        diagnostics = [{ line: 1, column: 1, message: error, severity: 'error' }];
      }
    } else if (isReady && (!content || !content.trim())) {
      svgContent = '';
      error = '';
      diagnostics = [];
    }
  });
</script>

<DiagramSplitViewer
  {content}
  {filePath}
  engineName="Merman"
  engineTooltip="Visualizador Merman basado en Rust y WebAssembly"
  mode="mermaid"
  {svgContent}
  {isReady}
  {error}
  {diagnostics}
  {readOnly}
  {showEditor}
  {vimMode}
  loadingMessage="Cargando Merman WASM..."
  {onChange}
  {onSelectionChange}
/>
