import { afterAll, beforeAll, expect, it } from 'bun:test';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { startApp, type Session } from './harness';

let s: Session;
let wsAlphaGit: string;
let wsBetaPlain: string;
let wsGammaGit: string;
let workspacesJsonPath: string;

function makeGitVault(dir: string, branch: string, noteName: string): void {
  mkdirSync(join(dir, '.git'), { recursive: true });
  writeFileSync(join(dir, '.git', 'HEAD'), `ref: refs/heads/${branch}\n`);
  writeFileSync(join(dir, noteName), `# ${branch}\n`);
}

function makePlainVault(dir: string, noteName: string): void {
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, noteName), '# Plain vault\n');
}

async function openWelcomeScreen(): Promise<void> {
  const btn = await s.browser.$(
    'button[aria-label*="Bóvedas recientes"], button[aria-label*="Recent vaults"]',
  );
  await btn.waitForExist({ timeout: 10_000 });
  await btn.click();
  await s.browser.$('.welcome-root .workspace-card').waitForExist({ timeout: 10_000 });
}

beforeAll(async () => {
  s = await startApp({
    'inicio.md': '# Inicio\n',
  });
  await s.browser.$('.file-tree-item').waitForExist({ timeout: 30_000 });

  const baseDir = dirname(s.vault);
  wsAlphaGit = join(baseDir, 'alpha-git-pinned');
  wsBetaPlain = join(baseDir, 'beta-plain-pinned');
  wsGammaGit = join(baseDir, 'gamma-git-unpinned');

  makeGitVault(wsAlphaGit, 'main', 'alpha.md');
  makePlainVault(wsBetaPlain, 'beta.md');
  makeGitVault(wsGammaGit, 'develop', 'gamma.md');

  workspacesJsonPath = join(s.home, '.config', 'synapsers', 'workspaces.json');
  mkdirSync(dirname(workspacesJsonPath), { recursive: true });

  const now = Date.now();
  writeFileSync(
    workspacesJsonPath,
    JSON.stringify(
      {
        workspaces: [
          {
            path: wsAlphaGit,
            display_path: wsAlphaGit,
            name: 'alpha-git-pinned',
            last_opened_ms: now - 3000,
            pinned: true,
            pinned_order: 0,
            notes_count: 1,
            git_branch: 'main',
            is_git_repo: true,
            exists: true,
          },
          {
            path: wsBetaPlain,
            display_path: wsBetaPlain,
            name: 'beta-plain-pinned',
            last_opened_ms: now - 2000,
            pinned: true,
            pinned_order: 1,
            notes_count: 1,
            git_branch: null,
            is_git_repo: false,
            exists: true,
          },
          {
            path: wsGammaGit,
            display_path: wsGammaGit,
            name: 'gamma-git-unpinned',
            last_opened_ms: now - 1000,
            pinned: false,
            pinned_order: null,
            notes_count: 1,
            git_branch: 'develop',
            is_git_repo: true,
            exists: true,
          },
        ],
        last_active_workspace: s.vault,
      },
      null,
      2,
    ),
  );

  await openWelcomeScreen();
}, 120_000);

afterAll(async () => {
  await s?.close();
});

it('muestra el icono de carpeta git en bóvedas fijadas con git manteniendo el color de fijado', async () => {
  const cardsInfo = await s.browser.execute(() => {
    const cards = Array.from(document.querySelectorAll('.workspace-card'));
    return cards.map((card) => {
      const name = card.querySelector('.ws-name')?.textContent?.trim() ?? '';
      const iconBox = card.querySelector('.ws-icon-box');
      const hasPinnedBox = iconBox?.classList.contains('pinned-box') ?? false;
      const hasTealBox = iconBox?.classList.contains('teal-box') ?? false;
      const hasGitIcon = !!iconBox?.querySelector('svg.lucide-folder-git-2');
      const hasPlainFolderIcon = !!iconBox?.querySelector('svg.lucide-folder');
      return { name, hasPinnedBox, hasTealBox, hasGitIcon, hasPlainFolderIcon };
    });
  });

  const alpha = cardsInfo.find((c) => c.name === 'alpha-git-pinned');
  const beta = cardsInfo.find((c) => c.name === 'beta-plain-pinned');
  const gamma = cardsInfo.find((c) => c.name === 'gamma-git-unpinned');

  expect(alpha).toEqual({
    name: 'alpha-git-pinned',
    hasPinnedBox: true,
    hasTealBox: false,
    hasGitIcon: true,
    hasPlainFolderIcon: false,
  });
  expect(beta).toEqual({
    name: 'beta-plain-pinned',
    hasPinnedBox: true,
    hasTealBox: false,
    hasGitIcon: false,
    hasPlainFolderIcon: true,
  });
  expect(gamma).toEqual({
    name: 'gamma-git-unpinned',
    hasPinnedBox: false,
    hasTealBox: true,
    hasGitIcon: true,
    hasPlainFolderIcon: false,
  });
}, 30_000);

