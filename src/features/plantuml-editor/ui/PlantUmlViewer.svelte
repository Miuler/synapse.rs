<script lang="ts">
  import { onDestroy } from 'svelte';
  import { DiagramSplitViewer } from '@shared/ui/diagram-viewer';
  import { createRenderScheduler, renderPlantUmlSvg } from '@shared/lib/diagrams';
  import { appSettings } from '@entities/settings';
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
  let error = $state('');
  let diagnostics = $state<EditorDiagnostic[]>([]);

  const server = $derived(appSettings.settings.plantumlServer);

  async function renderCode(rawCode: string) {
    const res = await renderPlantUmlSvg(rawCode, server);
    if (res.error) {
      error = res.error;
      diagnostics = res.errors.map((e) => ({ ...e, severity: 'error' as const }));
      return;
    }
    error = '';
    diagnostics = [];
    svgContent = res.svg || '';
  }

  const scheduler = createRenderScheduler('PlantUML', renderCode);
  onDestroy(() => scheduler.destroy());

  $effect(() => {
    const rawCode = content;
    if (rawCode && rawCode.trim()) {
      scheduler.schedule(rawCode);
    } else {
      svgContent = '';
      error = '';
      diagnostics = [];
    }
  });
</script>

<DiagramSplitViewer
  {content}
  {filePath}
  engineName="PlantUML"
  engineTooltip={`Se renderiza en el servidor ${server}: el código del diagrama se envía a ese servidor`}
  mode="plantuml"
  {svgContent}
  isReady={true}
  {error}
  {diagnostics}
  {readOnly}
  {showEditor}
  {vimMode}
  onRun={() => scheduler.now(content)}
  {isActive}
  loadingMessage="Cargando PlantUML..."
  {onChange}
  {onSelectionChange}
/>
