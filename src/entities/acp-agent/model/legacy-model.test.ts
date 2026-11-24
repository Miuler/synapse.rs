import { expect, it } from 'bun:test';
import { savedModelChoice } from './legacy-model';

const o = (id: string) => ({ id, name: id });

it('reconoce el modelo recordado, también con el esfuerzo en el identificador', () => {
  const models = [o('neon'), o('neon-pro')];
  const efforts = [o('low'), o('high')];
  expect(savedModelChoice('neon', models, efforts)).toEqual({ model: 'neon', effort: null });
  expect(savedModelChoice('neon-pro-high', models, efforts)).toEqual({ model: 'neon-pro', effort: 'high' });
  expect(savedModelChoice('otro', models, efforts)).toBeNull();
});
