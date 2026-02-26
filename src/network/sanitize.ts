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
    usedCardsCount: p.usedCards.length,
    workshopCards: [...p.workshopCards],
    draftedCards: [...p.draftedCards],
    colorWheel: { ...p.colorWheel },
    materials: { ...p.materials },
    completedBuyers: [...p.completedBuyers],
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
        direction: ds.direction,
        waitingForPass: ds.waitingForPass,
      },
    };
  } else if (fullState.phase.type === 'action') {
    const as_ = fullState.phase.actionState;
    phase = {
      type: 'action',
      actionState: {
        currentPlayerIndex: as_.currentPlayerIndex,
        abilityStack: as_.abilityStack.map(a => ({ ...a })),
        pendingChoice: as_.pendingChoice ? { ...as_.pendingChoice } : null,
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
    buyerDeckCount: fullState.buyerDeck.length,
    buyerDisplay: [...fullState.buyerDisplay],
    phase,
    round: fullState.round,
    aiPlayers: [...fullState.aiPlayers],
    myPlayerIndex: forPlayerIndex,
    logEntries: [...newLogEntries],
  };
}
