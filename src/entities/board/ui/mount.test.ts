import { afterEach, describe, expect, it } from 'bun:test';
import { i18n } from '@shared/lib/i18n';
import { mountBoardReferences, type BoardVault } from './mount';

interface FakeNode {
  tagName: string;
  className: string;
  textContent: string | null;
  children: FakeNode[];
  dataset: Record<string, string>;
  replaceChildren(...nodes: (FakeNode | string)[]): void;
}

function fakeElement(tagName: string): FakeNode {
  return {
    tagName: tagName.toUpperCase(),
    className: '',
    textContent: null,
    children: [],
    dataset: {},
    replaceChildren(...nodes) {
      this.children = nodes.map((n) => (typeof n === 'string' ? { ...fakeElement('#text'), textContent: n } : n));
    },
  };
}

const originalDocument = globalThis.document;
const initialLocale = i18n.locale;

afterEach(() => {
  if (originalDocument === undefined) {
    // @ts-expect-error restaurar entorno headless sin DOM
    delete globalThis.document;
  } else {
    globalThis.document = originalDocument;
  }
  i18n.setLocale(initialLocale);
});

describe('mountBoardReferences', () => {
  it('vuelve a pintar los bloques montados al cambiar el idioma y se desuscribe al limpiar', async () => {
    globalThis.document = {
      createElement: (tag: string) => fakeElement(tag),
    } as unknown as Document;

    const box = fakeElement('div');
    box.className = 'board-ref';
    box.dataset.code = encodeURIComponent('board: NoExiste');

    const root = {
      querySelectorAll(selector: string) {
        if (selector.includes('.board-ref')) return [box];
        return [];
      },
      addEventListener() {},
      removeEventListener() {},
    } as unknown as HTMLElement;

    const vault: BoardVault = {
      async resolveVaultLinkDetailed() {
        return { status: 'missing' };
      },
      async onFilesChanged() {
        return () => {};
      },
    };

    let paints = 0;
    let notifyPaint: (() => void) | null = null;
    const waitPaint = () =>
      new Promise<void>((resolve) => {
        notifyPaint = resolve;
      });

    i18n.setLocale('en');
    const firstPaint = waitPaint();
    const stop = mountBoardReferences(root, null, vault, () => {
      paints += 1;
      notifyPaint?.();
      notifyPaint = null;
    });

    await firstPaint;
    expect(box.children[0]?.textContent).toBe('There is no board «NoExiste».');

    const secondPaint = waitPaint();
    i18n.setLocale('es');
    await secondPaint;
    expect(box.children[0]?.textContent).toBe('No existe el tablero «NoExiste».');
    expect(paints).toBe(2);

    stop();
    i18n.setLocale('en');
    await Promise.resolve();
    expect(paints).toBe(2);
    expect(box.children[0]?.textContent).toBe('No existe el tablero «NoExiste».');
  });
});
