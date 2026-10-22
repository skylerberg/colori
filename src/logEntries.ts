import type {
  CardInstance, Choice, Color, GoldCoinPurchase, GameState, MaterialType, PlayerState, Buyer,
} from './data/types';
import { ALL_COLORS } from './data/colors';
import { getBuyerData } from './data/cards';

// The game log shown to players. Each entry is one move by one player, with
// what it changed for them, so a view can be filtered to a single player and
// draw the move with the game's own cards, colors and icons.

export interface LogChange {
  colors: Partial<Record<Color, number>>;
  materials: Partial<Record<MaterialType, number>>;
  goldCoins: number;
  boats: number;
  bowls: number;
  points: number;
}

export type LogMove =
  | { type: 'claim'; buyer: Buyer; unseen: boolean }
  | { type: 'picked'; cards: CardInstance[] }
  | { type: 'topAction'; card: CardInstance }
  // Cards a turn trashed on its way out, without using their top actions.
  | { type: 'trashWithoutUsing'; cards: CardInstance[] }
  | { type: 'keep'; card: CardInstance }
  | { type: 'workshop'; cards: CardInstance[] }
  | { type: 'mix'; mixes: [Color, Color][] }
  | { type: 'sell'; buyer: Buyer }
  | { type: 'lift'; card: CardInstance | null }
  | { type: 'secondary'; color: Color }
  | { type: 'spendGoldCoin'; purchase: GoldCoinPurchase }
  | { type: 'endTurn' };

export type LogEntry =
  | { kind: 'round'; round: number }
  | { kind: 'play'; round: number; player: number; moves: LogMove[]; change: LogChange }
  | { kind: 'note'; round: number; player: number | null; text: string };

const MATERIALS: MaterialType[] = ['Textiles', 'Ceramics', 'Paintings'];

export function roundEntry(round: number): LogEntry {
  return { kind: 'round', round };
}

export function noteEntry(round: number, text: string, player: number | null = null): LogEntry {
  return { kind: 'note', round, player, text };
}

// A copy of the parts of a player a log entry compares, taken before a move.
export function snapshot(player: PlayerState): PlayerState {
  return JSON.parse(JSON.stringify(player));
}

export function logChange(before: PlayerState, after: PlayerState): LogChange {
  const colors: LogChange['colors'] = {};
  for (const color of ALL_COLORS) {
    const delta = after.colorWheel[color] - before.colorWheel[color];
    if (delta !== 0) colors[color] = delta;
  }
  const materials: LogChange['materials'] = {};
  for (const material of MATERIALS) {
    const delta = after.materialsWheel[material] - before.materialsWheel[material];
    if (delta !== 0) materials[material] = delta;
  }
  const sold = (p: PlayerState) => p.completedOrders.reduce((sum, s) => sum + getBuyerData(s.card).points, 0);
  return {
    colors,
    materials,
    goldCoins: after.goldCoins - before.goldCoins,
    boats: after.boats - before.boats,
    bowls: after.bowls - before.bowls,
    points: sold(after) - sold(before),
  };
}

export function isEmptyChange(change: LogChange): boolean {
  return Object.keys(change.colors).length === 0 && Object.keys(change.materials).length === 0
    && change.goldCoins === 0 && change.boats === 0 && change.bowls === 0 && change.points === 0;
}

// The instances of `cards` among `from`, each used once, so a starter card
// keeps its own portrait.
function instancesOf(cards: string[], from: CardInstance[]): CardInstance[] {
  const pool = [...from];
  return cards.map(card => {
    const i = pool.findIndex(ci => ci.card === card);
    return i >= 0 ? pool.splice(i, 1)[0] : ({ instanceId: -1, card } as CardInstance);
  });
}

function movesFor(choice: Choice, before: PlayerState, after: PlayerState): LogMove[] {
  const used = (card: string): LogMove => ({ type: 'topAction', card: instancesOf([card], before.topCards)[0] });
  const workshopped = (cards: string[]): LogMove => ({ type: 'workshop', cards: instancesOf(cards, before.workshopCards) });
  const moved = (card: string | null): LogMove => ({
    type: 'lift',
    card: card === null ? null : instancesOf([card], before.workshopCards)[0],
  });
  switch (choice.type) {
    case 'draftPick':
      return [];
    case 'claimBuyer':
      return [{ type: 'claim', buyer: choice.buyer, unseen: false }];
    case 'drawBuyer': {
      const had = new Set(before.buyers.map(b => b.instanceId));
      const drawn = after.buyers.find(b => !had.has(b.instanceId));
      return drawn ? [{ type: 'claim', buyer: drawn.card, unseen: true }] : [];
    }
    case 'trashTopCard':
      return [used(choice.card)];
    case 'keepCard':
      return [{ type: 'keep', card: instancesOf([choice.card], before.topCards)[0] }];
    case 'trashAndWorkshop':
      return [used(choice.card), workshopped(choice.workshopCards)];
    case 'trashAndMix':
      return [used(choice.card), { type: 'mix', mixes: choice.mixes }];
    case 'trashAndSell':
      return [used(choice.card), { type: 'sell', buyer: choice.buyer }];
    case 'trashAndLift':
      return [used(choice.card), moved(choice.target)];
    case 'workshop':
      return [workshopped(choice.cardTypes)];
    case 'skipWorkshop':
      return [workshopped([])];
    case 'mixAll':
      return [{ type: 'mix', mixes: choice.mixes }];
    case 'sellToBuyer':
      return [{ type: 'sell', buyer: choice.buyer }];
    case 'liftCard':
      return [moved(choice.card)];
    case 'gainSecondary':
      return [{ type: 'secondary', color: choice.color }];
    case 'spendGoldCoin':
      return [{ type: 'spendGoldCoin', purchase: choice.purchase }];
    case 'endTurn':
      return choice.keepRemaining === false && before.topCards.length > 0
        ? [{ type: 'trashWithoutUsing', cards: before.topCards }, { type: 'endTurn' }]
        : [{ type: 'endTurn' }];
  }
}

// The entry for `choice`, given the player as it was before and is after; null
// for a move the log does not show, such as a draft pick still face down.
export function playEntry(round: number, player: number, choice: Choice, before: PlayerState, after: PlayerState): LogEntry | null {
  const moves = movesFor(choice, before, after);
  if (moves.length === 0) return null;
  return { kind: 'play', round, player, moves, change: logChange(before, after) };
}

// Everyone's draft, once it is over and the cards are face up.
export function topCardEntries(state: GameState, draftOrder?: number[][]): LogEntry[] {
  return state.players.map((player, i) => {
    const order = draftOrder?.[i] ?? [];
    const cards = [...player.topCards].sort((a, b) => rank(order, a) - rank(order, b));
    const none = logChange(player, player);
    return { kind: 'play', round: state.round, player: i, moves: [{ type: 'picked', cards }], change: none };
  });
}

function rank(order: number[], card: CardInstance): number {
  const i = order.indexOf(card.instanceId);
  return i < 0 ? Number.MAX_SAFE_INTEGER : i;
}

// Saved games from before entries were structured hold plain lines.
export function migrateLog(log: unknown[]): LogEntry[] {
  return log.map(entry => (typeof entry === 'string' ? noteEntry(0, entry) : (entry as LogEntry)));
}
