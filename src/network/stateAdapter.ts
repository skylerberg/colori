import type { GameState, PlayerState, GamePhase, CardInstance, BuyerInstance } from '../data/types';
import type { SanitizedGameState } from './types';

export function sanitizedToGameState(sanitized: SanitizedGameState): GameState {
  const players: PlayerState[] = sanitized.players.map(sp => ({
    name: sp.name,
    deck: new Array(sp.deckCount) as CardInstance[],
    discard: new Array(sp.discardCount) as CardInstance[],
    workshopCards: sp.workshopCards,
    draftedCards: sp.draftedCards,
    colorWheel: sp.colorWheel,
    materials: sp.materials,
    completedBuyers: sp.completedBuyers,
    ducats: sp.ducats,
  }));

  let phase: GamePhase;
  if (sanitized.phase.type === 'draft') {
    phase = {
      type: 'draft',
      draftState: { ...sanitized.phase.draftState },
    };
  } else if (sanitized.phase.type === 'action') {
    phase = {
      type: 'action',
      actionState: { ...sanitized.phase.actionState },
    };
  } else if (sanitized.phase.type === 'gameOver') {
    phase = { type: 'gameOver' };
  } else {
    phase = { type: 'draw' };
  }

  return {
    players,
    draftDeck: new Array(sanitized.draftDeckCount) as CardInstance[],
    destroyedPile: sanitized.destroyedPile,
    buyerDeck: new Array(sanitized.buyerDeckCount) as BuyerInstance[],
    buyerDisplay: sanitized.buyerDisplay,
    phase,
    round: sanitized.round,
    aiPlayers: sanitized.aiPlayers,
  };
}
