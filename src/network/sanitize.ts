import type { GameState } from '../data/types';
import type { SanitizedGameState, SanitizedPlayerState, SanitizedGamePhase } from './types';

export function sanitizeGameState(
  fullState: GameState,
  forPlayerIndex: number,
  newLogEntries: string[] = [],
): SanitizedGameState {
  const players: SanitizedPlayerState[] = fullState.players.map(p => ({
    deckCount: p.deck.length,
    discardCount: p.discard.length,
    workshoppedCardsCount: p.workshoppedCards.length,
    workshopCards: [...p.workshopCards],
    draftedCards: [...p.draftedCards],
    colorWheel: { ...p.colorWheel },
    materials: { ...p.materials },
    completedSellCards: [...p.completedSellCards],
    ducats: p.ducats,
  }));

  let phase: SanitizedGamePhase;
  if (fullState.phase.type === 'draft') {
    const ds = fullState.phase.draftState;
    phase = {
      type: 'draft',
      draftState: {
        pickNumber: ds.pickNumber,
        currentPlayerIndex: ds.currentPlayerIndex,
        hands: ds.hands.map((hand, i) =>
          i === forPlayerIndex ? [...hand] : []
        ),
      },
    };
  } else if (fullState.phase.type === 'action') {
    const actionState = fullState.phase.actionState;
    phase = {
      type: 'action',
      actionState: {
        currentPlayerIndex: actionState.currentPlayerIndex,
        abilityStack: actionState.abilityStack.map(a => ({ ...a })),
      },
    };
  } else if (fullState.phase.type === 'gameOver') {
    phase = { type: 'gameOver' };
  } else {
    phase = { type: 'draw' };
  }

  return {
    playerNames: fullState.playerNames,
    players,
    draftDeckCount: fullState.draftDeck.length,
    destroyedPile: [...fullState.destroyedPile],
    sellCardDeckCount: fullState.sellCardDeck.length,
    sellCardDisplay: [...fullState.sellCardDisplay],
    phase,
    round: fullState.round,
    aiPlayers: [...fullState.aiPlayers],
    myPlayerIndex: forPlayerIndex,
    logEntries: [...newLogEntries],
  };
}
