import { describe, it, expect } from 'vitest';
import type { PlayerState, GarmentCard, CardInstance } from '../data/types';
import { createEmptyWheel } from './colorWheel';
import { calculateScore, calculateScores, determineWinner } from './scoring';

function makePlayerWithGarments(name: string, stars: number[]): PlayerState {
  const completedGarments: CardInstance<GarmentCard>[] = stars.map((s, i) => ({
    instanceId: i + 1,
    card: {
      kind: 'garment' as const,
      name: `Garment ${i}`,
      stars: s,
      requiredFabric: 'Wool' as const,
      matchingDyeName: 'Kermes',
      colorCost: [],
    },
  }));

  return {
    name,
    deck: [],
    discard: [],
    drawnCards: [],
    draftedCards: [],
    colorWheel: createEmptyWheel(),
    completedGarments,
  };
}

describe('calculateScore', () => {
  it('sums the stars of completed garments', () => {
    const player = makePlayerWithGarments('Alice', [1, 2, 3]);
    expect(calculateScore(player)).toBe(6);
  });

  it('returns 0 when no garments are completed', () => {
    const player = makePlayerWithGarments('Alice', []);
    expect(calculateScore(player)).toBe(0);
  });

  it('handles single garment', () => {
    const player = makePlayerWithGarments('Alice', [5]);
    expect(calculateScore(player)).toBe(5);
  });
});

describe('calculateScores', () => {
  it('returns name and score for each player', () => {
    const players = [
      makePlayerWithGarments('Alice', [1, 2]),
      makePlayerWithGarments('Bob', [3, 4]),
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
      makePlayerWithGarments('Alice', [1, 2]),
      makePlayerWithGarments('Bob', [3, 4, 5]),
      makePlayerWithGarments('Charlie', [2]),
    ];

    expect(determineWinner(players)).toBe('Bob');
  });

  it('returns first player when tied at highest score', () => {
    const players = [
      makePlayerWithGarments('Alice', [3, 2]),
      makePlayerWithGarments('Bob', [5]),
    ];

    // Both have score 5, sort is stable so Alice comes first after sort
    const winner = determineWinner(players);
    // With .sort descending, first encountered 5 stays first
    expect(winner).toBe('Alice');
  });
});
