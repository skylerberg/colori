import { describe, bench } from 'vitest';
import { ColoriGame, cloneGameState } from './coloriGame';
import { ismcts } from './ismcts';
import { setupDraftGame, setupActionGame } from './benchHelper';

describe('Draft Phase', () => {
  const state = setupDraftGame(3);
  const game = new ColoriGame(state);

  bench('cloneGameState', () => {
    cloneGameState(state);
  });

  bench('getDeterminization', () => {
    game.getDeterminization(0);
  });

  bench('getAllChoices', () => {
    game.getAllChoices();
  });

  bench('ismcts 1 iteration', () => {
    const g = new ColoriGame(cloneGameState(state));
    ismcts(g, 1);
  });

  bench('ismcts 100 iterations', () => {
    const g = new ColoriGame(cloneGameState(state));
    ismcts(g, 100);
  });

  bench('ismcts 1000 iterations', () => {
    const g = new ColoriGame(cloneGameState(state));
    ismcts(g, 1000);
  });
});

describe('Action Phase', () => {
  const state = setupActionGame(3);
  const game = new ColoriGame(state);

  bench('cloneGameState', () => {
    cloneGameState(state);
  });

  bench('getDeterminization', () => {
    game.getDeterminization(0);
  });

  bench('getAllChoices', () => {
    game.getAllChoices();
  });

  bench('ismcts 1 iteration', () => {
    const g = new ColoriGame(cloneGameState(state));
    ismcts(g, 1);
  });

  bench('ismcts 100 iterations', () => {
    const g = new ColoriGame(cloneGameState(state));
    ismcts(g, 100);
  });

  bench('ismcts 1000 iterations', () => {
    const g = new ColoriGame(cloneGameState(state));
    ismcts(g, 1000);
  });
});