it('permite reordenar bóvedas fijadas con drag and drop y persiste el orden', async () => {
  const getNames = () =>
    s.browser.execute(() =>
      Array.from(document.querySelectorAll('.workspace-card .ws-name')).map(
        (el) => el.textContent?.trim() ?? '',
      ),
    ) as Promise<string[]>;

  const initialNames = await getNames();
  expect(initialNames).toEqual([
    'alpha-git-pinned',
    'beta-plain-pinned',
    'gamma-git-unpinned',
  ]);

  await s.browser.execute(() => {
    const cards = Array.from(document.querySelectorAll('.workspace-card'));
    const target = cards.find(
      (c) => c.querySelector('.ws-name')?.textContent?.trim() === 'alpha-git-pinned',
    ) as HTMLElement | undefined;
    const source = cards.find(
      (c) => c.querySelector('.ws-name')?.textContent?.trim() === 'beta-plain-pinned',
    ) as HTMLElement | undefined;
    if (!target || !source) throw new Error('Tarjetas no encontradas');

    const dt = new DataTransfer();
    const targetRect = target.getBoundingClientRect();

    source.dispatchEvent(
      new DragEvent('dragstart', { bubbles: true, cancelable: true, dataTransfer: dt }),
    );
    target.dispatchEvent(
      new DragEvent('dragover', {
        bubbles: true,
        cancelable: true,
        clientY: targetRect.top + 5,
        dataTransfer: dt,
      }),
    );
    target.dispatchEvent(
      new DragEvent('drop', {
        bubbles: true,
        cancelable: true,
        clientY: targetRect.top + 5,
        dataTransfer: dt,
      }),
    );
    source.dispatchEvent(
      new DragEvent('dragend', { bubbles: true, cancelable: true, dataTransfer: dt }),
    );
  });

  await s.browser.waitUntil(
    async () => {
      const names = await getNames();
      return names[0] === 'beta-plain-pinned' && names[1] === 'alpha-git-pinned';
    },
    { timeout: 10_000, timeoutMsg: 'El orden de tarjetas fijadas no se actualizó tras drag-and-drop' },
  );

  // Esperar a que workspaces.json en disco persista pinned_order
  await s.browser.waitUntil(
    () => {
      try {
        const raw = JSON.parse(readFileSync(workspacesJsonPath, 'utf8'));
        const beta = raw.workspaces?.find((w: { name: string }) => w.name === 'beta-plain-pinned');
        const alpha = raw.workspaces?.find((w: { name: string }) => w.name === 'alpha-git-pinned');
        return beta?.pinned_order === 0 && alpha?.pinned_order === 1;
      } catch {
        return false;
      }
    },
    { timeout: 10_000, timeoutMsg: 'pinned_order no se persistió en workspaces.json' },
  );

  // Cerrar y volver a abrir WelcomeScreen para confirmar que el backend mantiene el nuevo orden
  await s.browser.keys(['Escape']);
  await s.browser.$('.welcome-root').waitForExist({ reverse: true, timeout: 5_000 });
  await openWelcomeScreen();

  const persistedNames = await getNames();
  expect(persistedNames).toEqual([
    'beta-plain-pinned',
    'alpha-git-pinned',
    'gamma-git-unpinned',
  ]);
}, 30_000);
