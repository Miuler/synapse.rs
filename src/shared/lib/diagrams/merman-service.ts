import { initMerman, renderSvg, validate, isMermanInitialized } from '@mermanjs/web';

export interface MermanSyntaxError {
  line: number;
  column: number;
  message: string;
}

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

export function parseMermanErrors(val: { valid: boolean; error?: string; code_name?: string }): MermanSyntaxError[] {
  if (val.valid || !val.error) return [];
  const rawMsg = val.error;
  const lineMatch = rawMsg.match(/(?:line|línea)\s+(\d+)(?::(\d+))?/i);
  if (lineMatch) {
    return [
      {
        line: parseInt(lineMatch[1], 10),
        column: lineMatch[2] ? parseInt(lineMatch[2], 10) : 1,
        message: rawMsg,
      },
    ];
  }
  return [
    {
      line: 1,
      column: 1,
      message: rawMsg,
    },
  ];
}

export function renderMermaidSvg(code: string): { svg: string | null; error: string | null; errors: MermanSyntaxError[] } {
  try {
    const val = validate(code);
    if (!val.valid) {
      const errors = parseMermanErrors(val);
      return { svg: null, error: val.error || `Error de sintaxis (${val.code_name || 'MERMAN_PARSE_ERROR'})`, errors };
    }
    const svg = renderSvg(code);
    return { svg, error: null, errors: [] };
  } catch (e: any) {
    const raw = String(e?.message || e);
    return {
      svg: null,
      error: raw,
      errors: [{ line: 1, column: 1, message: raw }],
    };
  }
}

export { initMerman, renderSvg, validate, isMermanInitialized };
