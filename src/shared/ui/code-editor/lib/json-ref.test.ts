import { describe, expect, it } from 'bun:test';
import { findPointerLine, findRefsInLine, pointerSegments, resolveRef } from './json-ref';

describe('findRefsInLine', () => {
  it('reconoce $ref en YAML con y sin comillas, y en JSON', () => {
    expect(findRefsInLine('    $ref: ./paths/outbound-transfers.yaml')[0].target).toBe('./paths/outbound-transfers.yaml');
    expect(findRefsInLine("  - $ref: '#/components/schemas/Error'")[0].target).toBe('#/components/schemas/Error');
    expect(findRefsInLine('  "$ref": "common.json#/defs/Id",')[0].target).toBe('common.json#/defs/Id');
    expect(findRefsInLine('{ $ref: a.yaml }')[0].target).toBe('a.yaml');
    expect(findRefsInLine('  description: usa $ref para algo')).toEqual([]);
  });

  it('el rango cubre solo el valor', () => {
    const text = "    $ref: './a.yaml'";
    const [r] = findRefsInLine(text, 100);
    expect(text.slice(r.from - 100, r.to - 100)).toBe('./a.yaml');
  });
});

describe('resolveRef', () => {
  it('resuelve rutas relativas a la carpeta del archivo', () => {
    expect(resolveRef('api/src/main.yaml', './paths/x.yaml')).toEqual({ path: 'api/src/paths/x.yaml', pointer: null });
    expect(resolveRef('api/src/paths/x.yaml', '../components/s.yaml#/Bearer')).toEqual({
      path: 'api/src/components/s.yaml',
      pointer: '/Bearer',
    });
    expect(resolveRef('main.yaml', '#/components/schemas/Error')).toEqual({ path: null, pointer: '/components/schemas/Error' });
  });

  it('no sale de la bóveda ni sigue URLs', () => {
    expect(resolveRef('a.yaml', '../fuera.yaml')).toBeNull();
    expect(resolveRef('a.yaml', 'https://x.com/s.json')).toBeNull();
  });
});

describe('findPointerLine', () => {
  const yaml = [
    'openapi: 3.0.1', //                1
    'paths:', //                        2
    '  /outbound/transfers:', //        3
    '    post:', //                     4
    '      parameters:', //             5
    '        - name: id', //            6
    '          in: path', //            7
    '        - name: canal', //         8
    '          in: header', //          9
    'components:', //                   10
    '  schemas:', //                    11
    '    Error:', //                    12
    '      type: object', //            13
    '    "Quoted Key":', //             14
    '      type: string', //            15
  ];

  it('sigue las claves por sangría', () => {
    expect(findPointerLine(yaml, '/components/schemas/Error')).toBe(12);
    expect(findPointerLine(yaml, '/components/schemas/Quoted Key')).toBe(14);
    expect(findPointerLine(yaml, '/components/schemas/Nada')).toBeNull();
  });

  it('decodifica ~1 en rutas de OpenAPI y entra en elementos de lista', () => {
    expect(pointerSegments('/paths/~1outbound~1transfers/post')).toEqual(['paths', '/outbound/transfers', 'post']);
    expect(findPointerLine(yaml, '/paths/~1outbound~1transfers/post')).toBe(4);
    expect(findPointerLine(yaml, '/paths/~1outbound~1transfers/post/parameters/1')).toBe(8);
    expect(findPointerLine(yaml, '/paths/~1outbound~1transfers/post/parameters/1/in')).toBe(9);
    expect(findPointerLine(yaml, '/paths/~1outbound~1transfers/post/parameters/0/name')).toBe(6);
  });

  it('una clave con el mismo nombre en otra rama no confunde la búsqueda', () => {
    const lines = ['a:', '  x: 1', 'b:', '  x: 2'];
    expect(findPointerLine(lines, '/b/x')).toBe(4);
  });

  it('funciona con JSON con formato', () => {
    const json = ['{', '  "definitions": {', '    "Id": {', '      "type": "string"', '    }', '  }', '}'];
    expect(findPointerLine(json, '/definitions/Id')).toBe(3);
  });
});

import { extractKeyTree, keyLineAt } from './json-ref';

describe('extractKeyTree', () => {
  const yaml = [
    'openapi: 3.0.1', //                1
    'paths:', //                        2
    '  /outbound/transfers:', //        3
    '    post:', //                     4
    '      parameters:', //             5
    '        - name: id', //            6
    '          in: path', //            7
    '        - name: canal', //         8
    'components:', //                   9
    '  schemas:', //                    10
    '    Error:', //                    11
    '      type: object', //            12
  ];

  it('arma el árbol de claves con sus punteros y líneas', () => {
    const tree = extractKeyTree(yaml.join('\n'), 6);
    expect(tree.map((n) => n.text)).toEqual(['openapi', 'paths', 'components']);
    const post = tree[1].children[0].children[0];
    expect(post.raw).toBe('/paths/~1outbound~1transfers/post');
    const items = post.children[0].children;
    expect(items.map((n) => [n.text, n.raw, n.line])).toEqual([
      ['- id', '/paths/~1outbound~1transfers/post/parameters/0', 6],
      ['- canal', '/paths/~1outbound~1transfers/post/parameters/1', 8],
    ]);
    expect(items[0].children.map((n) => n.raw)).toEqual(['/paths/~1outbound~1transfers/post/parameters/0/in']);
    // Cada puntero lleva a su propia línea
    for (const n of [post, ...items, tree[2].children[0].children[0]]) expect(findPointerLine(yaml, n.raw)).toBe(n.line);
  });

  it('respeta la profundidad máxima sin colgar nodos profundos de la raíz', () => {
    const tree = extractKeyTree(yaml.join('\n'), 2);
    expect(tree.map((n) => n.text)).toEqual(['openapi', 'paths', 'components']);
    expect(tree[1].children[0].children).toEqual([]);
  });

  it('keyLineAt da la clave más profunda en o antes del cursor', () => {
    const tree = extractKeyTree(yaml.join('\n'), 6);
    expect(keyLineAt(tree, 12)).toBe(12);
    expect(keyLineAt(extractKeyTree(yaml.join("\n"), 3), 12)).toBe(11);
    expect(keyLineAt(tree, 7)).toBe(7);
    expect(keyLineAt(tree, 1)).toBe(1);
  });
});
