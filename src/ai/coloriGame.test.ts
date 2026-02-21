import { describe, it, expect, beforeEach } from 'vitest';
import { ColoriGame, cloneGameState } from './coloriGame';
import { ismcts } from './ismcts';
import { createInitialGameState } from '../engine/setupPhase';
import { executeDrawPhase } from '../engine/drawPhase';
import { resetInstanceIdCounter } from '../engine/deckUtils';

function setupGame(numPlayers: number = 2) {
  resetInstanceIdCounter();
  const names = Array.from({ length: numPlayers }, (_, i) => `Player ${i + 1}`);
  const state = createInitialGameState(names, names.map(() => true));
  executeDrawPhase(state);
  return state;
}

describe('ColoriGame', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('enumerates draft choices correctly', () => {
    const state = setupGame(2);
    const game = new ColoriGame(state);

    const status = game.status();
    expect(status.type).toBe('awaitingAction');
    if (status.type === 'awaitingAction') {
      expect(status.playerId).toBe(0);
    }

    const choices = game.getAllChoices();
    // Should have 5 draft picks (one per card in hand)
    expect(choices).toHaveLength(5);
    expect(choices.every(c => c.type === 'draftPick')).toBe(true);
  });

  it('can apply a draft pick choice', () => {
    const state = setupGame(2);
    const game = new ColoriGame(state);

    const choices = game.getAllChoices();
    game.applyChoice(choices[0]);

    // After pick + auto-confirmPass, next player should be active
    const status = game.status();
    expect(status.type).toBe('awaitingAction');
    if (status.type === 'awaitingAction') {
      expect(status.playerId).toBe(1);
    }
  });

  it('cloneGameState creates an independent copy', () => {
    const state = setupGame(2);
    const clone = cloneGameState(state);

    // Modifying clone should not affect original
    clone.players[0].name = 'Modified';
    expect(state.players[0].name).toBe('Player 1');

    clone.draftDeck.pop();
    expect(clone.draftDeck.length).toBe(state.draftDeck.length - 1);
  });

  it('getDeterminization preserves perspective player hand', () => {
    const state = setupGame(2);
    const game = new ColoriGame(state);

    if (state.phase.type !== 'draft') throw new Error('Expected draft phase');
    const originalHand = [...state.phase.draftState.hands[0]];

    const det = game.getDeterminization(0) as ColoriGame;
    if (det.state.phase.type !== 'draft') throw new Error('Expected draft phase');
    const detHand = det.state.phase.draftState.hands[0];

    // Perspective player's hand should be preserved
    expect(detHand.map(c => c.instanceId).sort()).toEqual(
      originalHand.map(c => c.instanceId).sort()
    );
  });

  it('choiceIsAvailable returns true for valid draft picks', () => {
    const state = setupGame(2);
    const game = new ColoriGame(state);

    const choices = game.getAllChoices();
    for (const choice of choices) {
      expect(game.choiceIsAvailable(choice)).toBe(true);
    }
  });

  it('choiceKey produces unique keys for different choices', () => {
    const state = setupGame(2);
    const game = new ColoriGame(state);

    const choices = game.getAllChoices();
    const keys = new Set(choices.map(c => game.choiceKey(c)));
    expect(keys.size).toBe(choices.length);
  });
});

describe('ISMCTS with ColoriGame', () => {
  it('selects a valid draft pick with small iteration count', () => {
    resetInstanceIdCounter();
    const state = setupGame(2);
    const game = new ColoriGame(state);

    if (state.phase.type !== 'draft') throw new Error('Expected draft phase');
    const validCardIds = state.phase.draftState.hands[0].map(c => c.instanceId);

    // Run with small number of iterations for speed
    const choice = ismcts(game, 50);

    expect(choice.type).toBe('draftPick');
    if (choice.type === 'draftPick') {
      expect(validCardIds).toContain(choice.cardInstanceId);
    }
  });
});

describe('Full AI game simulation', () => {
  it('completes a game with all AI players without errors', () => {
    resetInstanceIdCounter();
    const state = createInitialGameState(['AI 1', 'AI 2'], [true, true]);
    executeDrawPhase(state);

    let steps = 0;
    const maxSteps = 5000;

    while (state.phase.type !== 'gameOver' && steps < maxSteps) {
      const game = new ColoriGame(cloneGameState(state));
      const choice = ismcts(game, 10); // Very low iterations for speed

      // Apply choice to actual state
      const applyGame = new ColoriGame(state);
      applyGame.applyChoice(choice);
      steps++;
    }

    expect(state.phase.type).toBe('gameOver');
  }, 60000); // 60s timeout for full game
});
