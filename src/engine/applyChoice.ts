import type { GameState, Color, ColoriChoice, PlayerState, GamePhase } from '../data/types';
import { mixResult } from '../data/colors';
import { playerPick } from './draftPhase';
import {
  destroyDraftedCard, endPlayerTurn, resolveWorkshopChoice,
  resolveMixColors, skipMix, skipWorkshop, resolveDestroyCards,
  resolveSelectBuyer, resolveGainSecondary, resolveGainPrimary,
  resolveChooseTertiaryToLose, resolveChooseTertiaryToGain,
} from './actionPhase';

function assertNever(x: never): never {
  throw new Error(`Unhandled choice type: ${(x as { type: string }).type}`);
}

/**
 * Generate a human-readable log message for a choice, reading pre-mutation state.
 * Returns null for choices that don't need logging (e.g. draftPick).
 */
export function getChoiceLogMessage(
  state: GameState,
  choice: ColoriChoice,
  playerIndex: number,
): string | null {
  const name = state.players[playerIndex].name;
  const player = state.players[playerIndex];

  switch (choice.type) {
    case 'draftPick':
      return null;
    case 'destroyDraftedCard': {
      const card = player.draftedCards.find(c => c.instanceId === choice.cardInstanceId);
      return `${name} destroyed ${card && 'name' in card.card ? card.card.name : 'a card'} from drafted cards`;
    }
    case 'endTurn':
      return `${name} ended their turn`;
    case 'workshop': {
      const cardNames = choice.cardInstanceIds.map(id => {
        const c = player.workshopCards.find(c => c.instanceId === id);
        return c && 'name' in c.card ? c.card.name : 'a card';
      });
      return `${name} workshopped ${cardNames.join(', ')}`;
    }
    case 'skipWorkshop':
      return `${name} skipped workshop`;
    case 'destroyDrawnCards': {
      const cardNames = choice.cardInstanceIds.map(id => {
        const c = player.workshopCards.find(c => c.instanceId === id);
        return c && 'name' in c.card ? c.card.name : 'a card';
      });
      return `${name} destroyed ${cardNames.join(', ')} from workshop`;
    }
    case 'mix': {
      const result = mixResult(choice.colorA, choice.colorB);
      return `${name} mixed ${choice.colorA} + ${choice.colorB} to make ${result}`;
    }
    case 'skipMix':
      return `${name} skipped remaining mixes`;
    case 'selectBuyer': {
      const buyer = state.buyerDisplay.find(g => g.instanceId === choice.buyerInstanceId);
      return `${name} sold to a ${buyer?.card.stars ?? '?'}-star buyer`;
    }
    case 'gainSecondary':
      return `${name} gained ${choice.color}`;
    case 'gainPrimary':
      return `${name} gained ${choice.color}`;
    case 'chooseTertiaryToLose':
      return `${name} lost ${choice.color}`;
    case 'chooseTertiaryToGain':
      return `${name} gained ${choice.color}`;
    default:
      return assertNever(choice);
  }
}

// ── Deep clone ──

function clonePlayerState(p: PlayerState): PlayerState {
  return {
    name: p.name,
    deck: [...p.deck],
    discard: [...p.discard],
    workshopCards: [...p.workshopCards],
    draftedCards: [...p.draftedCards],
    colorWheel: { ...p.colorWheel },
    ducats: p.ducats,
    materials: { ...p.materials },
    completedBuyers: [...p.completedBuyers],
  };
}

function clonePhase(phase: GamePhase): GamePhase {
  switch (phase.type) {
    case 'draw':
      return { type: 'draw' };
    case 'draft': {
      const ds = phase.draftState;
      return {
        type: 'draft',
        draftState: {
          pickNumber: ds.pickNumber,
          currentPlayerIndex: ds.currentPlayerIndex,
          hands: ds.hands.map(h => [...h]),
          direction: ds.direction,
          waitingForPass: ds.waitingForPass,
        },
      };
    }
    case 'action': {
      const as_ = phase.actionState;
      return {
        type: 'action',
        actionState: {
          currentPlayerIndex: as_.currentPlayerIndex,
          abilityStack: as_.abilityStack.map(a => ({ ...a })),
          pendingChoice: as_.pendingChoice ? { ...as_.pendingChoice } : null,
        },
      };
    }
    case 'gameOver':
      return { type: 'gameOver' };
  }
}

export function cloneGameState(state: GameState): GameState {
  return {
    players: state.players.map(clonePlayerState),
    draftDeck: [...state.draftDeck],
    destroyedPile: [...state.destroyedPile],
    buyerDeck: [...state.buyerDeck],
    buyerDisplay: [...state.buyerDisplay],
    phase: clonePhase(state.phase),
    round: state.round,
    aiPlayers: [...state.aiPlayers],
  };
}

/**
 * Apply a choice to the game state. Dispatches to the appropriate engine function.
 * No post-processing (no draft pass confirmation, no draw phase execution).
 */
export function applyChoice(state: GameState, choice: ColoriChoice): void {
  switch (choice.type) {
    case 'draftPick':
      playerPick(state, choice.cardInstanceId);
      break;
    case 'destroyDraftedCard':
      destroyDraftedCard(state, choice.cardInstanceId);
      break;
    case 'endTurn':
      endPlayerTurn(state);
      break;
    case 'workshop':
      resolveWorkshopChoice(state, choice.cardInstanceIds);
      break;
    case 'skipWorkshop':
      skipWorkshop(state);
      break;
    case 'destroyDrawnCards':
      resolveDestroyCards(state, choice.cardInstanceIds);
      break;
    case 'mix':
      resolveMixColors(state, choice.colorA, choice.colorB);
      break;
    case 'skipMix':
      skipMix(state);
      break;
    case 'selectBuyer':
      resolveSelectBuyer(state, choice.buyerInstanceId);
      break;
    case 'gainSecondary':
      resolveGainSecondary(state, choice.color);
      break;
    case 'gainPrimary':
      resolveGainPrimary(state, choice.color);
      break;
    case 'chooseTertiaryToLose':
      resolveChooseTertiaryToLose(state, choice.color);
      break;
    case 'chooseTertiaryToGain':
      resolveChooseTertiaryToGain(state, choice.color);
      break;
    default:
      assertNever(choice);
  }
}
