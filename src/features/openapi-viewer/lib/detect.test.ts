import { expect, it } from 'bun:test';
import { isOpenApiContent } from './detect';

it('detecta OpenAPI 3 y Swagger 2 en YAML', () => {
  expect(isOpenApiContent('a/openapi.yaml', 'openapi: 3.1.0\ninfo:\n  title: X\n')).toBe(true);
  expect(isOpenApiContent('api.yml', '# c\nswagger: "2.0"\n')).toBe(true);
  expect(isOpenApiContent('x.yaml', 'info:\n  openapi: 3.0.0\n')).toBe(false);
  expect(isOpenApiContent('x.yaml', 'name: foo\n')).toBe(false);
});

it('detecta OpenAPI en JSON solo como clave raíz', () => {
  expect(isOpenApiContent('a.openapi.json', '{\n  "openapi": "3.0.0", "info": {}\n}')).toBe(true);
  expect(isOpenApiContent('p.json', '{"a": {"openapi": "3.0.0"}}')).toBe(false);
  expect(isOpenApiContent('p.json', '{"name": "openapi"}')).toBe(false);
  expect(isOpenApiContent('p.md', 'openapi: 3.0.0')).toBe(false);
});
