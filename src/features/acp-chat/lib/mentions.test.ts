import { describe, expect, it } from 'bun:test';
import { ago, bestMatches, mentionAt, fileMention, folderMention, foldersOf, mentionsPrompt, mergeVaults, splitMentions, vaultToken } from './mentions';

const conv = (status: 'working' | 'idle') => ({ handle: '#1', agent: 'Claude', title: 'x', status });

describe('menciones', () => {
  it('une bóvedas abiertas y conocidas, sin la propia ni repetidas', () => {
    const v = mergeVaults(
      [
        { vault: 'A', path: '/a', conversations: [conv('idle')] },
        { vault: 'B', path: '/b/', conversations: [conv('working')] },
      ],
      [
        { name: 'B', path: '/b' },
        { name: 'C', path: '/c' },
        { name: 'Yo', path: '/yo' },
      ],
      '/yo',
    );
    expect(v.map((x) => [x.name, x.state])).toEqual([
      ['A', 'active'],
      ['B', 'busy'],
      ['C', 'off'],
    ]);
  });

  it('sugiere primero los archivos cuyo nombre casa, no los primeros por orden alfabético', () => {
    const paths = [
      'api/docs/mapping/contract.xlsx',
      'api/src/main-helpers/Header.yaml',
      'business-gateway/src/main.yaml',
      'core/domain.yaml',
      'core/src/main.yaml',
      'core/src/old-main.yaml',
    ];
    const best = (q: string, n = 10) => bestMatches(paths, (p) => p, q, n);
    // `domain.yaml` también contiene «main.yaml», pero va después de los que se llaman así
    expect(best('main.yaml', 3)).toEqual(['core/src/main.yaml', 'business-gateway/src/main.yaml', 'core/domain.yaml']);
    expect(best('main', 2)).toEqual(['core/src/main.yaml', 'business-gateway/src/main.yaml']);
    // Con parte de la carpeta delante se llega al que se quiere
    expect(best('gate/main', 1)).toEqual(['business-gateway/src/main.yaml']);
    expect(best('', 2)).toEqual(paths.slice(0, 2));
  });

  it('busca nombres con espacios sin confundirlos con el resto de la frase', () => {
    expect(mentionAt('mira @Algo con', [])).toEqual({ query: 'algo con', start: 5 });
    expect(mentionAt('mira @ algo', [])).toBeNull();
    expect(mentionAt('mira @algo\ny más', [])).toBeNull();
    expect(mentionAt('@Bóveda#2 dime', [])).toBeNull();
    const picked = fileMention('notas/algo con espacio.md');
    expect(mentionAt(`mira ${picked.token} y dime`, [picked])).toBeNull();
    expect(mentionAt(`mira ${picked.token} y @otra`, [picked])?.query).toBe('otra');

    const paths = ['a/lago/mi conejo.md', 'notas/algo con espacio.md', 'notas/espacio de algo.md'];
    const best = (q: string) => bestMatches(paths, (p) => p, q, 10);
    expect(best('algo con')).toEqual(['notas/algo con espacio.md']);
    expect(best('algo ')[0]).toBe('notas/algo con espacio.md');
    // Las palabras en otro orden también valen; letras sueltas de una frase, no
    expect(best('espacio algo')).toEqual(['notas/espacio de algo.md', 'notas/algo con espacio.md']);
    expect(best('a y dime')).toEqual([]);
  });

  it('saca las carpetas de las rutas y menciona una con barra final', () => {
    expect(foldersOf(['docs/guias/a.md', 'docs/b.md', 'c.md'])).toEqual(['docs', 'docs/guias']);
    const folder = folderMention('docs/mis guias');
    expect(folder.token).toBe('@docs/mis_guias/');
    // La carpeta no se confunde con un archivo suyo
    const file = fileMention('docs/mis guias/a.md');
    const { used } = mentionsPrompt(`mira ${file.token} y ${folder.token}, por favor`, [folder, file]);
    expect(used).toEqual([folder, file]);
    expect(mentionsPrompt(`mira ${file.token}`, [folder, file]).used).toEqual([file]);
  });

  it('explica al agente solo las menciones que siguen en el texto', () => {
    const mentions = [
      { token: '#2', to: '#2', title: 'la conversación «Diagramas»' },
      { token: '@Mi_Bóveda', to: 'Mi Bóveda', title: 'la bóveda «Mi Bóveda»' },
    ];
    const { prompt, used } = mentionsPrompt('pregúntale a #2, por favor', mentions);
    expect(used.map((m) => m.token)).toEqual(['#2']);
    expect(prompt).toContain('to="#2"');
    expect(prompt).not.toContain('Mi Bóveda');
    // «#20» no es «#2»
    expect(mentionsPrompt('mira #20', mentions).used).toEqual([]);
  });

  it('un archivo queda en el texto y no se presenta como destinatario', () => {
    const m = fileMention('docs/mi nota.md');
    expect(m.token).toBe('@docs/mi_nota.md');
    const { prompt, used } = mentionsPrompt('compara @docs/mi_nota.md con lo anterior', [m]);
    expect(used).toEqual([m]);
    expect(prompt).toContain('- @docs/mi_nota.md: el archivo «docs/mi nota.md» (adjunto)');
    expect(prompt).not.toContain('send_to_conversation');
  });

  it('parte el texto en trozos y menciones', () => {
    const m = { token: '@Mi_Bóveda#2', to: 'Mi Bóveda#2', title: 't' };
    expect(splitMentions('hola @Mi_Bóveda#2 ya', [m])).toEqual([{ text: 'hola ' }, { mention: m }, { text: ' ya' }]);
    expect(splitMentions('sin menciones', undefined)).toEqual([{ text: 'sin menciones' }]);
  });

  it('formatea tokens y tiempos', () => {
    expect(vaultToken('BCP MAINFRAME')).toBe('@BCP_MAINFRAME');
    expect(ago(1_000_000 - 3 * 60_000, 1_000_000)).toBe('hace 3 min');
    expect(ago(1_000_000 - 2 * 3_600_000, 1_000_000)).toBe('hace 2 h');
  });
});
