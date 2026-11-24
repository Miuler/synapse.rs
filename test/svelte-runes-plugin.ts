// Precarga de `bun test`: compila los módulos `.svelte.ts` (runas de Svelte como $state) con el
// compilador de Svelte, igual que hace Vite en la app. Sin esto, cualquier prueba que importe un
// store de una entidad falla con "$state is not defined".
import { plugin } from 'bun';

plugin({
  name: 'svelte-runes',
  async setup(build) {
    // TypeScript 7 ya no trae API de JavaScript (`transpileModule`): el TS se quita con el transpilador de Bun
    const transpiler = new Bun.Transpiler({ loader: 'ts' });
    const { compileModule } = await import('svelte/compiler');
    build.onLoad({ filter: /\.svelte\.ts$/ }, async ({ path }) => {
      const source = await Bun.file(path).text();
      const js = transpiler.transformSync(source);
      const compiled = compileModule(js, { filename: path, generate: 'client' });
      return { contents: compiled.js.code, loader: 'js' };
    });
  },
});
