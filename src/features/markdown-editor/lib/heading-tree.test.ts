import { expect, it } from 'bun:test';
import { extractHeadingTree, flattenHeadings, headingPathAt } from './heading-anchor';

it('arma el árbol de títulos ignorando frontmatter y código', () => {
  const md = ['---', 'title: x', '---', '# Uno', '## Uno.a', '### Uno.a.i', '```', '# no', '```', '## Uno.b', '# Dos [[nota|Alias]]', '#### Suelto'].join('\n');
  const tree = extractHeadingTree(md);
  expect(tree.map((n) => n.text)).toEqual(['Uno', 'Dos Alias']);
  expect(tree[0].children.map((n) => n.text)).toEqual(['Uno.a', 'Uno.b']);
  expect(tree[0].children[0].children[0].text).toBe('Uno.a.i');
  expect(tree[1].children[0].level).toBe(4);
  expect(tree[1].raw).toBe('Dos [[nota|Alias]]');
});

it('ruta de títulos hasta una línea', () => {
  const tree = extractHeadingTree(['# A', 'x', '## B', '### C', '# D'].join('\n'));
  expect(flattenHeadings(tree).map((h) => h.line)).toEqual([0, 2, 3, 4]);
  expect(headingPathAt(tree, 3).map((h) => h.text)).toEqual(['A', 'B', 'C']);
  expect(headingPathAt(tree, 4).map((h) => h.text)).toEqual(['D']);
  expect(headingPathAt(tree, 1)).toEqual([]);
});
