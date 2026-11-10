import { expect, it } from 'bun:test';
import { extractHeadingTree } from './heading-anchor';

it('arma el árbol de títulos ignorando frontmatter y código', () => {
  const md = ['---', 'title: x', '---', '# Uno', '## Uno.a', '### Uno.a.i', '```', '# no', '```', '## Uno.b', '# Dos [[nota|Alias]]', '#### Suelto'].join('\n');
  const tree = extractHeadingTree(md);
  expect(tree.map((n) => n.text)).toEqual(['Uno', 'Dos Alias']);
  expect(tree[0].children.map((n) => n.text)).toEqual(['Uno.a', 'Uno.b']);
  expect(tree[0].children[0].children[0].text).toBe('Uno.a.i');
  expect(tree[1].children[0].level).toBe(4);
  expect(tree[1].raw).toBe('Dos [[nota|Alias]]');
});
