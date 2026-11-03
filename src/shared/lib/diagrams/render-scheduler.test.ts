import { describe, expect, it } from 'bun:test';
// Import dinámico: así el módulo .svelte.ts pasa por el compilador de Svelte de la precarga.
const { createRenderScheduler, diagramRenderStatus } = await import('./render-scheduler.svelte');

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

describe('agrupación de renders de diagramas', () => {
  it('no encola un render por tecla: tras el que está en curso, solo el último', async () => {
    const rendered: string[] = [];
    const scheduler = createRenderScheduler('D2', async (code) => {
      rendered.push(code);
      await wait(50);
    });
    scheduler.now('a');
    for (const code of ['ab', 'abc', 'abcd']) scheduler.now(code);
    await wait(200);
    expect(rendered).toEqual(['a', 'abcd']);
    expect(diagramRenderStatus.active).toBe(0);
    expect(diagramRenderStatus.engine).toBe('D2');
  });

  it('al escribir espera una pausa y no repite un código ya dibujado', async () => {
    const rendered: string[] = [];
    const scheduler = createRenderScheduler('Mermaid', async (code) => {
      rendered.push(code);
    });
    scheduler.schedule('graph TD');
    await wait(10);
    scheduler.schedule('graph TD\n A');
    scheduler.schedule('graph TD\n A --> B');
    await wait(100);
    expect(rendered).toEqual(['graph TD']);
    await wait(450);
    expect(rendered).toEqual(['graph TD', 'graph TD\n A --> B']);
    scheduler.schedule('graph TD\n A --> B');
    await wait(500);
    expect(rendered.length).toBe(2);
    // Ctrl+Enter redibuja aunque no haya cambios.
    scheduler.now('graph TD\n A --> B');
    await wait(10);
    expect(rendered.length).toBe(3);
  });
});
