import { describe, expect, it } from 'vitest';
import { AI_DIFFICULTY_LEVELS, DEFAULT_AI_ITERATIONS, aiDifficultyLabel, aiPlayerName } from './aiDifficulty';

describe('aiDifficultyLabel', () => {
  it('names each level the AI select offers', () => {
    expect(aiDifficultyLabel(1000)).toBe('Easy');
    expect(aiDifficultyLabel(4000)).toBe('Medium');
    expect(aiDifficultyLabel(25000)).toBe('Hard');
    expect(aiDifficultyLabel(100000)).toBe('Very Hard');
  });

  it('falls back to the closest level for an iteration count it does not offer', () => {
    expect(aiDifficultyLabel(1200)).toBe('Easy');
    expect(aiDifficultyLabel(80000)).toBe('Very Hard');
  });
});

describe('aiPlayerName', () => {
  it('states the difficulty and a one-based seat number', () => {
    expect(aiPlayerName(1, 25000)).toBe('Hard AI 2');
    expect(aiPlayerName(0, 1000)).toBe('Easy AI 1');
  });

  it('defaults to the strongest level', () => {
    expect(aiPlayerName(2)).toBe(`Very Hard AI 3`);
    expect(aiDifficultyLabel(DEFAULT_AI_ITERATIONS)).toBe(AI_DIFFICULTY_LEVELS.at(-1)!.label);
  });
});
