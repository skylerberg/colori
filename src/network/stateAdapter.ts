import type { GameState, PlayerState, GamePhase, CardInstance, SellCardInstance } from '../data/types';
import type { SanitizedGameState } from './types';

export function sanitizedToGameState(sanitized: SanitizedGameState): GameState {
  const players: PlayerState[] = sanitized.players.map(sp => ({
    // Owned player: full cards. Opponents: length-only sparse arrays (UI only reads .length for opponents).
    deck: sp.deck.length > 0 ? sp.deck : (new Array(sp.deckCount) as CardInstance[]),
    discard: sp.discard.length > 0 ? sp.discard : (new Array(sp.discardCount) as CardInstance[]),
    workshoppedCards: sp.workshoppedCards,
    workshopCards: sp.workshopCards,
    draftedCards: sp.draftedCards,
    colorWheel: sp.colorWheel,
    materials: sp.materials,
    completedSellCards: sp.completedSellCards,
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
    playerNames: sanitized.playerNames,
    players,
    draftDeck: new Array(sanitized.draftDeckCount) as CardInstance[],
    destroyedPile: sanitized.destroyedPile,
    sellCardDeck: new Array(sanitized.sellCardDeckCount) as SellCardInstance[],
    sellCardDisplay: sanitized.sellCardDisplay,
    phase,
    round: sanitized.round,
    aiPlayers: sanitized.aiPlayers,
  };
}
