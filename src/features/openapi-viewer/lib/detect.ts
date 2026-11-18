// Detecta si un archivo es una especificación OpenAPI (3.x) o Swagger (2.0) por su contenido.

/** ¿La extensión permite que el archivo sea OpenAPI? (.yaml, .yml, .json) */
export function isOpenApiCandidate(path: string | null | undefined): boolean {
  if (!path) return false;
  const lower = path.toLowerCase().split('?')[0].split('#')[0];
  return /\.(ya?ml|json)$/.test(lower);
}

/** Clave raíz `openapi:` / `swagger:` en YAML, o `"openapi"` / `"swagger"` como clave del objeto raíz en JSON. */
export function isOpenApiContent(path: string | null | undefined, content: string | null | undefined): boolean {
  if (!isOpenApiCandidate(path) || !content) return false;
  const head = content.slice(0, 4096);
  if (/\.json$/i.test(path!.split('?')[0])) {
    // Clave al primer nivel: se cuentan llaves hasta encontrarla.
    let depth = 0;
    const re = /[{}]|"(?:[^"\\]|\\.)*"/g;
    let m: RegExpExecArray | null;
    while ((m = re.exec(head))) {
      const tok = m[0];
      if (tok === '{') depth++;
      else if (tok === '}') depth--;
      else if (depth === 1 && (tok === '"openapi"' || tok === '"swagger"') && /^\s*:/.test(head.slice(re.lastIndex))) return true;
    }
    return false;
  }
  return /^(?:openapi|swagger)\s*:\s*["']?\d/m.test(head);
}
