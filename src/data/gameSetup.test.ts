import { describe, expect, it } from 'vitest';
import { DEFAULT_AI_ITERATIONS } from './aiDifficulty';
import { type GameSetup, restoreSetup } from './gameSetup';

describe('restoreSetup', () => {
  it('starts a first game with one human against AI at the default difficulty', () => {
    expect(restoreSetup(null)).toEqual({
      playerCount: 2,
      isAI: [false, true, true, true],
      aiIterations: Array(4).fill(DEFAULT_AI_ITERATIONS),
      names: [null, null, null, null],
      params: 'default',
      rolloutLookahead: null,
    });
  });

  it('gives back a setup it saved unchanged', () => {
    const setup: GameSetup = {
      playerCount: 3,
      isAI: [false, false, true, true],
      aiIterations: [100000, 25000, 1000, 4000],
      names: ['Skyler', '', null, null],
      params: 'lki08w-gen-32',
      rolloutLookahead: 6,
    };
    expect(restoreSetup(JSON.parse(JSON.stringify(setup)))).toEqual(setup);
  });

  it('replaces what the screen no longer offers with its default', () => {
    const restored = restoreSetup({
      playerCount: 5,
      isAI: [true],
      aiIterations: [80000, 'hard'],
      names: [7],
      params: 'retired-gen-3',
      rolloutLookahead: 9,
    });
    expect(restored).toEqual({
      playerCount: 2,
      isAI: [true, true, true, true],
      aiIterations: [100000, DEFAULT_AI_ITERATIONS, DEFAULT_AI_ITERATIONS, DEFAULT_AI_ITERATIONS],
      names: [null, null, null, null],
      params: 'default',
      rolloutLookahead: null,
    });
  });

  it('treats anything that is not a setup as no setup', () => {
    for (const stored of [undefined, 'setup', 3, []]) {
      expect(restoreSetup(stored)).toEqual(restoreSetup(null));
    }
  });
});
