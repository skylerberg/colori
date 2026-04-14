import type { GameState, Choice, PlayerState, Color, Card, DrawEvent } from '../data/types';
import { mixResult } from '../data/colors';
import { getCardData, getSellCardData, getAnyCardData } from '../data/cards';
import init, {
  wasm_create_initial_game_state,
  wasm_execute_draw_phase,
  wasm_apply_choice,
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
    playerNames.length,
    JSON.stringify(ai),
  );
  const state: GameState = JSON.parse(resultJson);
  state.playerNames = playerNames;
  return state;
}

// IMPORTANT: All state-mutating WASM functions below use Object.assign(state, newState)
// instead of direct assignment. This is because Rust's GameState does not include
// TypeScript-only fields like `playerNames`. Object.assign preserves these existing
// properties on `state` that are absent from the WASM output. If you change this to
// direct assignment or structuredClone, those fields will be silently lost.

export function executeDrawPhase(state: GameState): DrawEvent[] {
  const resultJson = wasm_execute_draw_phase(JSON.stringify(state));
  const result: { state: GameState; draws: DrawEvent[] } = JSON.parse(resultJson);
  Object.assign(state, result.state);
  return result.draws;
}

export function applyChoice(state: GameState, choice: Choice): DrawEvent[] {
  const resultJson = wasm_apply_choice(JSON.stringify(state), JSON.stringify(choice));
  const result: { state: GameState; draws: DrawEvent[] } = JSON.parse(resultJson);
  Object.assign(state, result.state);
  return result.draws;
}

export function simultaneousPick(state: GameState, playerIndex: number, card: Card): void {
  const resultJson = wasm_simultaneous_pick(JSON.stringify(state), playerIndex, JSON.stringify(card));
  const newState: GameState = JSON.parse(resultJson);
  Object.assign(state, newState);
}

export function advanceDraft(state: GameState): void {
  const resultJson = wasm_advance_draft(JSON.stringify(state));
  const newState: GameState = JSON.parse(resultJson);
  Object.assign(state, newState);
}

export function calculateScores(players: PlayerState[], playerNames: string[]): { name: string; score: number }[] {
  const resultJson = wasm_calculate_scores(JSON.stringify(players));
  const scores: number[] = JSON.parse(resultJson);
  return scores.map((score, i) => ({ name: playerNames[i], score }));
}

export function cloneGameState(state: GameState): GameState {
  return JSON.parse(JSON.stringify(state));
}

function assertNever(x: never): never {
  throw new Error(`Unhandled choice type: ${(x as { type: string }).type}`);
}

