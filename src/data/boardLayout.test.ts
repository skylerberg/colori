import { describe, expect, it } from 'vitest';
import { assignSlots } from './boardLayout';
import { buyerCapacity, formatScore, roundNews } from '../gameUtils';

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

describe('the round track', () => {
  it('opens a buyer slot in rounds 2 and 3', () => {
    expect([1, 2, 3, 4, 7].map(buyerCapacity)).toEqual([1, 2, 3, 3, 3]);
  });

  it('says what each round brings', () => {
    expect(roundNews(1)).toEqual([]);
    expect(roundNews(3)).toEqual(['Buyer slot 3 opens']);
    expect(roundNews(5)).toEqual(['Everyone gains a boat']);
    expect(roundNews(7)).toEqual(['Everyone gains a goldCoin, a boat and a wild primary']);
  });
});

describe('formatScore', () => {
  it('writes half points as ½', () => {
    expect(formatScore(7)).toBe('7');
    expect(formatScore(7.5)).toBe('7½');
    expect(formatScore(0.5)).toBe('½');
  });
});
