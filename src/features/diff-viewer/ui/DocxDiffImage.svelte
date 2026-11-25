<script lang="ts">
  import { t } from '@shared/lib/i18n';
  import { ImageOff } from 'lucide-svelte';
  import { docxDiffApi, type DocxImage } from '../api/docx-diff.api';
  import type { DiffSource } from '../lib/diff-sides';

  interface Props {
    image: DocxImage;
    source: DiffSource;
  }
  let { image, source }: Props = $props();

  let url = $state<string | null>(null);
  let failed = $state(false);

  $effect(() => {
    const { sha256, mime } = image;
    const from = source;
    url = null;
    failed = false;
    if (!sha256 || !mime) return;
    let current = true;
    docxDiffApi
      .image(from, sha256)
      .then((u) => current && ((url = u), (failed = !u)))
      .catch(() => current && (failed = true));
    return () => (current = false);
  });
</script>

{#if url}
  <img class="docx-diff-img" src={url} alt={image.alt || image.filename} />
{:else if image.external || !image.mime || !image.sha256 || failed}
  <span class="docx-diff-noimg">
    <ImageOff size={14} />
    {image.external ? t('docxDiff.externalImage') : t('docxDiff.unsupportedImage')}
    <code>{image.filename}</code>
  </span>
{/if}

<style>
  .docx-diff-img {
    max-width: 100%;
    max-height: 420px;
    object-fit: contain;
    background: #fff; /* fijo: papel, no depende del tema */
  }
  .docx-diff-noimg {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }
</style>
