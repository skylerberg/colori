import type { GameState, CardInstance, GamePhase, PlayerState } from './data/types';
import { getBuyerData } from './data/cards';

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

// Mirrors `colori_core::scoring::score_in_half_points`: the points on sold
// cards and a point a goldCoin, plus half a point for each unspent wild primary
// and boat.
export function playerScore(player: Pick<PlayerState, 'completedBuyers' | 'goldCoins' | 'bowls' | 'boats'>): number {
  const sold = player.completedBuyers.reduce((sum, buyer) => sum + getBuyerData(buyer.card).points, 0);
  return sold + player.goldCoins + (player.bowls + player.boats) / 2;
}

// Mirrors `colori_core::buyers_phase::buyer_capacity`.
export function buyerCapacity(round: number): number {
  if (round <= 1) return 1;
  if (round === 2) return 2;
  return 3;
}

// Mirrors `colori_core::draw_phase::round_income`: what everyone collects at
// the start of `round`, as the round track shows.
export function roundIncome(round: number): { goldCoins: number; boats: number; bowls: number } {
  switch (round) {
    case 4:
    case 6:
      return { goldCoins: 1, boats: 0, bowls: 0 };
    case 5:
      return { goldCoins: 0, boats: 1, bowls: 0 };
    case 7:
      return { goldCoins: 1, boats: 1, bowls: 1 };
    default:
      return { goldCoins: 0, boats: 0, bowls: 0 };
  }
}

// What the start of `round` brings everyone, in words.
export function roundNews(round: number): string[] {
  const news: string[] = [];
  const income = roundIncome(round);
  const gains = [
    income.goldCoins > 0 && 'a goldCoin',
    income.boats > 0 && 'a boat',
    income.bowls > 0 && 'a wild primary',
  ].filter((gain): gain is string => !!gain);
  if (gains.length > 0) {
    news.push(`Everyone gains ${gains.length > 1 ? `${gains.slice(0, -1).join(', ')} and ${gains.at(-1)}` : gains[0]}`);
  }
  if (round > 1 && buyerCapacity(round) > buyerCapacity(round - 1)) {
    news.push(`Buyer slot ${buyerCapacity(round)} opens`);
  }
  return news;
}

// Scores run in half points: 7.5 reads as 7½.
export function formatScore(score: number): string {
  const whole = Math.floor(score);
  return score === whole ? String(whole) : `${whole > 0 ? whole : ''}½`;
}

// Picks stay face down until every seat has picked for that round, so another
// seat's board shows only the picks from rounds already done; the one it is
// picking right now stays hidden. Your own seat shows each pick as you make it,
// which is what the count of `Infinity` means.
export function revealedDraftedCount(phase: GamePhase, seat: number, viewerIndex: number): number {
  if (phase.type !== 'draft' || seat === viewerIndex) return Infinity;
  // One pick per seat per round, and `pickNumber` counts the rounds that have
  // finished, so it is also how many of that seat's picks are face up.
  return phase.draftState.pickNumber;
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
