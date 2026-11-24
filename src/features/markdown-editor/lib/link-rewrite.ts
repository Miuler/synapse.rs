// Corregir un enlace ambiguo en el texto de una nota: `[[Glosario.docx]]` → `[[carpeta/Glosario.docx|Glosario.docx]]`
// y `[texto](Glosario.docx)` → `[texto](carpeta/Glosario.docx)`. Se cambian todos los enlaces de la nota
// que apuntan a ese mismo destino (mismo nombre, misma ambigüedad); el resto del texto no se toca.

/** Destino sin `#ancla`, sin `./` y con `.md` si no tiene extensión (como lo normaliza el resolvedor) */
export function normalizeLinkTarget(target: string): string {
  const file = target.trim().split('#')[0].replace(/^\.\/+/, '');
  return /\.[^/.]+$/.test(file) ? file : `${file}.md`;
}

/** Ruta del destino tal como conviene escribirla en un enlace Markdown desde `sourcePath` */
function markdownHref(chosen: string, sourcePath: string): string {
  const dir = sourcePath.includes('/') ? sourcePath.slice(0, sourcePath.lastIndexOf('/') + 1) : '';
  return dir && chosen.startsWith(dir) ? chosen.slice(dir.length) : chosen;
}

/**
 * Reescribe en `content` los enlaces cuyo destino es `rawTarget` para que apunten a `chosen` (ruta
 * relativa a la bóveda). Devuelve el texto nuevo y cuántos enlaces cambió.
 */
export function rewriteLinkTarget(content: string, rawTarget: string, chosen: string, sourcePath: string): { content: string; count: number } {
  const wanted = normalizeLinkTarget(rawTarget);
  const chosenNoMd = chosen.replace(/\.md$/i, '');
  let count = 0;

  // Wikilinks y embeds: [[destino#ancla|alias]]
  let out = content.replace(/(!?)\[\[([^\]\n|#]+)(#[^\]\n|]*)?(\|[^\]\n]*)?\]\]/g, (m, bang: string, target: string, anchor = '', alias = '') => {
    if (normalizeLinkTarget(target) !== wanted) return m;
    count++;
    // La extensión `.md` se omite como en el original; el texto visible se conserva con un alias
    const path = /\.md$/i.test(target.trim()) ? chosen : chosenNoMd;
    const keepAlias = alias || (bang ? '' : `|${target.trim()}`);
    return `${bang}[[${path}${anchor}${keepAlias}]]`;
  });

  // Enlaces e imágenes Markdown: [texto](destino) o [texto](<destino con espacios>), con título opcional
  out = out.replace(/(!?\[[^\]\n]*\]\()(<[^>\n]+>|[^)\s]+)((?:\s+"[^"\n]*")?\))/g, (m, head: string, href: string, tail: string) => {
    const bare = href.startsWith('<') ? href.slice(1, -1) : href;
    if (/^[a-z][a-z0-9+.-]*:/i.test(bare)) return m; // URL externa
    let decoded = bare;
    try {
      decoded = decodeURIComponent(bare);
    } catch {
      /* se usa tal cual */
    }
    if (normalizeLinkTarget(decoded) !== wanted) return m;
    count++;
    const anchor = decoded.includes('#') ? decoded.slice(decoded.indexOf('#')) : '';
    const next = markdownHref(chosen, sourcePath) + anchor;
    return `${head}${/\s/.test(next) ? `<${next}>` : next}${tail}`;
  });

  return { content: out, count };
}
