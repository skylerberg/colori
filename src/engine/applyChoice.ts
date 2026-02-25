import type { GameState, Color } from '../data/types';
import type { ColoriChoice } from '../ai/coloriGame';
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
