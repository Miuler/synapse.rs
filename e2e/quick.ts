// `bun run test:e2e:quick [nombre…]`: corre smoke más las e2e tocadas (cambiadas sin commitear o
// en el último commit) y las que se nombren (`drawio`, `compare` → todas las que lo contengan).
// La suite completa sigue siendo `bun run test:e2e`.
import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';

const git = (...args: string[]) => execFileSync('git', args, { encoding: 'utf8' }).split('\n').filter(Boolean);
const all = readdirSync('e2e').filter((f) => f.endsWith('.test.ts'));
const touched = [
  ...git('diff', '--name-only', 'HEAD'),
  ...git('ls-files', '--others', '--exclude-standard'),
  ...git('diff', '--name-only', 'HEAD~1', 'HEAD'),
]
  .filter((f) => f.startsWith('e2e/') && f.endsWith('.test.ts') && existsSync(f))
  .map((f) => f.slice(4));
const named = process.argv.slice(2).flatMap((n) => all.filter((f) => f.includes(n)));
const files = [...new Set(['smoke.test.ts', ...touched, ...named])];

console.log(`e2e rápidas: ${files.join(', ')}`);
// Un archivo por proceso, en serie: así un WebDriver caído no arrastra a los siguientes
let failed = 0;
for (const f of files) {
  const r = spawnSync('bun', ['test', '--timeout', '180000', `./e2e/${f}`], { stdio: 'inherit' });
  if (r.status !== 0) failed++;
}
console.log(failed ? `\n${failed} archivo(s) con fallos` : '\nTodo pasa');
process.exit(failed ? 1 : 0);
