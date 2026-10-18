import type { GameState, Choice, PlayerState, Card, Color, DrawEvent, MaterialType, SellCard } from '../data/types';
import { isPrimary, mixResult } from '../data/colors';
import { getSellCardData, getAnyCardData } from '../data/cards';
import init, {
  wasm_create_initial_game_state,
  wasm_execute_draw_phase,
  wasm_apply_choice,
  wasm_simultaneous_pick,
  wasm_advance_draft,
  wasm_calculate_scores,
} from '../wasm-pkg/colori_wasm.js';
import { engineCall } from './engineCall';

let initialized = false;

export async function initEngine(): Promise<void> {
  if (initialized) return;
  await init();
  initialized = true;
}

export function createInitialGameState(playerNames: string[], aiPlayers?: boolean[]): GameState {
  const ai = aiPlayers ?? playerNames.map(() => false);
  const aiJson = JSON.stringify(ai);
  const resultJson = engineCall(
    () => wasm_create_initial_game_state(playerNames.length, aiJson),
    { call: 'createInitialGameState', players: playerNames.length, ai: aiJson },
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
  const stateJson = JSON.stringify(state);
  const resultJson = engineCall(
    () => wasm_execute_draw_phase(stateJson),
    { call: 'executeDrawPhase', state: stateJson },
  );
  const result: { state: GameState; draws: DrawEvent[] } = JSON.parse(resultJson);
  Object.assign(state, result.state);
  return result.draws;
}

export function applyChoice(state: GameState, playerIndex: number, choice: Choice): DrawEvent[] {
  const stateJson = JSON.stringify(state);
  const choiceJson = JSON.stringify(choice);
  const resultJson = engineCall(
    () => wasm_apply_choice(stateJson, playerIndex, choiceJson),
    { call: 'applyChoice', playerIndex, choice: choiceJson, state: stateJson },
  );
  const result: { state: GameState; draws: DrawEvent[] } = JSON.parse(resultJson);
  Object.assign(state, result.state);
  return result.draws;
}

export function simultaneousPick(state: GameState, playerIndex: number, card: Card): void {
  const stateJson = JSON.stringify(state);
  const cardJson = JSON.stringify(card);
  const resultJson = engineCall(
    () => wasm_simultaneous_pick(stateJson, playerIndex, cardJson),
    { call: 'simultaneousPick', playerIndex, card: cardJson, state: stateJson },
  );
  const newState: GameState = JSON.parse(resultJson);
  Object.assign(state, newState);
}

export function advanceDraft(state: GameState): void {
  const stateJson = JSON.stringify(state);
  const resultJson = engineCall(
    () => wasm_advance_draft(stateJson),
    { call: 'advanceDraft', state: stateJson },
  );
  const newState: GameState = JSON.parse(resultJson);
  Object.assign(state, newState);
}

export function calculateScores(players: PlayerState[], playerNames: string[]): { name: string; score: number }[] {
  const playersJson = JSON.stringify(players);
  const resultJson = engineCall(
    () => wasm_calculate_scores(playersJson),
    { call: 'calculateScores', players: playersJson },
  );
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
    case 'moveToDraftPool': {
      if (choice.card === null) return `${name} had no card to move to draft pool`;
      const cardName = (getAnyCardData(choice.card) as { name?: string })?.name ?? 'a card';
      return `${name} moved ${cardName} from workshop to draft pool`;
    }
    case 'selectSellCard': {
      return `${name} sold to a ${getSellCardData(choice.sellCard).name} for ${getSellCardData(choice.sellCard).ducats} ducats`;
    }
    case 'spendDucat': {
      const bought = { workshop: 'a workshop pick', mixColors: 'a mix', sell: 'a sale' }[choice.purchase];
      return `${name} spent a ducat for ${bought}`;
    }
    case 'takeBuyer': {
      return `${name} claimed a ${getSellCardData(choice.sellCard).name} (${getSellCardData(choice.sellCard).ducats} ducats)`;
    }
    case 'drawBuyer':
      return `${name} claimed a buyer off the top of the deck`;
    case 'gainSecondary':
      return `${name} gained ${choice.color}`;
    case 'mixAll': {
      if (choice.mixes.length === 0) return `${name} skipped remaining mixes`;
      const parts = choice.mixes.map(([a, b]) => `mixed ${a} + ${b} to make ${mixResult(a, b)}`);
      return `${name} ${parts.join(', ')}`;
    }
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
      return `${name} destroyed ${cardName} from drafted cards, sold to a ${getSellCardData(choice.sellCard).name} for ${getSellCardData(choice.sellCard).ducats} ducats`;
    }
    case 'destroyAndWorkshop': {
      const cardName = (getAnyCardData(choice.card) as { name?: string })?.name ?? 'a card';
      if (choice.workshopCards.length === 0) {
        return `${name} destroyed ${cardName} from drafted cards, skipped workshop`;
      }
      const workshopNames = choice.workshopCards.map(c => (getAnyCardData(c) as { name?: string })?.name ?? 'a card');
      return `${name} destroyed ${cardName} from drafted cards, workshopped ${workshopNames.join(', ')}`;
    }
    case 'destroyAndMoveToDraftPool': {
      const cardName = (getAnyCardData(choice.card) as { name?: string })?.name ?? 'a card';
      // Two log lines joined by a newline — addLog splits on '\n' so the human
      // log shows the destroy and the move as separate entries, even though
      // the engine fuses them into one choice.
      if (choice.target === null) {
        return `${name} destroyed ${cardName} from drafted cards\n${name} had no card to move to draft pool`;
      }
      const targetName = (getAnyCardData(choice.target) as { name?: string })?.name ?? 'a card';
      return `${name} destroyed ${cardName} from drafted cards\n${name} moved ${targetName} from workshop to draft pool`;
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

export function canSell(player: PlayerState, sellCardInstanceId: number): boolean {
  // Only the player's own buyers are sellable; the shared display is claimed
  // from during the buyers phase and never sold to directly.
  const sellCardInstance = player.buyers.find(g => g.instanceId === sellCardInstanceId);
  return sellCardInstance !== undefined && saleShortfall(player, sellCardInstance.card).length === 0;
}

// What a sale to `sellCard` still lacks once boats and wild primaries have
// stood in where they can, one entry per missing item; empty when it can be
// made. Mirrors `action_phase::can_afford_sell_card`: a boat stands in for
// each material the player lacks, on top of any the card asks for outright.
export function saleShortfall(player: PlayerState, sellCard: SellCard): string[] {
  const data = getSellCardData(sellCard);
  const missing: string[] = [];

  let boats = player.boats;
  for (let i = 0; i < data.requiredBoats; i++) {
    if (boats > 0) boats--;
    else missing.push('Boat');
  }
  const used: Partial<Record<MaterialType, number>> = {};
  for (const material of data.requiredMaterials) {
    const taken = used[material] ?? 0;
    if (player.materials[material] > taken) used[material] = taken + 1;
    else if (boats > 0) boats--;
    else missing.push(material);
  }

  const spent: Partial<Record<Color, number>> = {};
  let wildPrimaries = player.wildPrimaries;
  for (const color of data.colorCost) {
    const taken = spent[color] ?? 0;
    if (player.colorWheel[color] > taken) spent[color] = taken + 1;
    else if (isPrimary(color) && wildPrimaries > 0) wildPrimaries--;
    else missing.push(color);
  }
  return missing;
}
