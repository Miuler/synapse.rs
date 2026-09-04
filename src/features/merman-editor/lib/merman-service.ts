import { initMerman, renderSvg, validate } from '@mermanjs/web';

let initPromise: Promise<unknown> | null = null;

export async function ensureMerman(): Promise<void> {
  if (!initPromise) {
    initPromise = initMerman().catch((err) => {
      initPromise = null;
      throw err;
    });
  }
  await initPromise;
}

export function renderMermaidSvg(code: string): { svg: string | null; error: string | null } {
  try {
    const val = validate(code);
    if (!val.valid) {
      return { svg: null, error: val.error || `Error de sintaxis (${val.code_name || 'MERMAN_PARSE_ERROR'})` };
    }
    const svg = renderSvg(code);
    return { svg, error: null };
  } catch (e: any) {
    return { svg: null, error: String(e?.message || e) };
  }
}

export { initMerman, renderSvg, validate };
