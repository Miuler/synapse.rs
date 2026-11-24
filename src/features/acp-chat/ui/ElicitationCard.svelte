<script lang="ts">
  import { untrack } from 'svelte';
  import { ExternalLink, HelpCircle, Globe } from 'lucide-svelte';
  import type {
    ElicitationAction,
    ElicitationAnswer,
    ElicitationPropertySchema,
    ElicitationRequestEvent,
  } from '@entities/acp-agent';

  interface Props {
    request: ElicitationRequestEvent;
    answered: ElicitationAnswer | null;
    onRespond: (action: ElicitationAction, content?: Record<string, unknown> | null) => void;
  }

  let { request, answered, onRespond }: Props = $props();

  const properties = $derived(Object.entries(request.requestedSchema?.properties ?? {}));
  const requiredSet = $derived(new Set(request.requestedSchema?.required ?? []));

  function enumOptionsOf(prop: ElicitationPropertySchema): { value: string | number | boolean; label: string }[] {
    if (prop.oneOf?.length) {
      return prop.oneOf.map((o) => ({ value: o.const, label: o.title ?? String(o.const) }));
    }
    if (prop.enum?.length) {
      return prop.enum.map((v, i) => ({ value: v, label: prop.enumNames?.[i] ?? String(v) }));
    }
    return [];
  }

  function arrayOptionsOf(prop: ElicitationPropertySchema): { value: string; label: string }[] {
    if (prop.items?.anyOf?.length) {
      return prop.items.anyOf.map((o) => ({ value: o.const, label: o.title ?? o.const }));
    }
    if (prop.items?.enum?.length) {
      return prop.items.enum.map((v) => ({ value: v, label: v }));
    }
    return [];
  }

  function initialValues(): Record<string, unknown> {
    const out: Record<string, unknown> = {};
    const props = untrack(() => request.requestedSchema?.properties ?? {});
    for (const [key, prop] of Object.entries(props)) {
      if (prop.default !== undefined) {
        out[key] = prop.default;
      } else if (prop.type === 'boolean') {
        out[key] = false;
      } else if (prop.type === 'array') {
        out[key] = [];
      } else {
        out[key] = '';
      }
    }
    return out;
  }

  let values = $state<Record<string, unknown>>(initialValues());

  const parsedDomain = $derived.by(() => {
    if (!request.url) return null;
    try {
      return new URL(request.url).host;
    } catch {
      return null;
    }
  });

  const canAcceptForm = $derived.by(() => {
    if (request.mode !== 'form') return true;
    for (const [key, prop] of properties) {
      if (!requiredSet.has(key)) continue;
      const val = values[key];
      if (prop.type === 'boolean') continue;
      if (prop.type === 'array') {
        if (!Array.isArray(val) || val.length === 0) return false;
      } else if (val === undefined || val === null || String(val).trim() === '') {
        return false;
      }
    }
    return true;
  });

  function submitForm(e: Event) {
    e.preventDefault();
    if (!canAcceptForm) return;
    const content: Record<string, unknown> = {};
    for (const [key, prop] of properties) {
      const val = values[key];
      if (prop.type === 'boolean') {
        content[key] = Boolean(val);
      } else if (prop.type === 'number' || prop.type === 'integer') {
        if (val !== '' && val !== null && val !== undefined) {
          const n = Number(val);
          if (!Number.isNaN(n)) content[key] = prop.type === 'integer' ? Math.trunc(n) : n;
        }
      } else if (prop.type === 'array') {
        content[key] = Array.isArray(val) ? val : [];
      } else if (val !== '' && val !== null && val !== undefined) {
        content[key] = val;
      }
    }
    onRespond('accept', content);
  }

  function toggleArrayOption(key: string, val: string) {
    const cur = Array.isArray(values[key]) ? (values[key] as string[]) : [];
    values[key] = cur.includes(val) ? cur.filter((x) => x !== val) : [...cur, val];
  }

  function inputTypeOf(prop: ElicitationPropertySchema): string {
    if (prop.format === 'password') return 'password';
    if (prop.format === 'email') return 'email';
    if (prop.format === 'uri' || prop.format === 'url') return 'url';
    if (prop.format === 'date') return 'date';
    return 'text';
  }

  function openExternalUrl() {
    if (request.url && /^https?:/i.test(request.url)) {
      window.open(request.url, '_blank', 'noopener');
    }
  }

  function answeredSummary(ans: ElicitationAnswer): string {
    if (ans.action === 'decline') return 'Rechazado';
    if (ans.action === 'cancel') return 'Cancelado';
    if (request.mode === 'url') return 'Confirmado';
    const entries = Object.entries(ans.content ?? {});
    if (!entries.length) return 'Aceptado';
    return `Aceptado: ${entries.map(([k, v]) => `${k}=${Array.isArray(v) ? v.join(', ') : String(v)}`).join(' · ')}`;
  }
