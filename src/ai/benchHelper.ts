import type { GameState } from '../data/types';
import { createInitialGameState, executeDrawPhase, applyChoice } from '../engine/wasmEngine';

export function setupDraftGame(numPlayers: number): GameState {
  const names = Array.from({ length: numPlayers }, (_, i) => `Player ${i + 1}`);
  const state = createInitialGameState(names, names.map(() => true));
  executeDrawPhase(state);
  resolveBuyers(state);
  return state;
}

/// The round opens with the buyers phase; claim the leftmost card each time to
/// get past it.
function resolveBuyers(state: GameState) {
  while (state.phase.type === 'buyers') {
    applyChoice(state, { type: 'takeBuyer', sellCard: state.sellCardDisplay[0].card });
  }
}

export function setupActionGame(numPlayers: number): GameState {
  const state = setupDraftGame(numPlayers);

  // Play through the entire draft: each player picks 4 cards
  while (state.phase.type === 'draft') {
    const ds = state.phase.draftState;

    const hand = ds.hands[ds.currentPlayerIndex];
    if (hand.length === 0) break;

    // Pick the first card in hand
    applyChoice(state, { type: 'draftPick', card: hand[0].card });
  }

  return state;
}
