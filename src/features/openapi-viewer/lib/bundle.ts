// Junta en un solo documento una especificación OpenAPI repartida en varios archivos (`$ref:
// ./paths/x.yaml`, `../schemas.yaml#/Pet`…). Scalar solo recibe texto, sin ruta, así que no puede
// seguir referencias relativas: se resuelven aquí con el lector que se le pase (archivos de la
// bóveda o de una revisión de Git). Nada sale de la bóveda: un `$ref` que apunta fuera o a una URL
// se deja como está.
import { parse } from 'yaml';

/** Devuelve el texto del archivo (ruta relativa a la bóveda, con `/`) o `null` si no existe. */
export type ReadFile = (path: string) => Promise<string | null>;

export interface BundleResult {
  /** Documento listo para Scalar (el original si no había referencias externas) */
  content: string;
  /** Archivos leídos además del principal */
  files: string[];
  /** Referencias que no se pudieron resolver, con el motivo */
  problems: string[];
}

const EXTERNAL_REF = /["']?\$ref["']?\s*:\s*["']?(?!#)[^\s"'#][^\s"']*/;

/** ¿Tiene algún `$ref` a otro archivo? */
export function hasExternalRefs(content: string): boolean {
  return EXTERNAL_REF.test(content);
}

function dirname(path: string): string {
  const i = path.lastIndexOf('/');
  return i < 0 ? '' : path.slice(0, i);
}

/** Une y normaliza; `null` si sale de la raíz de la bóveda. */
export function joinPath(base: string, rel: string): string | null {
  const out: string[] = [];
  for (const part of [...base.split('/'), ...rel.replace(/\\/g, '/').split('/')]) {
    if (!part || part === '.') continue;
    if (part === '..') {
      if (!out.length) return null;
      out.pop();
    } else out.push(part);
  }
  return out.join('/');
}

function pointer(doc: unknown, fragment: string): unknown {
  if (!fragment || fragment === '/') return doc;
  let cur: unknown = doc;
  for (const raw of fragment.replace(/^\//, '').split('/')) {
    const key = decodeURIComponent(raw).replace(/~1/g, '/').replace(/~0/g, '~');
    if (cur === null || typeof cur !== 'object' || !(key in (cur as object))) return undefined;
    cur = (cur as Record<string, unknown>)[key];
  }
  return cur;
}

function parseDoc(path: string, text: string): unknown {
  return /\.json$/i.test(path) ? JSON.parse(text) : parse(text);
}

export async function bundleOpenApi(content: string, path: string, read: ReadFile): Promise<BundleResult> {
  const root = path.replace(/\\/g, '/').replace(/^\.\//, '');
  if (!hasExternalRefs(content)) return { content, files: [], problems: [] };
  let rootDoc: unknown;
  try {
    rootDoc = parseDoc(root, content);
  } catch {
    return { content, files: [], problems: [] }; // Scalar ya mostrará el error de sintaxis
  }

  const docs = new Map<string, Promise<unknown>>([[root, Promise.resolve(rootDoc)]]);
  const files = new Set<string>();
  const problems: string[] = [];

  function load(file: string): Promise<unknown> {
    let p = docs.get(file);
    if (!p) {
      p = read(file).then((text) => {
        if (text == null) throw new Error(`no existe ${file}`);
        files.add(file);
        try {
          return parseDoc(file, text);
        } catch (e) {
          throw new Error(`${file}: ${e instanceof Error ? e.message : e}`);
        }
      });
      docs.set(file, p);
    }
    return p;
  }

  /** `node` pertenece a `file`; `stack` = referencias en curso (para no entrar en ciclos). */
  async function walk(node: unknown, file: string, stack: string[]): Promise<unknown> {
    if (Array.isArray(node)) return Promise.all(node.map((n) => walk(n, file, stack)));
    if (node === null || typeof node !== 'object') return node;
    const obj = node as Record<string, unknown>;
    const ref = obj.$ref;
    if (typeof ref === 'string' && !/^[a-z][a-z0-9+.-]*:/i.test(ref) && !(file === root && ref.startsWith('#'))) {
      const [filePart, fragment = ''] = ref.split('#');
      const target = filePart ? joinPath(dirname(file), filePart) : file;
      if (target === null) {
        problems.push(`${file}: «${ref}» apunta fuera de la bóveda`);
        return node;
      }
      const key = `${target}#${fragment}`;
      if (stack.includes(key)) return target === root ? { ...obj, $ref: `#${fragment}` } : node; // ciclo
      try {
        const value = pointer(await load(target), fragment);
        if (value === undefined) throw new Error(`no se encontró «#${fragment}» en ${target}`);
        const resolved = await walk(value, target, [...stack, key]);
        const { $ref: _, ...siblings } = obj;
        const rest = await walk(siblings, file, stack);
        return resolved && typeof resolved === 'object' && !Array.isArray(resolved)
          ? { ...(resolved as object), ...(rest as object) }
          : resolved;
      } catch (e) {
        problems.push(`${file}: «${ref}»: ${e instanceof Error ? e.message : e}`);
        return { description: `⚠ No se pudo resolver «${ref}»` };
      }
    }
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(obj)) out[k] = await walk(v, file, stack);
    return out;
  }

  const bundled = await walk(rootDoc, root, []);
  return { content: JSON.stringify(bundled), files: [...files], problems };
}
