import { describe, expect, it } from 'bun:test';
import { rewriteLinkTarget } from './link-rewrite';

describe('rewriteLinkTarget', () => {
  it('fija wikilinks conservando el texto visible, el alias y el ancla', () => {
    const md = 'Ver [[Glosario.docx]], [[Glosario.docx|el glosario]] y [[informe#Resumen]]. Otro: [[otra]]';
    expect(rewriteLinkTarget(md, 'Glosario.docx', 'proyecto/AS41/Glosario.docx', 'notas/plan.md')).toEqual({
      content:
        'Ver [[proyecto/AS41/Glosario.docx|Glosario.docx]], [[proyecto/AS41/Glosario.docx|el glosario]] y [[informe#Resumen]]. Otro: [[otra]]',
      count: 2,
    });
    expect(rewriteLinkTarget('[[informe#Resumen]]', 'informe.md', 'a/b/informe.md', 'c/x.md').content).toBe('[[a/b/informe#Resumen|informe]]');
  });

  it('un embed de imagen no gana alias', () => {
    expect(rewriteLinkTarget('![[foto.png]]', 'foto.png', 'img/foto.png', 'x.md').content).toBe('![[img/foto.png]]');
  });

  it('fija enlaces Markdown con ruta relativa a la nota si cae debajo de su carpeta, y con <> si hay espacios', () => {
    const md = '[glosario](Glosario.docx) y ![f](foto.png "t") y [web](https://x.com/Glosario.docx)';
    expect(rewriteLinkTarget(md, 'Glosario.docx', 'notas/Teams AS41/Glosario.docx', 'notas/plan.md').content).toBe(
      '[glosario](<Teams AS41/Glosario.docx>) y ![f](foto.png "t") y [web](https://x.com/Glosario.docx)',
    );
    expect(rewriteLinkTarget('[g](Glosario%20BCP.docx#x)', 'Glosario BCP.docx', 'otra/Glosario BCP.docx', 'notas/plan.md').content).toBe(
      '[g](<otra/Glosario BCP.docx#x>)',
    );
  });
});
