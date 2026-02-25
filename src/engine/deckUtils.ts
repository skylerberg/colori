import type { AnyCard, CardInstance, PlayerState } from '../data/types';

let nextInstanceId = 1;

export function resetInstanceIdCounter(): void {
  nextInstanceId = 1;
}

export function getNextInstanceId(): number {
  return nextInstanceId++;
}

export function shuffle<T>(array: T[]): T[] {
  const result = [...array];
  for (let i = result.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [result[i], result[j]] = [result[j], result[i]];
  }
  return result;
}

export function shuffleInPlace<T>(array: T[]): void {
  for (let i = array.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    const tmp = array[i]; array[i] = array[j]; array[j] = tmp;
  }
}

export function createCardInstances<T extends AnyCard>(cards: T[]): CardInstance<T>[] {
  return cards.map(card => ({ instanceId: getNextInstanceId(), card }));
}

/**
 * Draw N cards from player's deck. If deck runs out, shuffle discard into deck
 * and continue drawing. Returns the drawn cards. Mutates player's deck and
 * discard in place.
 */
export function drawFromDeck(player: PlayerState, count: number): CardInstance[] {
  const drawn: CardInstance[] = [];
  for (let i = 0; i < count; i++) {
    if (player.deck.length === 0) {
      if (player.discard.length === 0) break;
      shuffleInPlace(player.discard);
      player.deck = player.discard;
      player.discard = [];
    }
    drawn.push(player.deck.pop()!);
  }
  return drawn;
}
