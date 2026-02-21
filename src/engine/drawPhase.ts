import type { GameState } from '../data/types';
import { drawFromDeck } from './deckUtils';
import { initializeDraft } from './draftPhase';

/**
 * Execute the draw phase: each player draws 5 cards from their personal deck
 * into drawnCards. Then transition to draft phase.
 */
export function executeDrawPhase(state: GameState): void {
  for (const player of state.players) {
    const drawn = drawFromDeck(player, 5);
    player.drawnCards.push(...drawn);
  }
  initializeDraft(state);
}
