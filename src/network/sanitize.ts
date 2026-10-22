import type { GameState } from '../data/types';
import type { SanitizedGameState, SanitizedPlayerState, SanitizedGamePhase } from './types';
import type { LogEntry } from '../logEntries';

export function sanitizeGameState(
  fullState: GameState,
  forPlayerIndex: number,
  newLogEntries: LogEntry[] = [],
): SanitizedGameState {
  const players: SanitizedPlayerState[] = fullState.players.map((p, i) => {
    const isOwner = i === forPlayerIndex;
    return {
      deckCount: p.deck.reduce((n, segment) => n + segment.length, 0),
      deck: isOwner ? p.deck.map(segment => [...segment]) : [],
      buyers: [...p.buyers],
      usedCards: [...p.usedCards],
      workshopCards: [...p.workshopCards],
      topCards: [...p.topCards],
      colorWheel: { ...p.colorWheel },
      materials: { ...p.materials },
      completedBuyers: [...p.completedBuyers],
      goldCoins: p.goldCoins,
      bowls: p.bowls,
      boats: p.boats,
    };
  });

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
    // Everyone's turn is public, as it is played.
    phase = {
      type: 'action',
      actionState: {
        turns: fullState.phase.actionState.turns.map(turn => ({
          abilityStack: turn.abilityStack.map(a => ({ ...a })),
          ended: turn.ended,
        })),
      },
    };
  } else if (fullState.phase.type === 'buyers') {
    phase = {
      type: 'buyers',
      buyersState: { currentPlayerIndex: fullState.phase.buyersState.currentPlayerIndex },
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
    trash: [...fullState.trash],
    buyerDeckCount: fullState.buyerDeck.length,
    buyerDisplay: [...fullState.buyerDisplay],
    phase,
    round: fullState.round,
    maxRounds: fullState.maxRounds,
    botPlayers: [...fullState.botPlayers],
    myPlayerIndex: forPlayerIndex,
    logEntries: [...newLogEntries],
  };
}
