import { describe, expect, it } from 'bun:test';
import { ago, fileMention, mentionsPrompt, mergeVaults, splitMentions, vaultToken } from './mentions';

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
