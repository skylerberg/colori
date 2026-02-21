import { describe, it, expect, beforeEach } from 'vitest';
import type { GameState, DraftState } from '../data/types';
import { resetInstanceIdCounter } from './deckUtils';
import { createInitialGameState } from './setupPhase';
import { executeDrawPhase } from './drawPhase';
import {
  initializeDraft,
  playerPick,
  confirmPass,
  isDraftComplete,
} from './draftPhase';

function setupDraftState(playerNames: string[] = ['Alice', 'Bob']): GameState {
  resetInstanceIdCounter();
  const state = createInitialGameState(playerNames);
  executeDrawPhase(state);
  return state;
}

function getDraftState(state: GameState): DraftState {
  return (state.phase as { type: 'draft'; draftState: DraftState }).draftState;
}

describe('initializeDraft', () => {
  it('deals 5 cards per player from the draft deck', () => {
    const state = setupDraftState(['Alice', 'Bob']);
    const draftState = getDraftState(state);
    expect(draftState.hands).toHaveLength(2);
    expect(draftState.hands[0]).toHaveLength(5);
    expect(draftState.hands[1]).toHaveLength(5);
  });

  it('sets direction to 1 for odd rounds', () => {
    const state = setupDraftState();
    const draftState = getDraftState(state);
    expect(draftState.direction).toBe(1);
  });

  it('starts at pickNumber 0 with currentPlayerIndex 0', () => {
    const state = setupDraftState();
    const draftState = getDraftState(state);
    expect(draftState.pickNumber).toBe(0);
    expect(draftState.currentPlayerIndex).toBe(0);
    expect(draftState.waitingForPass).toBe(false);
  });

  it('removes cards from the draft deck', () => {
    resetInstanceIdCounter();
    const state = createInitialGameState(['Alice', 'Bob']);
    const deckSizeBefore = state.draftDeck.length;
    executeDrawPhase(state);
    const draftState = getDraftState(state);
    const totalDealt = draftState.hands.reduce((sum, h) => sum + h.length, 0);
    expect(state.draftDeck.length).toBe(deckSizeBefore - totalDealt);
  });
});

describe('playerPick', () => {
  it('moves the chosen card from draft hand to draftedCards', () => {
    const state = setupDraftState(['Alice', 'Bob']);
    const draftState = getDraftState(state);
    const cardToPick = draftState.hands[0][0];

    playerPick(state, cardToPick.instanceId);

    expect(state.players[0].draftedCards).toHaveLength(1);
    expect(state.players[0].draftedCards[0].instanceId).toBe(cardToPick.instanceId);
    expect(draftState.hands[0]).toHaveLength(4);
  });

  it('sets waitingForPass after first player picks', () => {
    const state = setupDraftState(['Alice', 'Bob']);
    const draftState = getDraftState(state);
    const cardToPick = draftState.hands[0][0];

    playerPick(state, cardToPick.instanceId);

    expect(draftState.waitingForPass).toBe(true);
    expect(draftState.currentPlayerIndex).toBe(1);
  });

  it('throws when card is not in draft hand', () => {
    const state = setupDraftState(['Alice', 'Bob']);
    expect(() => playerPick(state, 99999)).toThrow();
  });
});

describe('confirmPass', () => {
  it('clears the waitingForPass flag', () => {
    const state = setupDraftState(['Alice', 'Bob']);
    const draftState = getDraftState(state);
    const cardToPick = draftState.hands[0][0];

    playerPick(state, cardToPick.instanceId);
    expect(draftState.waitingForPass).toBe(true);

    confirmPass(state);
    expect(draftState.waitingForPass).toBe(false);
  });
});

describe('full draft flow', () => {
  it('completes after 4 pick rounds with 2 players', () => {
    const state = setupDraftState(['Alice', 'Bob']);

    for (let pick = 0; pick < 4; pick++) {
      const draftState = getDraftState(state);

      // Player 0 picks
      const card0 = draftState.hands[0][0];
      playerPick(state, card0.instanceId);

      if (state.phase.type !== 'draft') break; // transitioned to action

      // Confirm pass to Player 1
      confirmPass(state);

      // Player 1 picks
      const updatedDraftState = getDraftState(state);
      const card1 = updatedDraftState.hands[1][0];
      playerPick(state, card1.instanceId);

      // After last player picks, hands rotate (or draft ends)
      if (state.phase.type !== 'draft') break;

      // If not the last round, confirm pass for next round
      if (pick < 3) {
        confirmPass(state);
      }
    }

    // After 4 pick rounds, should transition to action phase
    expect(state.phase.type).toBe('action');
  });

  it('each player ends up with 4 drafted cards after a full draft', () => {
    const state = setupDraftState(['Alice', 'Bob']);

    for (let pick = 0; pick < 4; pick++) {
      const draftState = getDraftState(state);
      const card0 = draftState.hands[0][0];
      playerPick(state, card0.instanceId);

      if (state.phase.type !== 'draft') break;
      confirmPass(state);

      const updatedDraftState = getDraftState(state);
      const card1 = updatedDraftState.hands[1][0];
      playerPick(state, card1.instanceId);

      if (state.phase.type !== 'draft') break;
      if (pick < 3) confirmPass(state);
    }

    expect(state.players[0].draftedCards).toHaveLength(4);
    expect(state.players[1].draftedCards).toHaveLength(4);
  });

  it('remaining undrafted cards go to destroyedPile after draft completes', () => {
    const state = setupDraftState(['Alice', 'Bob']);
    const destroyedBefore = state.destroyedPile.length;

    for (let pick = 0; pick < 4; pick++) {
      const draftState = getDraftState(state);
      const card0 = draftState.hands[0][0];
      playerPick(state, card0.instanceId);

      if (state.phase.type !== 'draft') break;
      confirmPass(state);

      const updatedDraftState = getDraftState(state);
      const card1 = updatedDraftState.hands[1][0];
      playerPick(state, card1.instanceId);

      if (state.phase.type !== 'draft') break;
      if (pick < 3) confirmPass(state);
    }

    // Each player had 5 cards and picked 4, so 1 card per player remains = 2 destroyed
    expect(state.destroyedPile.length).toBe(destroyedBefore + 2);
  });
});

describe('isDraftComplete', () => {
  it('returns false when pickNumber < 4', () => {
    const draftState: DraftState = {
      pickNumber: 3,
      currentPlayerIndex: 0,
      hands: [],
      direction: 1,
      waitingForPass: false,
    };
    expect(isDraftComplete(draftState)).toBe(false);
  });

  it('returns true when pickNumber >= 4', () => {
    const draftState: DraftState = {
      pickNumber: 4,
      currentPlayerIndex: 0,
      hands: [],
      direction: 1,
      waitingForPass: false,
    };
    expect(isDraftComplete(draftState)).toBe(true);
  });
});
