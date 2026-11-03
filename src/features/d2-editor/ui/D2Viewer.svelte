<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { DiagramSplitViewer } from '@shared/ui/diagram-viewer';
  import { createRenderScheduler, ensureD2, renderD2Svg, type D2SyntaxError } from '@shared/lib/diagrams';
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
      await ensureD2();
      isReady = true;
    } catch (e) {
      error = 'Error al cargar motor D2 (WASM): ' + String(e);
    }
  });

  async function renderCode(rawCode: string) {
    try {
      const res = await renderD2Svg(rawCode);
      if (res.error) {
        error = res.error;
        diagnostics = res.errors.map((e: D2SyntaxError) => ({
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

  const scheduler = createRenderScheduler('D2', renderCode);
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
  engineName="D2"
  engineTooltip="Visualizador D2 basado en WebAssembly (D2 v0.9 + TALA)"
  mode="d2"
  {svgContent}
  {isReady}
  {error}
  {diagnostics}
  {readOnly}
  {showEditor}
  {vimMode}
  onRun={() => scheduler.now(content)}
  {isActive}
  loadingMessage="Cargando motor D2 WASM..."
  {onChange}
  {onSelectionChange}
/>
