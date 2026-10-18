import { describe, expect, it } from 'vitest';
import { assignSlots } from './boardLayout';
import { buyerSlotOpensInRound, formatScore } from '../gameUtils';

describe('assignSlots', () => {
  it('fills slots left to right in the order given', () => {
    expect(assignSlots([], [7, 3], 4)).toEqual([7, 3, null, null]);
  });

  it('leaves a gap where a card left, and keeps the rest in place', () => {
    expect(assignSlots([7, 3, 9, null], [7, 9], 4)).toEqual([7, null, 9, null]);
  });

  it('puts a new card in the first free slot', () => {
    expect(assignSlots([7, null, 9, null], [7, 9, 12], 4)).toEqual([7, 12, 9, null]);
  });
});

describe('buyerSlotOpensInRound', () => {
  it('opens one slot to start, a second in round 2 and a third in round 4', () => {
    expect([0, 1, 2].map(buyerSlotOpensInRound)).toEqual([1, 2, 4]);
  });
});

describe('formatScore', () => {
  it('writes half points as ½', () => {
    expect(formatScore(7)).toBe('7');
    expect(formatScore(7.5)).toBe('7½');
    expect(formatScore(0.5)).toBe('½');
  });
});
