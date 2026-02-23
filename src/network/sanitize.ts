import type { GameState } from '../data/types';
import type { SanitizedGameState, SanitizedPlayerState, SanitizedGamePhase } from './types';

export function sanitizeGameState(
  fullState: GameState,
  forPlayerIndex: number,
  newLogEntries: string[] = [],
): SanitizedGameState {
  const players: SanitizedPlayerState[] = fullState.players.map(p => ({
    name: p.name,
    deckCount: p.deck.length,
    discardCount: p.discard.length,
    drawnCards: [...p.drawnCards],
    draftedCards: [...p.draftedCards],
    colorWheel: { ...p.colorWheel },
    materials: { ...p.materials },
    completedGarments: [...p.completedGarments],
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
    players,
    draftDeckCount: fullState.draftDeck.length,
    destroyedPile: [...fullState.destroyedPile],
    garmentDeckCount: fullState.garmentDeck.length,
    garmentDisplay: [...fullState.garmentDisplay],
    phase,
    round: fullState.round,
    aiPlayers: [...fullState.aiPlayers],
    myPlayerIndex: forPlayerIndex,
    logEntries: [...newLogEntries],
  };
}
