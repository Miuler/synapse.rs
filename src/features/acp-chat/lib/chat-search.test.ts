import { expect, test } from 'bun:test';
import { findAll, foldForSearch } from './chat-search';

test('plegar ignora mayúsculas y tildes sin cambiar la longitud', () => {
  expect(foldForSearch('Conversación ÁÉ')).toBe('conversacion ae');
  for (const t of ['İstanbul', '🧠 año', 'ﬁn', '한글', 'é']) expect(foldForSearch(t)).toHaveLength(t.length);
});

test('encuentra todas las apariciones sin solaparse', () => {
  expect(findAll(foldForSearch('Canción, cancion y CANCIÓN'), 'canción')).toEqual([0, 9, 19]);
  expect(findAll('aaaa', 'aa')).toEqual([0, 2]);
  expect(findAll('algo', '')).toEqual([]);
});
