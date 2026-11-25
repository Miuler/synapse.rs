<script lang="ts">
  // Busca en lo dibujado dentro de un elemento (chat del agente, documento de Word), sin tocar su DOM
  import { paintMatches, revealRange, searchRanges } from '@shared/lib/text-search';
  import { untrack, type Snippet } from 'svelte';
  import FindBar from './FindBar.svelte';

  interface Props {
    scroller: HTMLElement;
    request: number;
    onClose: () => void;
    placeholder?: string;
    /** Texto que no se mira; si cambia, se vuelve a buscar. */
    skip?: (parent: Element) => boolean;
    /** Empieza por la última coincidencia (un chat) en vez de la primera. */
    fromEnd?: boolean;
    extra?: Snippet;
  }
  let { scroller, request, onClose, placeholder, skip, fromEnd = false, extra }: Props = $props();

  let query = $state('');
  let bar = $state<ReturnType<typeof FindBar> | null>(null);
  let matches = $state.raw<Range[]>([]);
  let index = $state(0);
  /** Pliegues abiertos para mostrar una coincidencia: se vuelven a cerrar al pasar a otra. */
  let opened: HTMLDetailsElement[] = [];
  /** Distingue lo resaltado aquí de lo de otra barra abierta a la vez. */
  const owner = {};

  export function focus() {
    bar?.focus();
  }

  function show(i: number) {
    index = i;
    const range = matches[i] ?? null;
    const keep: HTMLDetailsElement[] = [];
    if (range) {
      // Pliegues cerrados: se abre el que contiene la coincidencia
      const inSummary = range.startContainer.parentElement?.closest('summary');
      let d = (inSummary?.parentElement?.parentElement ?? range.startContainer.parentElement)?.closest('details') ?? null;
      while (d && scroller.contains(d)) {
        if (!d.open) {
          d.open = true;
          keep.push(d);
        } else if (opened.includes(d)) keep.push(d);
        d = d.parentElement?.closest('details') ?? null;
      }
    }
    for (const d of opened) if (!keep.includes(d)) d.open = false;
    opened = keep;
    paintMatches(owner, matches, range);
    if (range) revealRange(scroller, range);
  }

  function step(by: number) {
    if (timer) run(true);
    else if (matches.length) show((index + by + matches.length) % matches.length);
  }

  let timer: ReturnType<typeof setTimeout> | undefined;
  /** `fresh`: búsqueda nueva; si no, se conserva la posición. */
  function run(fresh: boolean) {
    clearTimeout(timer);
    timer = undefined;
    matches = searchRanges(scroller, query, skip);
    const last = Math.max(0, matches.length - 1);
    if (fresh) return show(fromEnd ? last : 0);
    index = Math.min(index, last);
    paintMatches(owner, matches, matches[index] ?? null);
  }

  $effect(() => {
    void query;
    untrack(() => {
      clearTimeout(timer);
      timer = setTimeout(() => run(true), 120);
    });
  });
  let first = true;
  $effect(() => {
    void skip;
    if (first) first = false;
    else untrack(() => run(true));
  });

  // Contenido nuevo, o que se vuelve a dibujar, mientras se busca
  $effect(() => {
    let wait: ReturnType<typeof setTimeout> | undefined;
    const observer = new MutationObserver(() => {
      if (wait || !query || timer) return;
      wait = setTimeout(() => {
        wait = undefined;
        if (!timer) run(false);
      }, 400);
    });
    observer.observe(scroller, { childList: true, characterData: true, subtree: true });
    return () => {
      observer.disconnect();
      clearTimeout(wait);
      clearTimeout(timer);
      paintMatches(owner, [], null);
    };
  });
</script>

<FindBar bind:this={bar} bind:query count={matches.length} {index} {request} {placeholder} onStep={step} {onClose} {extra} />

<style>
  :global(::highlight(synapse-search)) {
    background-color: rgba(255, 223, 93, 0.45); /* fijo: resaltado de búsqueda, no depende del tema */
  }
  :global(::highlight(synapse-search-active)) {
    background-color: #ff9632; /* fijo: resaltado de búsqueda, no depende del tema */
    color: #000000; /* fijo: texto sobre resaltado naranja */
  }
</style>
