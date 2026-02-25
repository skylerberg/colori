import type { GameState, ColoriChoice, PlayerState, Color } from '../data/types';
import { mixResult } from '../data/colors';
import { getCardData, getBuyerData, getAnyCardData } from '../data/cards';
import init, {
  wasm_create_initial_game_state,
  wasm_execute_draw_phase,
  wasm_apply_choice,
  wasm_confirm_pass,
  wasm_simultaneous_pick,
  wasm_advance_draft,
  wasm_calculate_scores,
} from '../wasm-pkg/colori_wasm.js';

let initialized = false;

export async function initEngine(): Promise<void> {
  if (initialized) return;
  await init();
  initialized = true;
}

export function createInitialGameState(playerNames: string[], aiPlayers?: boolean[]): GameState {
  const ai = aiPlayers ?? playerNames.map(() => false);
  const resultJson = wasm_create_initial_game_state(
    JSON.stringify(playerNames),
    JSON.stringify(ai),
  );
  return JSON.parse(resultJson);
}

export function executeDrawPhase(state: GameState): void {
  const resultJson = wasm_execute_draw_phase(JSON.stringify(state));
  const newState: GameState = JSON.parse(resultJson);
  Object.assign(state, newState);
}

export function applyChoice(state: GameState, choice: ColoriChoice): void {
  const resultJson = wasm_apply_choice(JSON.stringify(state), JSON.stringify(choice));
  const newState: GameState = JSON.parse(resultJson);
  Object.assign(state, newState);
}

export function confirmPass(state: GameState): void {
  const resultJson = wasm_confirm_pass(JSON.stringify(state));
  const newState: GameState = JSON.parse(resultJson);
  Object.assign(state, newState);
}

export function simultaneousPick(state: GameState, playerIndex: number, cardInstanceId: number): void {
  const resultJson = wasm_simultaneous_pick(JSON.stringify(state), playerIndex, cardInstanceId);
  const newState: GameState = JSON.parse(resultJson);
  Object.assign(state, newState);
}

export function advanceDraft(state: GameState): void {
  const resultJson = wasm_advance_draft(JSON.stringify(state));
  const newState: GameState = JSON.parse(resultJson);
  Object.assign(state, newState);
}

export function calculateScores(players: PlayerState[]): { name: string; score: number }[] {
  const resultJson = wasm_calculate_scores(JSON.stringify(players));
  return JSON.parse(resultJson);
}

export function cloneGameState(state: GameState): GameState {
  return JSON.parse(JSON.stringify(state));
}

function assertNever(x: never): never {
  throw new Error(`Unhandled choice type: ${(x as { type: string }).type}`);
}

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
      const cardName = card ? (getAnyCardData(card.card) as { name?: string })?.name ?? 'a card' : 'a card';
      return `${name} destroyed ${cardName} from drafted cards`;
    }
    case 'endTurn':
      return `${name} ended their turn`;
    case 'workshop': {
      const cardNames = choice.cardInstanceIds.map(id => {
        const c = player.workshopCards.find(c => c.instanceId === id);
        return c ? (getAnyCardData(c.card) as { name?: string })?.name ?? 'a card' : 'a card';
      });
      return `${name} workshopped ${cardNames.join(', ')}`;
    }
    case 'skipWorkshop':
      return `${name} skipped workshop`;
    case 'destroyDrawnCards': {
      const cardNames = choice.cardInstanceIds.map(id => {
        const c = player.workshopCards.find(c => c.instanceId === id);
        return c ? (getAnyCardData(c.card) as { name?: string })?.name ?? 'a card' : 'a card';
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
      return `${name} sold to a ${buyer ? getBuyerData(buyer.card).stars : '?'}-star buyer`;
    }
    case 'gainSecondary':
      return `${name} gained ${choice.color}`;
    case 'gainPrimary':
      return `${name} gained ${choice.color}`;
    case 'chooseTertiaryToLose':
      return `${name} lost ${choice.color}`;
    case 'chooseTertiaryToGain':
      return `${name} gained ${choice.color}`;
    case 'mixAll': {
      if (choice.mixes.length === 0) return `${name} skipped remaining mixes`;
      const parts = choice.mixes.map(([a, b]) => `mixed ${a} + ${b} to make ${mixResult(a, b)}`);
      return `${name} ${parts.join(', ')}`;
    }
    case 'swapTertiary':
      return `${name} swapped ${choice.loseColor} for ${choice.gainColor}`;
    case 'destroyAndMixAll': {
      const card = player.draftedCards.find(c => c.instanceId === choice.cardInstanceId);
      const cardName = card ? (getAnyCardData(card.card) as { name?: string })?.name ?? 'a card' : 'a card';
      let msg = `${name} destroyed ${cardName} from drafted cards`;
      if (choice.mixes.length > 0) {
        const parts = choice.mixes.map(([a, b]) => `mixed ${a} + ${b} to make ${mixResult(a, b)}`);
        msg += `, ${parts.join(', ')}`;
      }
      return msg;
    }
    case 'destroyAndSell': {
      const card = player.draftedCards.find(c => c.instanceId === choice.cardInstanceId);
      const cardName = card ? (getAnyCardData(card.card) as { name?: string })?.name ?? 'a card' : 'a card';
      const buyer = state.buyerDisplay.find(g => g.instanceId === choice.buyerInstanceId);
      return `${name} destroyed ${cardName} from drafted cards, sold to a ${buyer ? getBuyerData(buyer.card).stars : '?'}-star buyer`;
    }
    default:
      return assertNever(choice);
  }
}

export function determineWinner(players: PlayerState[]): string {
  const scores = calculateScores(players);
  scores.sort((a, b) => b.score - a.score);
  return scores[0].name;
}

function canPayCost(wheel: Record<Color, number>, cost: Color[]): boolean {
  const used: Partial<Record<Color, number>> = {};
  for (const c of cost) {
    const needed = (used[c] ?? 0) + 1;
    if (wheel[c] < needed) return false;
    used[c] = needed;
  }
  return true;
}

export function canSell(state: GameState, buyerInstanceId: number): boolean {
  if (state.phase.type !== 'action') return false;
  const player = state.players[state.phase.actionState.currentPlayerIndex];
  const buyerInstance = state.buyerDisplay.find(g => g.instanceId === buyerInstanceId);
  if (!buyerInstance) return false;
  const buyer = getBuyerData(buyerInstance.card);
  if (player.materials[buyer.requiredMaterial] < 1) return false;
  return canPayCost(player.colorWheel, buyer.colorCost);
}
