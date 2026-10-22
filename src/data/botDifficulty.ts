// How strong a bot seat plays, expressed as the MCTS iteration budget it searches with.
export interface BotDifficultyLevel {
  label: string;
  iterations: number;
}

// Ordered easiest to hardest; players see these labels wherever they pick a difficulty.
export const BOT_DIFFICULTY_LEVELS: BotDifficultyLevel[] = [
  { label: 'Easy', iterations: 1000 },
  { label: 'Medium', iterations: 4000 },
  { label: 'Hard', iterations: 25000 },
  { label: 'Very Hard', iterations: 100000 },
];

// The strongest level is what a bot seat plays unless a player picks otherwise.
export const DEFAULT_BOT_ITERATIONS = 100000;

/** The level matching an iteration count, or the closest one when it is off-menu (an old save, say). */
export function botDifficultyLevel(iterations: number): BotDifficultyLevel {
  let closest = BOT_DIFFICULTY_LEVELS[0];
  for (const level of BOT_DIFFICULTY_LEVELS) {
    if (Math.abs(level.iterations - iterations) < Math.abs(closest.iterations - iterations)) {
      closest = level;
    }
  }
  return closest;
}

export function botDifficultyLabel(iterations: number): string {
  return botDifficultyLevel(iterations).label;
}

/** A bot seat's default name, so a game reads the difficulty off the players' names. */
export function botPlayerName(seat: number, iterations: number = DEFAULT_BOT_ITERATIONS): string {
  return `${botDifficultyLabel(iterations)} Bot ${seat + 1}`;
}

export function humanPlayerName(seat: number): string {
  return `Player ${seat + 1}`;
}