export function getChoiceLogMessage(
  state: GameState,
  choice: Choice,
  playerIndex: number,
): string | null {
  const name = state.playerNames[playerIndex];

  switch (choice.type) {
    case 'draftPick':
      return null;
    case 'destroyDraftedCard': {
      const cardName = (getAnyCardData(choice.card) as { name?: string })?.name ?? 'a card';
      return `${name} destroyed ${cardName} from drafted cards`;
    }
    case 'endTurn':
      return `${name} ended their turn`;
    case 'workshop': {
      const cardNames = choice.cardTypes.map(c => (getAnyCardData(c) as { name?: string })?.name ?? 'a card');
      return `${name} workshopped ${cardNames.join(', ')}`;
    }
    case 'skipWorkshop':
      return `${name} skipped workshop`;
    case 'destroyDrawnCards': {
      if (choice.card === null) return `${name} did not move a card to draft pool`;
      const cardName = (getAnyCardData(choice.card) as { name?: string })?.name ?? 'a card';
      // Two log lines joined by a newline — addLog splits on '\n' so the human
      // log shows "moved" and "destroyed" as separate entries, even though the
      // engine treats this as a single atomic choice.
      return `${name} moved ${cardName} from workshop to draft pool\n${name} destroyed ${cardName} from draft pool`;
    }
    case 'selectSellCard': {
      return `${name} sold to a ${getSellCardData(choice.sellCard).ducats}-ducat sell card`;
    }
    case 'gainSecondary':
      return `${name} gained ${choice.color}`;
    case 'gainPrimary':
      return `${name} gained ${choice.color}`;
    case 'mixAll': {
      if (choice.mixes.length === 0) return `${name} skipped remaining mixes`;
      const parts = choice.mixes.map(([a, b]) => `mixed ${a} + ${b} to make ${mixResult(a, b)}`);
      return `${name} ${parts.join(', ')}`;
    }
    case 'swapTertiary':
      return `${name} swapped ${choice.loseColor} for ${choice.gainColor}`;
    case 'destroyAndMix': {
      const cardName = (getAnyCardData(choice.card) as { name?: string })?.name ?? 'a card';
      let msg = `${name} destroyed ${cardName} from drafted cards`;
      if (choice.mixes.length > 0) {
        const parts = choice.mixes.map(([a, b]) => `mixed ${a} + ${b} to make ${mixResult(a, b)}`);
        msg += `, ${parts.join(', ')}`;
      }
      return msg;
    }
    case 'destroyAndSell': {
      const cardName = (getAnyCardData(choice.card) as { name?: string })?.name ?? 'a card';
      return `${name} destroyed ${cardName} from drafted cards, sold to a ${getSellCardData(choice.sellCard).ducats}-ducat sell card`;
    }
    case 'destroyAndWorkshop': {
      const cardName = (getAnyCardData(choice.card) as { name?: string })?.name ?? 'a card';
      if (choice.workshopCards.length === 0) {
        return `${name} destroyed ${cardName} from drafted cards, skipped workshop`;
      }
      const workshopNames = choice.workshopCards.map(c => (getAnyCardData(c) as { name?: string })?.name ?? 'a card');
      return `${name} destroyed ${cardName} from drafted cards, workshopped ${workshopNames.join(', ')}`;
    }
    case 'destroyAndDestroyCards': {
      const cardName = (getAnyCardData(choice.card) as { name?: string })?.name ?? 'a card';
      if (choice.target === null) {
        return `${name} destroyed ${cardName} from drafted cards\n${name} did not move a card to draft pool`;
      }
      const targetName = (getAnyCardData(choice.target) as { name?: string })?.name ?? 'a card';
      return `${name} destroyed ${cardName} from drafted cards\n${name} moved ${targetName} from workshop to draft pool\n${name} destroyed ${targetName} from draft pool`;
    }
    case 'selectMoveToDrafted': {
      const cardName = (getAnyCardData(choice.card) as { name?: string })?.name ?? 'a card';
      return `${name} moved ${cardName} from workshop to drafted`;
    }
    case 'skipMoveToDrafted':
      return `${name} skipped moving a card to drafted`;
    case 'selectMoveToWorkshop': {
      const cardName = (getAnyCardData(choice.card) as { name?: string })?.name ?? 'a card';
      return `${name} moved ${cardName} from drafted to workshop`;
    }
    case 'skipMoveToWorkshop':
      return `${name} skipped moving a card to workshop`;
    case 'deferredMoveToDraft': {
      const cardName = (getAnyCardData(choice.card) as { name?: string })?.name ?? 'a card';
      return `${name} moved ${cardName} from workshop to draft pool`;
    }
    case 'destroyWorkshopCardDeferred': {
      const cardName = (getAnyCardData(choice.card) as { name?: string })?.name ?? 'a card';
      return `${name} destroyed ${cardName} from draft pool`;
    }
    default:
      return assertNever(choice);
  }
}

export function determineWinners(players: PlayerState[], playerNames: string[]): string[] {
  const ranked = players.map((p, i) => ({
    name: playerNames[i],
    score: calculateScores([p], [playerNames[i]])[0].score,
    sellCards: p.completedSellCards.length,
    colors: Object.values(p.colorWheel).reduce((sum, c) => sum + (c as number), 0),
  }));
  ranked.sort((a, b) => b.score - a.score || b.sellCards - a.sellCards || b.colors - a.colors);
  const best = ranked[0];
  return ranked
    .filter(r => r.score === best.score && r.sellCards === best.sellCards && r.colors === best.colors)
    .map(r => r.name);
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

export function canSell(state: GameState, sellCardInstanceId: number): boolean {
  if (state.phase.type !== 'action') return false;
  const player = state.players[state.phase.actionState.currentPlayerIndex];
  const sellCardInstance = state.sellCardDisplay.find(g => g.instanceId === sellCardInstanceId);
  if (!sellCardInstance) return false;
  const sellCard = getSellCardData(sellCardInstance.card);
  if (player.materials[sellCard.requiredMaterial] < 1) return false;
  return canPayCost(player.colorWheel, sellCard.colorCost);
}
