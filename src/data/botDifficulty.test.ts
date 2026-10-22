import { describe, expect, it } from 'vitest';
import { BOT_DIFFICULTY_LEVELS, DEFAULT_BOT_ITERATIONS, botDifficultyLabel, botPlayerName } from './botDifficulty';

describe('botDifficultyLabel', () => {
  it('names each level the bot select offers', () => {
    expect(botDifficultyLabel(1000)).toBe('Easy');
    expect(botDifficultyLabel(4000)).toBe('Medium');
    expect(botDifficultyLabel(25000)).toBe('Hard');
    expect(botDifficultyLabel(100000)).toBe('Very Hard');
  });

  it('falls back to the closest level for an iteration count it does not offer', () => {
    expect(botDifficultyLabel(1200)).toBe('Easy');
    expect(botDifficultyLabel(80000)).toBe('Very Hard');
  });
});

describe('botPlayerName', () => {
  it('states the difficulty and a one-based seat number', () => {
    expect(botPlayerName(1, 25000)).toBe("Hard Bot 2");
    expect(botPlayerName(0, 1000)).toBe("Easy Bot 1");
  });

  it('defaults to the strongest level', () => {
    expect(botPlayerName(2)).toBe(`Very Hard Bot 3`);
    expect(botDifficultyLabel(DEFAULT_BOT_ITERATIONS)).toBe(BOT_DIFFICULTY_LEVELS.at(-1)!.label);
  });
});
