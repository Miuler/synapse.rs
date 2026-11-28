// Plegado por títulos en la vista de lectura: cada título con contenido lleva una flecha que oculta
// lo suyo hasta el siguiente título del mismo nivel o superior. Lo plegado se recuerda por archivo
// mientras la app está abierta (cambiar a edición y volver no lo pierde).

import { t } from '@shared/lib/i18n';

/** Elemento de la nota: nivel de título (0 si no es título) y su clave. */
export interface FoldBlock {
  level: number;
  key: string;
}

/** Qué bloques quedan ocultos con las claves plegadas. */
export function hiddenBlocks(blocks: FoldBlock[], folded: ReadonlySet<string>): boolean[] {
  let foldLevel = 0;
  return blocks.map(({ level, key }) => {
    if (foldLevel && level && level <= foldLevel) foldLevel = 0;
    const hidden = foldLevel > 0;
    if (!hidden && level && folded.has(key)) foldLevel = level;
    return hidden;
  });
}

/** Títulos que contienen al bloque `index`, del más cercano al exterior (sin incluirlo). */
export function ancestorKeys(blocks: FoldBlock[], index: number): string[] {
  const keys: string[] = [];
  let level = blocks[index].level || 7;
  for (let i = index - 1; i >= 0; i--) {
    if (!blocks[i].level || blocks[i].level >= level) continue;
    keys.push(blocks[i].key);
    level = blocks[i].level;
  }
  return keys;
}

const foldsByFile = new Map<string, Set<string>>();

function foldsOf(file: string | null): Set<string> {
  const id = file ?? '';
  let set = foldsByFile.get(id);
  if (!set) foldsByFile.set(id, (set = new Set()));
  return set;
}

/** Bloques de la nota, en orden. La clave distingue títulos repetidos. */
function blocksOf(article: HTMLElement): { els: HTMLElement[]; blocks: FoldBlock[] } {
  const els = [...article.children] as HTMLElement[];
  const seen = new Map<string, number>();
  const blocks = els.map((el) => {
    const level = /^H[1-6]$/.test(el.tagName) ? Number(el.tagName[1]) : 0;
    if (!level) return { level, key: '' };
    const base = `${level}:${el.textContent?.trim()}`;
    const n = seen.get(base) ?? 0;
    seen.set(base, n + 1);
    return { level, key: `${base}:${n}` };
  });
  return { els, blocks };
}

/** Pone las flechas y oculta lo plegado. Se llama tras cada renderizado y cada cambio. */
export function applyFolds(article: HTMLElement, file: string | null): void {
  const folded = foldsOf(file);
  const { els, blocks } = blocksOf(article);
  const hidden = hiddenBlocks(blocks, folded);
  els.forEach((el, i) => {
    el.classList.toggle('reading-fold-hidden', hidden[i]);
    const { level, key } = blocks[i];
    if (!level) return;
    const next = blocks[i + 1];
    const foldable = !!next && (!next.level || next.level > level);
    let button = el.querySelector<HTMLButtonElement>(':scope > .reading-fold');
    if (!foldable) return button?.remove();
    if (!button) {
      button = document.createElement('button');
      button.type = 'button';
      button.className = 'reading-fold';
      el.prepend(button);
    }
    const isFolded = folded.has(key);
    button.setAttribute('aria-expanded', String(!isFolded));
    button.title = isFolded ? t('readingView.unfoldTitle') : t('readingView.foldTitle');
    button.setAttribute('aria-label', isFolded ? t('readingView.unfoldSection') : t('readingView.foldSection'));
    el.classList.toggle('reading-folded', isFolded);
  });
}

/** Pliega o despliega el título de `button`; con `sameLevel`, todos los de su nivel. */
export function toggleFold(article: HTMLElement, file: string | null, button: HTMLElement, sameLevel: boolean): void {
  const folded = foldsOf(file);
  const { els, blocks } = blocksOf(article);
  const target = blocks[els.indexOf(button.parentElement as HTMLElement)];
  if (!target) return;
  const unfold = folded.has(target.key);
  for (const b of sameLevel ? blocks.filter((x) => x.level === target.level) : [target]) {
    if (unfold) folded.delete(b.key);
    else folded.add(b.key);
  }
  applyFolds(article, file);
}

export function setAllFolds(article: HTMLElement, file: string | null, fold: boolean): void {
  const folded = foldsOf(file);
  folded.clear();
  if (fold) for (const b of blocksOf(article).blocks) if (b.level) folded.add(b.key);
  applyFolds(article, file);
}

/** Despliega lo necesario para que se vea `el` (un título buscado, una coincidencia…). */
export function revealFolded(article: HTMLElement, file: string | null, el: Element): void {
  const { els, blocks } = blocksOf(article);
  const index = els.findIndex((x) => x.contains(el));
  if (index < 0 || !els[index].classList.contains('reading-fold-hidden')) return;
  const folded = foldsOf(file);
  for (const key of ancestorKeys(blocks, index)) folded.delete(key);
  applyFolds(article, file);
}
