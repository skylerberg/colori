import type { GameState, PlayerState, GamePhase, CardInstance, BuyerInstance } from '../data/types';
import type { SanitizedGameState } from './types';

export function sanitizedToGameState(sanitized: SanitizedGameState): GameState {
  const players: PlayerState[] = sanitized.players.map(sp => ({
    // Owned player: real segments. Opponents: one length-only sparse segment,
    // since the UI reads only the total for them.
    deck: sp.deck.length > 0 ? sp.deck : [new Array(sp.deckCount) as CardInstance[]],
    buyers: sp.buyers,
    usedCards: sp.usedCards,
    workshopCards: sp.workshopCards,
    topCards: sp.topCards,
    colorWheel: sp.colorWheel,
    materialsWheel: sp.materialsWheel,
    completedOrders: sp.completedOrders,
    goldCoins: sp.goldCoins,
    bowls: sp.bowls,
    boats: sp.boats,
  }));

  let phase: GamePhase;
  if (sanitized.phase.type === 'draft') {
    phase = {
      type: 'draft',
      draftState: { ...sanitized.phase.draftState },
    };
  } else if (sanitized.phase.type === 'buyers') {
    phase = {
      type: 'buyers',
      buyersState: { ...sanitized.phase.buyersState },
    };
  } else if (sanitized.phase.type === 'action') {
    phase = {
      type: 'action',
      actionState: { ...sanitized.phase.actionState },
    };
  } else {
    phase = { type: 'gameOver' };
  }

  return {
    playerNames: sanitized.playerNames,
    players,
    draftDeck: new Array(sanitized.draftDeckCount) as CardInstance[],
    trash: sanitized.trash,
    buyerDeck: new Array(sanitized.buyerDeckCount) as BuyerInstance[],
    buyerDisplay: sanitized.buyerDisplay,
    phase,
    round: sanitized.round,
    maxRounds: sanitized.maxRounds,
    botPlayers: sanitized.botPlayers,
  };
}
