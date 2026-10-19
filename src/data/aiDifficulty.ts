// How strong an AI seat plays, expressed as the MCTS iteration budget it searches with.
export interface AiDifficultyLevel {
  label: string;
  iterations: number;
}

// Ordered easiest to hardest; players see these labels wherever they pick a difficulty.
export const AI_DIFFICULTY_LEVELS: AiDifficultyLevel[] = [
  { label: 'Easy', iterations: 1000 },
  { label: 'Medium', iterations: 4000 },
  { label: 'Hard', iterations: 25000 },
  { label: 'Very Hard', iterations: 100000 },
];

// The strongest level is what an AI seat plays unless a player picks otherwise.
export const DEFAULT_AI_ITERATIONS = 100000;

/** The level matching an iteration count, or the closest one when it is off-menu (an old save, say). */
export function aiDifficultyLevel(iterations: number): AiDifficultyLevel {
  let closest = AI_DIFFICULTY_LEVELS[0];
  for (const level of AI_DIFFICULTY_LEVELS) {
    if (Math.abs(level.iterations - iterations) < Math.abs(closest.iterations - iterations)) {
      closest = level;
    }
  }
  return closest;
}

export function aiDifficultyLabel(iterations: number): string {
  return aiDifficultyLevel(iterations).label;
}

/** An AI seat's default name, so a game reads the difficulty off the players' names. */
export function aiPlayerName(seat: number, iterations: number = DEFAULT_AI_ITERATIONS): string {
  return `${aiDifficultyLabel(iterations)} AI ${seat + 1}`;
}

export function humanPlayerName(seat: number): string {
  return `Player ${seat + 1}`;
}
