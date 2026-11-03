import DOMPurify from 'dompurify';
import { encode } from 'plantuml-encoder';
import { invokeTauri } from '@shared/repositories/tauri';

export interface PlantUmlSyntaxError {
  line: number; // 1-indexed
  column: number; // 1-indexed
  message: string;
}

export interface PlantUmlRenderResult {
  svg: string | null;
  error: string | null;
  errors: PlantUmlSyntaxError[];
}

/** Respuesta del comando `fetch_plantuml_svg`. */
export interface PlantUmlServerResponse {
  svg: string | null;
  error: string | null;
  error_line: number | null;
}

/** Añade `@startuml`/`@enduml` si faltan; `lineOffset` son las líneas añadidas antes del código. */
export function preparePlantUmlSource(code: string): { source: string; lineOffset: number } {
  if (/^\s*@start/m.test(code)) return { source: code, lineOffset: 0 };
  return { source: `@startuml\n${code}\n@enduml\n`, lineOffset: 1 };
}

/** Convierte un error del servidor en diagnóstico, con la línea referida al código del usuario. */
export function plantUmlErrors(response: PlantUmlServerResponse, lineOffset: number): PlantUmlSyntaxError[] {
  if (!response.error) return [];
  const line = Math.max(1, (response.error_line ?? 1) - lineOffset);
  return [{ line, column: 1, message: response.error }];
}

/**
 * El SVG viene de un servidor externo y se inserta en la página, que tiene acceso a la API de Tauri:
 * se limpia de scripts y manejadores. Los enlaces se quitan para que un clic no saque a la app de su página.
 */
function sanitizeSvg(svg: string): string {
  return DOMPurify.sanitize(svg, { USE_PROFILES: { svg: true, svgFilters: true }, FORBID_TAGS: ['a'] });
}

/** Renderiza en el servidor PlantUML indicado: el código del diagrama sale de la máquina. */
export async function renderPlantUmlSvg(code: string, server: string): Promise<PlantUmlRenderResult> {
  if (!code.trim()) return { svg: null, error: null, errors: [] };
  const { source, lineOffset } = preparePlantUmlSource(code);
  try {
    const response = await invokeTauri<PlantUmlServerResponse>('fetch_plantuml_svg', {
      server,
      encoded: encode(source),
    });
    if (response.error) return { svg: null, error: response.error, errors: plantUmlErrors(response, lineOffset) };
    return { svg: sanitizeSvg(response.svg ?? ''), error: null, errors: [] };
  } catch (e) {
    return { svg: null, error: String(e), errors: [] };
  }
}
