import { expect, it } from 'bun:test';
import { droppedFilePaths, endInternalDrag, internalDragPaths, startInternalDrag } from './native-drop';

it('solo rutas absolutas de archivos', () => {
  expect(droppedFilePaths(['/a/b c.md', 'C:\\x\\y.md', '\\\\srv\\z', 'https://x.com/a', 'a.md', ''])).toEqual([
    '/a/b c.md',
    'C:\\x\\y.md',
    '\\\\srv\\z',
  ]);
});

it('el arrastre propio sobrevive un instante al final, y no borra el siguiente', async () => {
  startInternalDrag(['a.md']);
  endInternalDrag();
  expect(internalDragPaths()).toEqual(['a.md']);
  startInternalDrag(['b.md']);
  await Bun.sleep(600);
  expect(internalDragPaths()).toEqual(['b.md']);
  endInternalDrag();
  await Bun.sleep(600);
  expect(internalDragPaths()).toBeNull();
});
