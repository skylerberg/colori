import { describe, it, expect } from 'vitest';
import { ALL_COLORS } from '../data/colors';
import {
  createEmptyWheel,
  storeColor,
  removeColor,
  performMix,
  canPayCost,
  payCost,
} from './colorWheel';

describe('createEmptyWheel', () => {
  it('has all 12 colors initialized to 0', () => {
    const wheel = createEmptyWheel();
    expect(Object.keys(wheel)).toHaveLength(12);
    for (const color of ALL_COLORS) {
      expect(wheel[color]).toBe(0);
    }
  });
});

describe('storeColor', () => {
  it('increments the count for a color', () => {
    const wheel = createEmptyWheel();
    storeColor(wheel, 'Red');
    expect(wheel['Red']).toBe(1);
    storeColor(wheel, 'Red');
    expect(wheel['Red']).toBe(2);
  });
});

describe('removeColor', () => {
  it('decrements the count and returns true', () => {
    const wheel = createEmptyWheel();
    storeColor(wheel, 'Blue');
    const result = removeColor(wheel, 'Blue');
    expect(result).toBe(true);
    expect(wheel['Blue']).toBe(0);
  });

  it('returns false when count is 0', () => {
    const wheel = createEmptyWheel();
    const result = removeColor(wheel, 'Blue');
    expect(result).toBe(false);
    expect(wheel['Blue']).toBe(0);
  });
});

describe('performMix', () => {
  it('mixes two adjacent-by-2 colors successfully', () => {
    const wheel = createEmptyWheel();
    // Red (index 0) and Orange (index 2) are 2 apart, mix to Vermilion (index 1)
    storeColor(wheel, 'Red');
    storeColor(wheel, 'Orange');
    const result = performMix(wheel, 'Red', 'Orange');
    expect(result).toBe(true);
    expect(wheel['Red']).toBe(0);
    expect(wheel['Orange']).toBe(0);
    expect(wheel['Vermilion']).toBe(1);
  });

  it('fails for non-adjacent colors (distance != 2)', () => {
    const wheel = createEmptyWheel();
    // Red and Yellow are 4 apart
    storeColor(wheel, 'Red');
    storeColor(wheel, 'Yellow');
    const result = performMix(wheel, 'Red', 'Yellow');
    expect(result).toBe(false);
    // Colors should not be consumed
    expect(wheel['Red']).toBe(1);
    expect(wheel['Yellow']).toBe(1);
  });

  it('fails when a required color has 0 count', () => {
    const wheel = createEmptyWheel();
    storeColor(wheel, 'Red');
    // Orange has 0 count
    const result = performMix(wheel, 'Red', 'Orange');
    expect(result).toBe(false);
    expect(wheel['Red']).toBe(1);
  });

  it('handles wrapping around the wheel', () => {
    const wheel = createEmptyWheel();
    // Purple (index 10) and Red (index 0) are distance 2 wrapping, mix to Magenta (index 11)
    storeColor(wheel, 'Purple');
    storeColor(wheel, 'Red');
    const result = performMix(wheel, 'Purple', 'Red');
    expect(result).toBe(true);
    expect(wheel['Purple']).toBe(0);
    expect(wheel['Red']).toBe(0);
    expect(wheel['Magenta']).toBe(1);
  });
});

describe('canPayCost', () => {
  it('returns true when wheel has enough colors', () => {
    const wheel = createEmptyWheel();
    storeColor(wheel, 'Red');
    storeColor(wheel, 'Red');
    storeColor(wheel, 'Blue');
    expect(canPayCost(wheel, ['Red', 'Red', 'Blue'])).toBe(true);
  });

  it('returns false when wheel is short on a color', () => {
    const wheel = createEmptyWheel();
    storeColor(wheel, 'Red');
    expect(canPayCost(wheel, ['Red', 'Red'])).toBe(false);
  });

  it('does not mutate the wheel', () => {
    const wheel = createEmptyWheel();
    storeColor(wheel, 'Red');
    storeColor(wheel, 'Red');
    canPayCost(wheel, ['Red', 'Red']);
    expect(wheel['Red']).toBe(2);
  });
});

describe('payCost', () => {
  it('removes the cost from the wheel and returns true', () => {
    const wheel = createEmptyWheel();
    storeColor(wheel, 'Red');
    storeColor(wheel, 'Blue');
    const result = payCost(wheel, ['Red', 'Blue']);
    expect(result).toBe(true);
    expect(wheel['Red']).toBe(0);
    expect(wheel['Blue']).toBe(0);
  });

  it('returns false and does not mutate when cost cannot be paid', () => {
    const wheel = createEmptyWheel();
    storeColor(wheel, 'Red');
    const result = payCost(wheel, ['Red', 'Red']);
    expect(result).toBe(false);
    expect(wheel['Red']).toBe(1);
  });
});
