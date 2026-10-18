import type { GameState, CardInstance, PlayerState } from './data/types';
import { getSellCardData } from './data/cards';

// Players with a decision to make: one at a time in the buyers phase and the
// draft, and everyone still taking their turn in the action phase.
export function actingPlayers(gs: GameState): number[] {
  switch (gs.phase.type) {
    case 'buyers':
      return [gs.phase.buyersState.currentPlayerIndex];
    case 'draft':
      return [gs.phase.draftState.currentPlayerIndex];
    case 'action':
      return gs.phase.actionState.turns.flatMap((turn, i) => (turn.ended ? [] : [i]));
    default:
      return [];
  }
}

export function isTakingActionTurn(gs: GameState, playerIndex: number): boolean {
  return gs.phase.type === 'action' && !gs.phase.actionState.turns[playerIndex].ended;
}

// Mirrors `colori_core::scoring::score_in_half_points`: sell cards and ducats
// at face value, plus half a point for each unspent wild primary and boat.
export function playerScore(player: Pick<PlayerState, 'completedSellCards' | 'ducats' | 'wildPrimaries' | 'boats'>): number {
  const sold = player.completedSellCards.reduce((sum, sellCard) => sum + getSellCardData(sellCard.card).ducats, 0);
  return sold + player.ducats + (player.wildPrimaries + player.boats) / 2;
}

// Mirrors `colori_core::buyers_phase::buyer_capacity`.
export function buyerCapacity(round: number): number {
  if (round <= 1) return 1;
  if (round <= 3) return 2;
  return 3;
}

// The first round in which buyer slot `slot` (0-based) is open.
export function buyerSlotOpensInRound(slot: number): number {
  let round = 1;
  while (buyerCapacity(round) <= slot) round++;
  return round;
}

// Scores run in half points: 7.5 reads as 7½.
export function formatScore(score: number): string {
  const whole = Math.floor(score);
  return score === whole ? String(whole) : `${whole > 0 ? whole : ''}½`;
}

export function orderByDraftOrder(cards: CardInstance[], draftOrder: number[]): CardInstance[] {
  if (draftOrder.length === 0) return cards;
  const byId = new Map(cards.map(c => [c.instanceId, c]));
  const ordered = draftOrder.filter(id => byId.has(id)).map(id => byId.get(id)!);
  const orderedIds = new Set(draftOrder);
  const remaining = cards.filter(c => !orderedIds.has(c.instanceId));
  return [...ordered, ...remaining];
}

export function formatTime(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) {
    return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  }
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}
