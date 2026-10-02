<script lang="ts">
  import {
    File,
    FileText,
    FileCode,
    FileImage,
    FileJson,
    GitBranch,
    Workflow,
    Binary,
    FileCog,
    type IconProps
  } from 'lucide-svelte';

  interface Props {
    path?: string;
    name?: string;
    extension?: string;
    size?: number;
    strokeWidth?: number;
    class?: string;
  }

  let {
    path = '',
    name = '',
    extension = '',
    size = 15,
    strokeWidth = 2,
    class: className = ''
  }: Props = $props();

  // Extraer extensión y nombre si no se proporcionaron directamente
  const fileName = $derived((name || path.split('/').pop() || '').toLowerCase());
  const fileExt = $derived((extension || fileName.split('.').pop() || '').toLowerCase());

  // Clasificador de tipos de archivo para theming y variantes de icono
  type FileCategory = 'markdown' | 'rust' | 'code' | 'image' | 'json' | 'config' | 'diagram' | 'git' | 'binary' | 'default';

  const category = $derived((): FileCategory => {
    if (fileName === '.gitignore' || fileName === '.gitmodules') return 'git';
    if (fileName.endsWith('.lock') || fileName.endsWith('.lockb')) return 'config';

    switch (fileExt) {
      case 'md':
      case 'markdown':
      case 'txt':
        return 'markdown';
      case 'rs':
        return 'rust';
      case 'ts':
      case 'js':
      case 'svelte':
      case 'html':
      case 'css':
      case 'py':
      case 'go':
      case 'c':
      case 'cpp':
      case 'sh':
      case 'bash':
        return 'code';
      case 'png':
      case 'jpg':
      case 'jpeg':
      case 'gif':
      case 'webp':
      case 'svg':
      case 'bmp':
      case 'ico':
        return 'image';
      case 'json':
        return 'json';
      case 'toml':
      case 'yaml':
      case 'yml':
      case 'xml':
      case 'ini':
      case 'env':
        return 'config';
      case 'mermaid':
      case 'mmd':
      case 'merman':
      case 'excalidraw':
      case 'canvas':
        return 'diagram';
      case 'wasm':
      case 'bin':
      case 'exe':
      case 'tar':
      case 'gz':
      case 'zip':
        return 'binary';
      default:
        return 'default';
    }
  });
</script>

{#if category() === 'markdown'}
  <FileText {size} {strokeWidth} class="synapse-file-icon synapse-file-md {className}" />
{:else if category() === 'rust'}
  <FileCode {size} {strokeWidth} class="synapse-file-icon synapse-file-rs {className}" />
{:else if category() === 'code'}
  <FileCode {size} {strokeWidth} class="synapse-file-icon synapse-file-code {className}" />
{:else if category() === 'image'}
  <FileImage {size} {strokeWidth} class="synapse-file-icon synapse-file-image {className}" />
{:else if category() === 'json'}
  <FileJson {size} {strokeWidth} class="synapse-file-icon synapse-file-json {className}" />
{:else if category() === 'config'}
  <FileCog {size} {strokeWidth} class="synapse-file-icon synapse-file-config {className}" />
{:else if category() === 'diagram'}
  <Workflow {size} {strokeWidth} class="synapse-file-icon synapse-file-diagram {className}" />
{:else if category() === 'git'}
  <GitBranch {size} {strokeWidth} class="synapse-file-icon synapse-file-git {className}" />
{:else if category() === 'binary'}
  <Binary {size} {strokeWidth} class="synapse-file-icon synapse-file-binary {className}" />
{:else}
  <File {size} {strokeWidth} class="synapse-file-icon synapse-file-default {className}" />
{/if}

<style>
  :global(.synapse-file-icon) {
    flex-shrink: 0;
    display: inline-block;
    vertical-align: middle;
    color: var(--icon-file-color, var(--accent, #0969da));
    transition: color 0.15s ease;
  }

  :global(.synapse-file-icon.synapse-file-rs) {
    color: var(--icon-file-rs-color, #dea584);
  }

  :global(.synapse-file-icon.synapse-file-image) {
    color: var(--icon-file-image-color, #2da44e);
  }

  :global(.synapse-file-icon.synapse-file-diagram) {
    color: var(--icon-file-diagram-color, #8250df);
  }

  :global(.synapse-file-icon.synapse-file-config) {
    color: var(--icon-file-config-color, #6e7781);
  }

  :global(.synapse-file-icon.synapse-file-json) {
    color: var(--icon-file-json-color, #bf8700);
  }

  :global(.synapse-file-icon.synapse-file-git) {
    color: var(--icon-file-git-color, #fd8c73);
  }
</style>
