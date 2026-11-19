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
