import { describe, expect, it } from 'vitest';
import type { GamePhase } from './data/types';
import { revealedDraftedCount, startingPlayerIndex } from './gameUtils';

function draft(pickNumber: number): GamePhase {
  return { type: 'draft', draftState: { pickNumber, currentPlayerIndex: 0, hands: [[], [], []] } };
}

const VIEWER = 0;

describe('startingPlayerIndex', () => {
  it('starts with the first seat and moves one each round', () => {
    expect(startingPlayerIndex(1, 4)).toBe(0);
    expect(startingPlayerIndex(2, 4)).toBe(1);
    expect(startingPlayerIndex(4, 4)).toBe(3);
  });

  it('wraps after a full turn round the table', () => {
    expect(startingPlayerIndex(5, 4)).toBe(0);
    expect(startingPlayerIndex(7, 3)).toBe(0);
    expect(startingPlayerIndex(7, 5)).toBe(1);
  });
});

describe('revealedDraftedCount', () => {
  it('shows every pick of your own as you make it', () => {
    expect(revealedDraftedCount(draft(0), VIEWER, VIEWER)).toBe(Infinity);
    expect(revealedDraftedCount(draft(2), VIEWER, VIEWER)).toBe(Infinity);
  });

  it('keeps the pick being made hidden until everyone has picked', () => {
    // In the first pick round nobody has finished a round, so nothing shows.
    expect(revealedDraftedCount(draft(0), 1, VIEWER)).toBe(0);
  });

  it('shows the picks of rounds everyone has finished', () => {
    // While the second pick is being made, the first is public.
    expect(revealedDraftedCount(draft(1), 1, VIEWER)).toBe(1);
    // And the third pick round shows the first two.
    expect(revealedDraftedCount(draft(2), 1, VIEWER)).toBe(2);
  });

  it('hides nothing outside the draft', () => {
    expect(revealedDraftedCount({ type: 'draw' }, 1, VIEWER)).toBe(Infinity);
    expect(revealedDraftedCount({ type: 'gameOver' }, 1, VIEWER)).toBe(Infinity);
  });
});
