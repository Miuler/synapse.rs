// Precarga de `bun test`: compila los módulos `.svelte.ts` (runas de Svelte como $state) con el
// compilador de Svelte, igual que hace Vite en la app. Sin esto, cualquier prueba que importe un
// store de una entidad falla con "$state is not defined".
import { plugin } from 'bun';

plugin({
  name: 'svelte-runes',
  async setup(build) {
    const ts = await import('typescript');
    const { compileModule } = await import('svelte/compiler');
    build.onLoad({ filter: /\.svelte\.ts$/ }, async ({ path }) => {
      const source = await Bun.file(path).text();
      const js = ts.transpileModule(source, {
        compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
      }).outputText;
      const compiled = compileModule(js, { filename: path, generate: 'client' });
      return { contents: compiled.js.code, loader: 'js' };
    });
  },
});
