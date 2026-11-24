// La interfaz solo accede a la bóveda y a los archivos elegidos; lo demás del disco queda fuera.
import { afterAll, expect, it } from 'bun:test';
import { mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startAppOnFile, startAppOnVault, type Session } from './harness';

const PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==',
  'base64',
);
const base = mkdtempSync(join(tmpdir(), 'synapse-e2e-paths-'));
afterAll(() => rmSync(base, { recursive: true, force: true }));

/** `ok` o el error de un comando de Tauri llamado desde la página, como haría un script inyectado. */
const call = (s: Session, cmd: string, args: unknown) =>
  s.browser.execute(
    async (c, a) => {
      try {
        const r = await (window as any).__TAURI_INTERNALS__.invoke(c, a);
        return r === null && c.includes('asset') ? 'nada' : 'ok';
      } catch (e) {
        return `error: ${e}`;
      }
    },
    cmd,
    args,
  ) as Promise<string>;

/** ¿El webview puede mostrar la imagen por `asset:`? */
const shows = (s: Session, path: string) =>
  s.browser.execute(
    (p) =>
      new Promise<boolean>((resolve) => {
        const img = new Image();
        img.onload = () => resolve(true);
        img.onerror = () => resolve(false);
        img.src = (window as any).__TAURI_INTERNALS__.convertFileSrc(p, 'asset');
      }),
    path,
  ) as Promise<boolean>;

it('con bóveda: dentro sí, fuera no', async () => {
  const vault = join(base, 'boveda');
  const outside = join(base, 'fuera');
  mkdirSync(join(vault, '.synapse'), { recursive: true });
  mkdirSync(join(outside, 'enlazada'), { recursive: true });
  mkdirSync(join(outside, 'otra', '.synapse'), { recursive: true });
  writeFileSync(join(vault, 'nota.md'), '# Nota\n');
  writeFileSync(join(vault, 'dentro.png'), PNG);
  writeFileSync(join(outside, 'foto.png'), PNG);
  writeFileSync(join(outside, 'secreto.md'), 'secreto');
  writeFileSync(join(outside, 'enlazada', 'img.png'), PNG);
  symlinkSync(join(outside, 'enlazada'), join(vault, 'enlace'));

  // La persona rechaza todas las confirmaciones
  process.env.SYNAPSE_E2E_CONFIRM = 'no';
  const s = await startAppOnVault(vault).finally(() => delete process.env.SYNAPSE_E2E_CONFIRM);
  try {
    await s.browser.$('.empty-workspace').waitForExist({ timeout: 30_000 });
    expect(await shows(s, join(vault, 'dentro.png'))).toBe(true);
    expect(await shows(s, join(outside, 'foto.png'))).toBe(false);
    // Un enlace simbólico de la bóveda se autoriza al resolverlo
    expect(await call(s, 'resolve_asset_file_path', { src: 'enlace/img.png', baseFile: null })).toBe('ok');
    expect(await shows(s, join(vault, 'enlace/img.png'))).toBe(true);

    const secret = join(outside, 'secreto.md');
    expect(await call(s, 'read_note_content', { relativePath: join(vault, 'nota.md') })).toBe('ok');
    expect(await call(s, 'read_note_content', { relativePath: secret })).toContain('no autorizada');
    expect(
      await call(s, 'save_note_content', { relativePath: join(outside, 'nuevo.md'), title: 'x', content: 'x' }),
    ).toContain('no autorizada');
    expect(existsSync(join(outside, 'nuevo.md'))).toBe(false);
    for (const cmd of ['resolve_asset_file_path', 'read_asset_data_url']) {
      expect(await call(s, cmd, { src: join(outside, 'foto.png'), baseFile: null })).toBe('nada');
      expect(await call(s, cmd, { src: '../fuera/foto.png', baseFile: null })).toBe('nada');
    }
    expect(await call(s, 'spreadsheet_open', { path: secret })).toContain('no autorizada');
    expect(await call(s, 'open_file_with_default_app', { path: secret })).toContain('no autorizada');
    expect(await call(s, 'copy_files_to_system_clipboard', { paths: [secret] })).toContain('no autorizada');
    expect(await call(s, 'import_external_items', { paths: [secret], destDir: '' })).toContain('portapapeles');
    expect(await call(s, 'get_vault_git_status', { folderPath: outside })).toContain('no autorizada');
    expect(await call(s, 'get_installed_community_plugins', { vaultPath: join(outside, 'otra') })).toContain('no autorizada');
    expect(await call(s, 'set_active_vault_path', { newPath: join(outside, 'otra') })).toContain('Cancelado');
    expect(await call(s, 'set_active_vault_path', { newPath: vault })).toBe('ok');
    // Sin confirmación no se guarda un agente ni se ejecuta otro Git
    const ran = join(base, 'ejecutado');
    const program = join(outside, 'git');
    writeFileSync(program, `#!/bin/sh\ntouch '${ran}'\necho git version 9\n`, { mode: 0o755 });
    expect(await call(s, 'set_git_path', { path: program })).toContain('Cancelado');
    expect(existsSync(ran)).toBe(false);
    expect(await call(s, 'set_git_path', { path: null })).toBe('ok');
    expect(
      await call(s, 'agent_save_profile', { id: 'x', name: 'x', command: program, args: [], env: {} }),
    ).toContain('Cancelado');
    expect(await call(s, 'list_vault_directory', { relativePath: '../fuera' })).toContain('no permitida');
  } finally {
    await s.close();
  }
}, 120_000);

it('archivo suelto: se ve él y las imágenes de su carpeta, nada más', async () => {
  const dir = join(base, 'suelto');
  mkdirSync(join(dir, 'img'), { recursive: true });
  writeFileSync(join(dir, 'nota.md'), '# Suelta\n\n![](img/foto.png)\n');
  writeFileSync(join(dir, 'img', 'foto.png'), PNG);
  writeFileSync(join(dir, 'otra.md'), 'otra');
  writeFileSync(join(base, 'arriba.png'), PNG);

  const note = join(dir, 'nota.md');
  const s = await startAppOnFile(note);
  try {
    await s.browser.$('.markdown-body h1, .cm-content').waitForExist({ timeout: 30_000 });
    expect(await call(s, 'read_note_content', { relativePath: note })).toBe('ok');
    expect(await call(s, 'read_note_content', { relativePath: join(dir, 'otra.md') })).toContain('no autorizada');
    // La imagen de la nota llega a pintarse de verdad
    await s.browser.waitUntil(
      () =>
        s.browser.execute(() =>
          [...document.querySelectorAll('img')].some((i) => decodeURIComponent(i.src).includes('img/foto.png') && i.naturalWidth > 0),
        ) as Promise<boolean>,
      { timeout: 10_000, timeoutMsg: 'la imagen de la nota suelta no se mostró' },
    );
    expect(await shows(s, join(base, 'arriba.png'))).toBe(false);
    expect(await call(s, 'resolve_asset_file_path', { src: '../arriba.png', baseFile: note })).toBe('nada');
    expect(await call(s, 'resolve_asset_file_path', { src: 'otra.md', baseFile: note })).toBe('nada');
  } finally {
    await s.close();
  }
}, 120_000);
