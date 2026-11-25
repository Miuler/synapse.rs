<script lang="ts">
  // Búsqueda en una conversación (Ctrl+F)
  import { t } from '@shared/lib/i18n';
  import { DomFindBar } from '@shared/ui/find-bar';
  import { Wrench } from 'lucide-svelte';

  interface Props {
    scroller: HTMLElement;
    request: number;
    onClose: () => void;
  }
  let { scroller, request, onClose }: Props = $props();

  /** Buscar también en la entrada y el resultado de las herramientas; se recuerda. */
  const TOOLS_KEY = 'synapse.agent.search-tools';
  let tools = $state(load());
  let bar = $state<ReturnType<typeof DomFindBar> | null>(null);

  function load() {
    try {
      return localStorage.getItem(TOOLS_KEY) === '1';
    } catch {
      return false;
    }
  }
  function toggleTools() {
    tools = !tools;
    try {
      localStorage.setItem(TOOLS_KEY, tools ? '1' : '0');
    } catch {
      /* sin almacenamiento */
    }
    bar?.focus();
  }
  // Sin el botón, de las herramientas solo se mira el título
  const inToolBody = (parent: Element) => !!parent.closest('.acp-tool') && !parent.closest('summary');
</script>

<DomFindBar
  bind:this={bar}
  {scroller}
  {request}
  {onClose}
  placeholder={t('chatSearch.placeholder')}
  skip={tools ? undefined : inToolBody}
  fromEnd
>
  {#snippet extra()}
    <button
      type="button"
      class="acp-search-tools"
      class:active={tools}
      aria-pressed={tools}
      title={t('chatSearch.tools')}
      aria-label={t('chatSearch.tools')}
      onclick={toggleTools}><Wrench size={13} /></button
    >
  {/snippet}
</DomFindBar>
