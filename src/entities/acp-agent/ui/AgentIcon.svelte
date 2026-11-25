<script module lang="ts">
  const files = import.meta.glob('../../../shared/assets/agents/*.svg', {
    query: '?url&no-inline',
    import: 'default',
    eager: true,
  }) as Record<string, string>;
  const urls = new Map(Object.entries(files).map(([path, url]) => [path.split('/').pop()!.replace('.svg', ''), url]));
</script>

<script lang="ts">
  import { Bot } from 'lucide-svelte';
  import { agentIconId, isColoredIcon } from '../lib/agent-icon';

  let { agentId, size = 14 }: { agentId: string; size?: number } = $props();

  const iconId = $derived(agentIconId(agentId));
  const url = $derived(urls.get(iconId));
</script>

<!-- Decorativo: el nombre del agente va al lado -->
{#if !url}
  <Bot class="agent-icon" {size} aria-hidden="true" />
{:else if isColoredIcon(iconId)}
  <img class="agent-icon" src={url} alt="" width={size} height={size} />
{:else}
  <span
    class="agent-icon mono"
    style:width="{size}px"
    style:height="{size}px"
    style:mask-image={`url("${url}")`}
    style:-webkit-mask-image={`url("${url}")`}
    aria-hidden="true"
  ></span>
{/if}

<style>
  .mono {
    display: inline-block;
    background: currentColor;
    -webkit-mask: center / contain no-repeat;
    mask: center / contain no-repeat;
  }
</style>
