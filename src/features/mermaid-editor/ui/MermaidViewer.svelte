<script lang="ts">
  import { onMount } from 'svelte';
  import { DiagramSplitViewer } from '@shared/ui/diagram-viewer';
  import {
    ensureMermaidJs,
    renderMermaidJsSvg,
    validateMermaidJs,
    type MermaidSyntaxError,
  } from '@shared/lib/diagrams';
  import type { EditorDiagnostic } from '@shared/ui/code-editor';

  interface Props {
    content: string;
    readOnly?: boolean;
    showEditor?: boolean;
    vimMode?: boolean;
    onChange?: (content: string) => void;
    onSelectionChange?: (info: any) => void;
  }

  let {
    content = '',
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
  let currentRequestId = 0;

  onMount(async () => {
    try {
      await ensureMermaidJs();
      isReady = true;
    } catch (e) {
      error = 'Error al cargar Mermaid.js: ' + String(e);
    }
  });

  $effect(() => {
    const rawCode = content;
    if (isReady && rawCode && rawCode.trim()) {
      const requestId = ++currentRequestId;

      (async () => {
        try {
          const valRes = await validateMermaidJs(rawCode);
          if (requestId !== currentRequestId) return;

          if (!valRes.valid) {
            error = valRes.error || 'Error de sintaxis en diagrama Mermaid';
            diagnostics = valRes.errors.map((e: MermaidSyntaxError) => ({
              line: e.line,
              column: e.column,
              endLine: e.endLine,
              endColumn: e.endColumn,
              message: e.message,
              severity: 'error' as const,
            }));
            return;
          }

          const res = await renderMermaidJsSvg(rawCode, 'mermaidjs-viewer');
          if (requestId !== currentRequestId) return;

          if (res.error) {
            error = res.error;
            diagnostics = res.errors.map((e: MermaidSyntaxError) => ({
              line: e.line,
              column: e.column,
              endLine: e.endLine,
              endColumn: e.endColumn,
              message: e.message,
              severity: 'error' as const,
            }));
            return;
          }

          error = '';
          diagnostics = [];
          svgContent = res.svg || '';
        } catch (e: any) {
          if (requestId === currentRequestId) {
            error = String(e?.message || e);
            diagnostics = [{ line: 1, column: 1, message: error, severity: 'error' }];
          }
        }
      })();
    } else if (isReady && (!rawCode || !rawCode.trim())) {
      svgContent = '';
      error = '';
      diagnostics = [];
    }
  });
</script>

<DiagramSplitViewer
  {content}
  engineName="Mermaid.js"
  engineTooltip="Visualizador oficial de Mermaid.js"
  mode="mermaid"
  {svgContent}
  {isReady}
  {error}
  {diagnostics}
  {readOnly}
  {showEditor}
  {vimMode}
  loadingMessage="Cargando Mermaid.js..."
  {onChange}
  {onSelectionChange}
/>
