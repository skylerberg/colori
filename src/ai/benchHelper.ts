import type { GameState } from '../data/types';
import { createInitialGameState, executeDrawPhase, applyChoice, confirmPass } from '../engine/wasmEngine';

export function setupDraftGame(numPlayers: number): GameState {
  const names = Array.from({ length: numPlayers }, (_, i) => `Player ${i + 1}`);
  const state = createInitialGameState(names, names.map(() => true));
  executeDrawPhase(state);
  return state;
}

export function setupActionGame(numPlayers: number): GameState {
  const state = setupDraftGame(numPlayers);

  // Play through the entire draft: each player picks 4 cards
  while (state.phase.type === 'draft') {
    const ds = state.phase.draftState;

    if (ds.waitingForPass) {
      confirmPass(state);
      continue;
    }

    const hand = ds.hands[ds.currentPlayerIndex];
    if (hand.length === 0) break;

    // Pick the first card in hand
    applyChoice(state, { type: 'draftPick', cardInstanceId: hand[0].instanceId });
  }

  return state;
}
