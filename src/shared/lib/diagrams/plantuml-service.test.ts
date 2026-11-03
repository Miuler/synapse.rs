import { describe, expect, it } from 'bun:test';
import { plantUmlErrors, preparePlantUmlSource } from './plantuml-service';

describe('código enviado al servidor PlantUML', () => {
  it('respeta un diagrama que ya trae @startuml', () => {
    const code = "' comentario\n@startuml\nA -> B\n@enduml\n";
    expect(preparePlantUmlSource(code)).toEqual({ source: code, lineOffset: 0 });
  });

  it('reconoce otros tipos de diagrama (@startmindmap)', () => {
    expect(preparePlantUmlSource('@startmindmap\n* a\n@endmindmap').lineOffset).toBe(0);
  });

  it('envuelve el código suelto', () => {
    expect(preparePlantUmlSource('A -> B')).toEqual({ source: '@startuml\nA -> B\n@enduml\n', lineOffset: 1 });
  });
});

describe('errores del servidor PlantUML', () => {
  it('sin error no hay diagnósticos', () => {
    expect(plantUmlErrors({ svg: '<svg/>', error: null, error_line: null }, 0)).toEqual([]);
  });

  it('la línea se refiere al código del usuario, sin el @startuml añadido', () => {
    const errors = plantUmlErrors({ svg: null, error: 'Syntax Error?', error_line: 3 }, 1);
    expect(errors).toEqual([{ line: 2, column: 1, message: 'Syntax Error?' }]);
  });

  it('un error sin línea o en la línea añadida cae en la primera', () => {
    expect(plantUmlErrors({ svg: null, error: 'x', error_line: null }, 0)[0].line).toBe(1);
    expect(plantUmlErrors({ svg: null, error: 'x', error_line: 1 }, 1)[0].line).toBe(1);
  });
});