</script>

<div class="acp-elicitation" class:answered={answered !== null} data-mode={request.mode}>
  <div class="acp-elicitation-title">
    {#if request.mode === 'url'}
      <Globe size={14} />
    {:else}
      <HelpCircle size={14} />
    {/if}
    <span>{request.message}</span>
  </div>

  {#if answered === null}
    {#if request.mode === 'url'}
      {#if request.url}
        <div class="acp-elicitation-url-box">
          {#if parsedDomain}
            <span class="acp-elicitation-domain">{parsedDomain}</span>
          {/if}
          <span class="acp-elicitation-url" title={request.url}>{request.url}</span>
          {#if /^https?:/i.test(request.url)}
            <button type="button" class="acp-btn" onclick={openExternalUrl}>
              <ExternalLink size={12} /> Abrir enlace
            </button>
          {/if}
        </div>
      {/if}
      <div class="acp-elicitation-actions">
        <button type="button" class="acp-btn primary" onclick={() => onRespond('accept')}>Confirmar</button>
        <button type="button" class="acp-btn danger" onclick={() => onRespond('decline')}>Rechazar</button>
        <button type="button" class="acp-btn" onclick={() => onRespond('cancel')}>Cancelar</button>
      </div>
    {:else}
      <form class="acp-elicitation-form" onsubmit={submitForm}>
        {#each properties as [key, prop] (key)}
          {@const opts = enumOptionsOf(prop)}
          {@const arrOpts = arrayOptionsOf(prop)}
          {@const isReq = requiredSet.has(key)}
          <div class="acp-elicitation-field" data-field={key}>
            {#if prop.type === 'boolean'}
              <label class="acp-elicitation-check">
                <input
                  type="checkbox"
                  checked={Boolean(values[key])}
                  onchange={(e) => (values[key] = e.currentTarget.checked)}
                />
                <span>{prop.title ?? key}{#if isReq}<span class="req">*</span>{/if}</span>
              </label>
              {#if prop.description}
                <div class="acp-elicitation-desc">{prop.description}</div>
              {/if}
            {:else}
              <label class="acp-elicitation-label" for="el-{request.requestId}-{key}">
                {prop.title ?? key}{#if isReq}<span class="req">*</span>{/if}
              </label>
              {#if prop.description}
                <div class="acp-elicitation-desc">{prop.description}</div>
              {/if}
              {#if opts.length > 0 && opts.length <= 5}
                <div class="acp-elicitation-pills" role="radiogroup" aria-label={prop.title ?? key}>
                  {#each opts as opt (String(opt.value))}
                    <button
                      type="button"
                      class="acp-elicitation-pill"
                      class:selected={values[key] === opt.value}
                      onclick={() => (values[key] = opt.value)}
                    >
                      {opt.label}
                    </button>
                  {/each}
                </div>
              {:else if opts.length > 5}
                <select
                  id="el-{request.requestId}-{key}"
                  class="acp-elicitation-input"
                  value={String(values[key] ?? '')}
                  onchange={(e) => {
                    const matched = opts.find((o) => String(o.value) === e.currentTarget.value);
                    values[key] = matched ? matched.value : e.currentTarget.value;
                  }}
                >
                  <option value="">Seleccionar…</option>
                  {#each opts as opt (String(opt.value))}
                    <option value={String(opt.value)}>{opt.label}</option>
                  {/each}
                </select>
              {:else if prop.type === 'array' && arrOpts.length > 0}
                <div class="acp-elicitation-pills">
                  {#each arrOpts as opt (opt.value)}
                    <button
                      type="button"
                      class="acp-elicitation-pill"
                      class:selected={Array.isArray(values[key]) && (values[key] as string[]).includes(opt.value)}
                      onclick={() => toggleArrayOption(key, opt.value)}
                    >
                      {opt.label}
                    </button>
                  {/each}
                </div>
              {:else if prop.type === 'number' || prop.type === 'integer'}
                <input
                  id="el-{request.requestId}-{key}"
                  type="number"
                  step={prop.type === 'integer' ? '1' : 'any'}
                  min={prop.minimum}
                  max={prop.maximum}
                  class="acp-elicitation-input"
                  value={String(values[key] ?? '')}
                  oninput={(e) => (values[key] = e.currentTarget.value)}
                  required={isReq}
                />
              {:else}
                <input
                  id="el-{request.requestId}-{key}"
                  type={inputTypeOf(prop)}
                  minlength={prop.minLength}
                  maxlength={prop.maxLength}
                  class="acp-elicitation-input"
                  value={String(values[key] ?? '')}
                  oninput={(e) => (values[key] = e.currentTarget.value)}
                  required={isReq}
                />
              {/if}
            {/if}
          </div>
        {/each}

        <div class="acp-elicitation-actions">
          <button type="submit" class="acp-btn primary" disabled={!canAcceptForm}>Aceptar</button>
          <button type="button" class="acp-btn danger" onclick={() => onRespond('decline')}>Rechazar</button>
          <button type="button" class="acp-btn" onclick={() => onRespond('cancel')}>Cancelar</button>
        </div>
      </form>
    {/if}
  {:else}
    <div class="acp-elicitation-done">{answeredSummary(answered)}</div>
  {/if}
</div>

<style>
  .acp-elicitation {
    border: 1px solid color-mix(in srgb, var(--accent) 45%, var(--border-primary));
    background: var(--bg-secondary);
    border-radius: 6px;
    padding: 9px 11px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 12.5px;
  }

  .acp-elicitation.answered {
    opacity: 0.72;
    border-color: var(--border-primary);
  }

  .acp-elicitation-title {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.35;
  }

  .acp-elicitation-title :global(svg) {
    flex-shrink: 0;
    margin-top: 2px;
    color: var(--accent);
  }

  .acp-elicitation-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .acp-elicitation-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .acp-elicitation-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .acp-elicitation-label .req,
  .acp-elicitation-check .req {
    color: var(--danger, #e5484d);
    margin-left: 2px;
  }

  .acp-elicitation-desc {
    font-size: 11px;
    color: var(--text-muted);
  }

  .acp-elicitation-input {
    width: 100%;
    box-sizing: border-box;
    padding: 5px 8px;
    border-radius: 4px;
    border: 1px solid var(--border-primary);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 12px;
    font-family: inherit;
    outline: none;
  }

  .acp-elicitation-input:focus {
    border-color: var(--accent);
  }

  .acp-elicitation-check {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-primary);
    cursor: pointer;
  }

  .acp-elicitation-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }

  .acp-elicitation-pill {
    padding: 3px 9px;
    border-radius: 999px;
    border: 1px solid var(--border-primary);
    background: var(--bg-primary);
    color: var(--text-secondary);
    font-size: 11.5px;
    cursor: pointer;
  }

  .acp-elicitation-pill:hover {
    border-color: var(--accent);
    color: var(--text-primary);
  }

  .acp-elicitation-pill.selected {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .acp-elicitation-url-box {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    border-radius: 4px;
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
  }

  .acp-elicitation-domain {
    font-weight: 600;
    font-size: 11.5px;
    color: var(--accent);
    padding: 1px 6px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }

  .acp-elicitation-url {
    font-size: 11.5px;
    color: var(--text-secondary);
    word-break: break-all;
    flex: 1 1 140px;
  }

  .acp-elicitation-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 2px;
  }

  .acp-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    border-radius: 4px;
    border: 1px solid var(--border-primary);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    font-size: 12px;
    cursor: pointer;
  }

  .acp-btn:hover:not(:disabled) {
    border-color: var(--accent);
  }

  .acp-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .acp-btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .acp-btn.danger {
    color: var(--danger, #e5484d);
    border-color: color-mix(in srgb, var(--danger, #e5484d) 45%, var(--border-primary));
  }

  .acp-elicitation-done {
    font-size: 11.5px;
    color: var(--text-muted);
  }
</style>
