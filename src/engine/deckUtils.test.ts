import { describe, it, expect, beforeEach } from 'vitest';
import type { PlayerState, CardInstance, BasicDyeCard } from '../data/types';
import { BASIC_DYE_CARDS } from '../data/cards';
import {
  shuffle,
  createCardInstances,
  drawFromDeck,
  resetInstanceIdCounter,
} from './deckUtils';
import { createEmptyWheel } from './colorWheel';

function makePlayer(deckSize: number, discardSize: number = 0): PlayerState {
  const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
  const deckCards = Array(deckSize).fill(basicRed);
  const discardCards = Array(discardSize).fill(basicRed);

  return {
    name: 'Test Player',
    deck: createCardInstances(deckCards),
    discard: createCardInstances(discardCards),
    drawnCards: [],
    draftedCards: [],
    colorWheel: createEmptyWheel(),
    materials: { Textiles: 0, Ceramics: 0, Paintings: 0 },
    completedBuyers: [],
  };
}

describe('shuffle', () => {
  it('returns an array with the same length and elements', () => {
    const input = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    const result = shuffle(input);
    expect(result).toHaveLength(input.length);
    expect(result.sort()).toEqual(input.sort());
  });

  it('does not mutate the original array', () => {
    const input = [1, 2, 3, 4, 5];
    const original = [...input];
    shuffle(input);
    expect(input).toEqual(original);
  });
});

describe('createCardInstances', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('assigns unique instance IDs to each card', () => {
    const cards = [BASIC_DYE_CARDS[0], BASIC_DYE_CARDS[1], BASIC_DYE_CARDS[2]];
    const instances = createCardInstances(cards);

    expect(instances).toHaveLength(3);
    const ids = instances.map(i => i.instanceId);
    const uniqueIds = new Set(ids);
    expect(uniqueIds.size).toBe(3);
  });

  it('preserves the card data on each instance', () => {
    const card = BASIC_DYE_CARDS[0];
    const instances = createCardInstances([card]);
    expect(instances[0].card).toBe(card);
  });
});

describe('drawFromDeck', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('draws the correct number of cards', () => {
    const player = makePlayer(10);
    const drawn = drawFromDeck(player, 5);
    expect(drawn).toHaveLength(5);
    expect(player.deck).toHaveLength(5);
  });

  it('draws fewer cards if deck and discard combined are insufficient', () => {
    const player = makePlayer(2, 0);
    const drawn = drawFromDeck(player, 5);
    expect(drawn).toHaveLength(2);
    expect(player.deck).toHaveLength(0);
  });

  it('reshuffles discard when deck is empty', () => {
    const player = makePlayer(0, 5);
    expect(player.deck).toHaveLength(0);
    expect(player.discard).toHaveLength(5);

    const drawn = drawFromDeck(player, 3);
    expect(drawn).toHaveLength(3);
    // Discard should be empty now (all moved to deck)
    expect(player.discard).toHaveLength(0);
    // Remaining deck should have the undrawn cards
    expect(player.deck).toHaveLength(2);
  });

  it('reshuffles discard mid-draw when deck runs out', () => {
    const player = makePlayer(2, 5);
    const drawn = drawFromDeck(player, 5);
    expect(drawn).toHaveLength(5);
    expect(player.discard).toHaveLength(0);
    expect(player.deck).toHaveLength(2);
  });
});
