import type { GameState, Choice, PlayerState, Card, Color, DrawEvent, MaterialType, Buyer } from '../data/types';
import { isPrimary } from '../data/colors';
import { getBuyerData } from '../data/cards';
import init, {
  wasm_create_initial_game_state,
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

export function createInitialGameState(playerNames: string[], botPlayers?: boolean[]): GameState {
  const ai = botPlayers ?? playerNames.map(() => false);
  const botJson = JSON.stringify(ai);
  const resultJson = engineCall(
    () => wasm_create_initial_game_state(playerNames.length, botJson),
    { call: 'createInitialGameState', players: playerNames.length, ai: botJson },
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

export function determineWinners(players: PlayerState[], playerNames: string[]): string[] {
  const ranked = players.map((p, i) => ({
    name: playerNames[i],
    score: calculateScores([p], [playerNames[i]])[0].score,
    buyers: p.completedOrders.length,
    colors: Object.values(p.colorWheel).reduce((sum, c) => sum + (c as number), 0),
  }));
  ranked.sort((a, b) => b.score - a.score || b.buyers - a.buyers || b.colors - a.colors);
  const best = ranked[0];
  return ranked
    .filter(r => r.score === best.score && r.buyers === best.buyers && r.colors === best.colors)
    .map(r => r.name);
}

export function canSell(player: PlayerState, buyerInstanceId: number): boolean {
  // Only the player's own buyers are sellable; the shared display is claimed
  // from during the buyers phase and never sold to directly.
  const buyerInstance = player.buyers.find(g => g.instanceId === buyerInstanceId);
  return buyerInstance !== undefined && saleShortfall(player, buyerInstance.card).length === 0;
}

// What a sale to `buyer` still lacks once boats and bowls have
// stood in where they can, one entry per missing item; empty when it can be
// made. Mirrors `action_phase::can_afford_sell_card`: a boat stands in for
// each material the player lacks, on top of any the card asks for outright.
export function saleShortfall(player: PlayerState, buyer: Buyer): string[] {
  const data = getBuyerData(buyer);
  const missing: string[] = [];

  let boats = player.boats;
  for (let i = 0; i < data.requiredBoats; i++) {
    if (boats > 0) boats--;
    else missing.push('Boat');
  }
  const used: Partial<Record<MaterialType, number>> = {};
  for (const material of data.requiredMaterials) {
    const taken = used[material] ?? 0;
    if (player.materialsWheel[material] > taken) used[material] = taken + 1;
    else if (boats > 0) boats--;
    else missing.push(material);
  }

  const spent: Partial<Record<Color, number>> = {};
  let bowls = player.bowls;
  for (const color of data.colorCost) {
    const taken = spent[color] ?? 0;
    if (player.colorWheel[color] > taken) spent[color] = taken + 1;
    else if (isPrimary(color) && bowls > 0) bowls--;
    else missing.push(color);
  }
  return missing;
}
