// Todos los diálogos unificados con @shared/ui/dialog-resize se redimensionan arrastrando
// bordes/esquinas y recuerdan su tamaño al volver a abrirlos.
import { afterAll, beforeAll, expect, it } from 'bun:test';
import { Key } from 'webdriverio';
import { startApp, type Session } from './harness';

let s: Session;

beforeAll(async () => {
  s = await startApp({
    'nota.md': '# Nota\n\nContenido de prueba para búsqueda.\n',
  });
  await s.browser.$('.file-tree-item[data-path="nota.md"]').waitForExist({ timeout: 30_000 });
  await s.browser.$('.file-tree-item[data-path="nota.md"]').click();
  await s.browser.execute(() => {
    localStorage.removeItem('synapse.quickOpen.size');
    localStorage.removeItem('synapse.ftsDialog.size');
    localStorage.removeItem('synapse.commandPalette.size');
    localStorage.removeItem('synapse.settingsPanel.size');
    localStorage.removeItem('synapse.communityPlugins.size');
    localStorage.removeItem('synapse.outlineDialog.size');
  });
}, 120_000);

afterAll(async () => {
  await s?.close();
});

async function dragHandle(selector: string, dx: number, dy: number) {
  await s.browser.execute(
    (sel: string, deltaX: number, deltaY: number) => {
      const handle = document.querySelector(sel) as HTMLElement;
      if (!handle) throw new Error(`Handle no encontrado: ${sel}`);
      const r = handle.getBoundingClientRect();
      const x = r.left + r.width / 2;
      const y = r.top + r.height / 2;
      handle.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, clientX: x, clientY: y }));
      window.dispatchEvent(new PointerEvent('pointermove', { clientX: x + deltaX, clientY: y + deltaY }));
      window.dispatchEvent(new PointerEvent('pointerup', { clientX: x + deltaX, clientY: y + deltaY }));
    },
    selector,
    dx,
    dy,
  );
}

async function rectOf(selector: string) {
  return (await s.browser.execute((sel: string) => {
    const r = document.querySelector(sel)!.getBoundingClientRect();
    return { width: r.width, height: r.height };
  }, selector)) as { width: number; height: number };
}

async function verifyDialogResize(opts: {
  dialogSel: string;
  handleSel: string;
  dx: number;
  dy: number;
  open: () => Promise<void>;
  close: () => Promise<void>;
}) {
  await opts.open();
  await s.browser.$(opts.handleSel).waitForDisplayed({ timeout: 5_000 });
  await Bun.sleep(250);

  const before = await rectOf(opts.dialogSel);
  await dragHandle(opts.handleSel, opts.dx, opts.dy);
  const after = await rectOf(opts.dialogSel);

  if (opts.dx > 0) expect(after.width).toBeGreaterThan(before.width + 40);
  if (opts.dx < 0) expect(after.width).toBeLessThan(before.width - 40);
  if (opts.dy > 0) expect(after.height).toBeGreaterThan(before.height + 30);

  await opts.close();
  await s.browser.$(opts.dialogSel).waitForExist({ reverse: true, timeout: 5_000 });

  await opts.open();
  await s.browser.$(opts.dialogSel).waitForDisplayed({ timeout: 5_000 });
  await Bun.sleep(250);
  const reopened = await rectOf(opts.dialogSel);

  expect(Math.round(reopened.width)).toBe(Math.round(after.width));
  expect(Math.round(reopened.height)).toBe(Math.round(after.height));

  await opts.close();
  await s.browser.$(opts.dialogSel).waitForExist({ reverse: true, timeout: 5_000 });
}

it('QuickOpen (Ctrl+O) se redimensiona y recuerda su tamaño', async () => {
  await verifyDialogResize({
    dialogSel: '.quick-open-container:not(.is-inline)',
    handleSel: '.quick-open-container:not(.is-inline) .dialog-resize.se',
    dx: 50,
    dy: 60,
    open: () => s.browser.keys([Key.Ctrl, 'o']),
    close: () => s.browser.keys(['Escape']),
  });
}, 30_000);

it('FullTextSearch (Ctrl+Shift+F) se redimensiona y recuerda su tamaño', async () => {
  await verifyDialogResize({
    dialogSel: '.fts-container',
    handleSel: '.fts-container .dialog-resize.se',
    dx: 50,
    dy: 60,
    open: () => s.browser.keys([Key.Ctrl, Key.Shift, 'f']),
    close: () => s.browser.keys(['Escape']),
  });
}, 30_000);

it('CommandPalette (Ctrl+P) se redimensiona y recuerda su tamaño', async () => {
  await verifyDialogResize({
    dialogSel: '.palette-container',
    handleSel: '.palette-container .dialog-resize.se',
    dx: 50,
    dy: 60,
    open: () => s.browser.keys([Key.Ctrl, 'p']),
    close: () => s.browser.keys(['Escape']),
  });
}, 30_000);

it('SettingsPanel se redimensiona y recuerda su tamaño', async () => {
  await verifyDialogResize({
    dialogSel: '.settings-panel',
    handleSel: '.settings-panel .dialog-resize.se',
    dx: 50,
    dy: 50,
    open: () => s.browser.$('[aria-label="Configuración"]').click(),
    close: () => s.browser.keys(['Escape']),
  });
}, 30_000);

it('CommunityPluginsModal se redimensiona y recuerda su tamaño', async () => {
  await verifyDialogResize({
    dialogSel: '.plugin-modal-content',
    handleSel: '.plugin-modal-content .dialog-resize.e',
    dx: -60,
    dy: 0,
    open: () => s.browser.$('button[aria-label*="Plugins"]').click(),
    close: () => s.browser.keys(['Escape']),
  });
}, 30_000);

it('Índice de la nota (Ctrl+F12) se redimensiona y recuerda su tamaño', async () => {
  await verifyDialogResize({
    dialogSel: '.outline-dialog',
    handleSel: '.outline-dialog .dialog-resize.se',
    dx: 50,
    dy: 60,
    open: async () => {
      await s.browser.keys([Key.Ctrl, Key.F12]);
      await s.browser.$('.outline-dialog .corner-grip-icon').waitForDisplayed({ timeout: 5_000 });
    },
    close: () => s.browser.keys(['Escape']),
  });
}, 30_000);
