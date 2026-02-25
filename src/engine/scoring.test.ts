import { describe, it, expect } from 'vitest';
import type { PlayerState, BuyerCard, CardInstance } from '../data/types';
import { createEmptyWheel } from './colorWheel';
import { calculateScore, calculateScores, determineWinner } from './scoring';

function makePlayerWithBuyers(name: string, stars: number[]): PlayerState {
  const completedBuyers: CardInstance<BuyerCard>[] = stars.map((s, i) => ({
    instanceId: i + 1,
    card: {
      kind: 'buyer' as const,
      name: `Buyer ${i}`,
      stars: s,
      requiredMaterial: 'Ceramics' as const,
      colorCost: [],
    },
  }));

  return {
    name,
    deck: [],
    discard: [],
    workshopCards: [],
    draftedCards: [],
    colorWheel: createEmptyWheel(),
    materials: { Textiles: 0, Ceramics: 0, Paintings: 0 },
    completedBuyers,
    ducats: 0,
  };
}

describe('calculateScore', () => {
  it('sums the stars of completed buyers', () => {
    const player = makePlayerWithBuyers('Alice', [1, 2, 3]);
    expect(calculateScore(player)).toBe(6);
  });

  it('returns 0 when no buyers are completed', () => {
    const player = makePlayerWithBuyers('Alice', []);
    expect(calculateScore(player)).toBe(0);
  });

  it('handles single buyer', () => {
    const player = makePlayerWithBuyers('Alice', [5]);
    expect(calculateScore(player)).toBe(5);
  });
});

describe('calculateScores', () => {
  it('returns name and score for each player', () => {
    const players = [
      makePlayerWithBuyers('Alice', [1, 2]),
      makePlayerWithBuyers('Bob', [3, 4]),
    ];

    const scores = calculateScores(players);
    expect(scores).toEqual([
      { name: 'Alice', score: 3 },
      { name: 'Bob', score: 7 },
    ]);
  });
});

describe('determineWinner', () => {
  it('returns the player with the highest score', () => {
    const players = [
      makePlayerWithBuyers('Alice', [1, 2]),
      makePlayerWithBuyers('Bob', [3, 4, 5]),
      makePlayerWithBuyers('Charlie', [2]),
    ];

    expect(determineWinner(players)).toBe('Bob');
  });

  it('returns first player when tied at highest score', () => {
    const players = [
      makePlayerWithBuyers('Alice', [3, 2]),
      makePlayerWithBuyers('Bob', [5]),
    ];

    // Both have score 5, sort is stable so Alice comes first after sort
    const winner = determineWinner(players);
    // With .sort descending, first encountered 5 stays first
    expect(winner).toBe('Alice');
  });
});
