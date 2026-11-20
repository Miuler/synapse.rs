import { describe, expect, it } from 'bun:test';
import { bundleOpenApi, hasExternalRefs, joinPath } from './bundle';

const files: Record<string, string> = {
  'api/src/paths/search.yaml': 'get:\n  summary: Buscar\n  responses:\n    "200":\n      $ref: "../components/responses.yaml#/Ok"\n',
  'api/src/components/responses.yaml': 'Ok:\n  description: Bien\n  content:\n    application/json:\n      schema:\n        $ref: "#/Item"\nItem:\n  type: object\n',
  'api/src/components/auth.yaml': 'type: http\nscheme: bearer\n',
};
const read = async (p: string) => files[p] ?? null;

describe('bundleOpenApi', () => {
  it('sigue referencias relativas entre archivos, también las internas de cada archivo', async () => {
    const main = [
      'openapi: 3.0.1',
      'paths:',
      '  /search:',
      '    $ref: ./paths/search.yaml',
      'components:',
      '  securitySchemes:',
      '    bearerAuth:',
      '      $ref: ./components/auth.yaml',
      '  schemas:',
      '    Local:',
      '      $ref: "#/components/schemas/Other"',
    ].join('\n');
    const r = await bundleOpenApi(main, 'api/src/main.yaml', read);
    const doc = JSON.parse(r.content);
    expect(doc.paths['/search'].get.summary).toBe('Buscar');
    expect(doc.paths['/search'].get.responses['200'].content['application/json'].schema.type).toBe('object');
    expect(doc.components.securitySchemes.bearerAuth.scheme).toBe('bearer');
    expect(doc.components.schemas.Local.$ref).toBe('#/components/schemas/Other');
    expect(r.files.sort()).toEqual(['api/src/components/auth.yaml', 'api/src/components/responses.yaml', 'api/src/paths/search.yaml']);
    expect(r.problems).toEqual([]);
  });

  it('avisa de los archivos que faltan y no sale de la bóveda', async () => {
    const main = 'openapi: 3.0.1\npaths:\n  /a:\n    $ref: ./paths/missing.yaml\n  /b:\n    $ref: ../../../fuera.yaml\n';
    const r = await bundleOpenApi(main, 'api/src/main.yaml', read);
    expect(r.problems).toHaveLength(2);
    expect(JSON.parse(r.content).paths['/a'].description).toContain('missing.yaml');
  });

  it('sin referencias externas devuelve el texto tal cual', async () => {
    const main = 'openapi: 3.0.1\ncomponents:\n  schemas:\n    A:\n      $ref: "#/components/schemas/B"\n';
    expect(hasExternalRefs(main)).toBe(false);
    expect((await bundleOpenApi(main, 'main.yaml', read)).content).toBe(main);
  });

  it('normaliza rutas', () => {
    expect(joinPath('a/b', '../c/./d.yaml')).toBe('a/c/d.yaml');
    expect(joinPath('a', '../../x')).toBeNull();
  });
});
