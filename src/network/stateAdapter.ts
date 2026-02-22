import type { GameState, PlayerState, GamePhase, CardInstance, GarmentCard } from '../data/types';
import type { SanitizedGameState } from './types';

export function sanitizedToGameState(sanitized: SanitizedGameState): GameState {
  const players: PlayerState[] = sanitized.players.map(sp => ({
    name: sp.name,
    deck: new Array(sp.deckCount) as CardInstance[],
    discard: new Array(sp.discardCount) as CardInstance[],
    drawnCards: sp.drawnCards,
    draftedCards: sp.draftedCards,
    colorWheel: sp.colorWheel,
    fabrics: sp.fabrics,
    completedGarments: sp.completedGarments,
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
    garmentDeck: new Array(sanitized.garmentDeckCount) as CardInstance<GarmentCard>[],
    garmentDisplay: sanitized.garmentDisplay,
    phase,
    round: sanitized.round,
    aiPlayers: sanitized.aiPlayers,
  };
}
