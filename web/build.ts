// Genera la web en web/dist: inglés en `/` y español en `/es/`.
// La plantilla lleva los textos en inglés entre llaves dobles; es.json los traduce. Falla si falta o sobra alguno.
import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const root = import.meta.dir;
const docs = join(root, '../docs');
const dist = join(root, 'dist');
const template = readFileSync(join(root, 'template.html'), 'utf8');
const es: Record<string, string> = JSON.parse(readFileSync(join(root, 'es.json'), 'utf8'));

const used = new Set([...template.matchAll(/\{\{(.+?)\}\}/g)].map((m) => m[1]));
const missing = [...used].filter((k) => !es[k]);
const unused = Object.keys(es).filter((k) => !used.has(k));
if (missing.length || unused.length) {
  for (const k of missing) console.error(`Falta la traducción al español: ${k}`);
  for (const k of unused) console.error(`Traducción que ya no se usa: ${k}`);
  process.exit(1);
}

const PAGES = [
  {
    dir: '', text: (k: string) => k,
    vars: {
      LANG: 'en', CHANGELOG: 'CHANGELOG.md', BASE: '', URL: 'https://synapsers.app/', OTHER: 'es', OTHER_HREF: 'es/', OTHER_NAME: 'Español',
      OFFER: 'Esta página también está en español.', OFFER_LINK: 'Ver en español', OFFER_CLOSE: 'Cerrar',
    },
  },
  {
    dir: 'es', text: (k: string) => es[k],
    vars: {
      LANG: 'es', CHANGELOG: 'CHANGELOG.es.md', BASE: '../', URL: 'https://synapsers.app/es/', OTHER: 'en', OTHER_HREF: '../', OTHER_NAME: 'English',
      OFFER: 'This page is also available in English.', OFFER_LINK: 'View in English', OFFER_CLOSE: 'Close',
    },
  },
];

rmSync(dist, { recursive: true, force: true });
const images = new Set<string>();
for (const page of PAGES) {
  const vars: Record<string, string> = page.vars;
  const html = template
    .replace(/\{\{(.+?)\}\}/g, (_, k) => page.text(k))
    .replace(/%([A-Z_]+)%/g, (all, name) => vars[name] ?? all);
  const left = html.match(/%[A-Z_]+%/);
  if (left) throw new Error(`Variable sin valor: ${left[0]}`);
  for (const m of html.matchAll(/src="(?:\.\.\/)?([\w.-]+\.(?:png|avif|webp|svg))"/g)) images.add(m[1]);
  mkdirSync(join(dist, page.dir), { recursive: true });
  writeFileSync(join(dist, page.dir, 'index.html'), html);
}
for (const image of images) cpSync(join(docs, image), join(dist, image));
console.log(`${PAGES.length} páginas, ${used.size} textos, ${images.size} imágenes → ${dist}`);
