<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { DiagramSplitViewer } from '@shared/ui/diagram-viewer';
  import {
    createRenderScheduler,
    ensureMermaidJs,
    renderMermaidJsSvg,
    validateMermaidJs,
    type MermaidSyntaxError,
  } from '@shared/lib/diagrams';
  import type { EditorDiagnostic } from '@shared/ui/code-editor';

  interface Props {
    content: string;
    filePath?: string;
    readOnly?: boolean;
    showEditor?: boolean;
    vimMode?: boolean;
    /** Pestaña activa: al volver a ella, el editor recupera el foco y su cursor. */
    isActive?: boolean;
    onChange?: (content: string) => void;
    onSelectionChange?: (info: any) => void;
  }

  let {
    content = '',
    filePath,
    readOnly = false,
    showEditor = false,
    vimMode = false,
    isActive = false,
    onChange,
    onSelectionChange,
  }: Props = $props();

  let svgContent = $state('');
  let isReady = $state(false);
  let error = $state('');
  let diagnostics = $state<EditorDiagnostic[]>([]);

  onMount(async () => {
    try {
      await ensureMermaidJs();
      isReady = true;
    } catch (e) {
      error = 'Error al cargar Mermaid.js: ' + String(e);
    }
  });

  async function renderCode(rawCode: string) {
    try {
      const valRes = await validateMermaidJs(rawCode);
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
      error = String(e?.message || e);
      diagnostics = [{ line: 1, column: 1, message: error, severity: 'error' }];
    }
  }

  const scheduler = createRenderScheduler('Mermaid', renderCode);
  onDestroy(() => scheduler.destroy());

  $effect(() => {
    const rawCode = content;
    if (isReady && rawCode && rawCode.trim()) {
      scheduler.schedule(rawCode);
    } else if (isReady && (!rawCode || !rawCode.trim())) {
      svgContent = '';
      error = '';
      diagnostics = [];
    }
  });
</script>

<DiagramSplitViewer
  {content}
  {filePath}
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
  onRun={() => scheduler.now(content)}
  {isActive}
  loadingMessage="Cargando Mermaid.js..."
  {onChange}
  {onSelectionChange}
/>
