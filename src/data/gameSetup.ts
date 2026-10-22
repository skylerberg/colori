import { DEFAULT_BOT_ITERATIONS, botDifficultyLevel } from './botDifficulty';

export const PLAYER_COUNTS = [2, 3, 4];
const MAX_PLAYERS = Math.max(...PLAYER_COUNTS);

/** The trained parameter sets the setup screen offers, by their names in colori-wasm. */
export const BOT_PARAMS_CHOICES = [
  { value: 'default', label: 'alobyo gen 45 (default)' },
  { value: 'lki08w-gen-32', label: 'lki08w gen 32' },
  { value: '9iii4n-gen-19', label: '9iii4n gen 19' },
  { value: '9iii4n-gen-19-avoid-alum-warehouse-gondola', label: '9iii4n gen 19, avoiding Alum, Warehouse, Gondola' },
];

// Six rounds past round 1 is the end of the game.
export const END_OF_GAME_LOOKAHEAD = 6;

/** What the local setup screen offers, as the player last started a game from it. */
export interface GameSetup {
  playerCount: number;
  // The seat arrays cover every seat the screen offers, not only those in play,
  // so lowering the player count and raising it again loses nothing.
  isBot: boolean[];
  botIterations: number[];
  /** A name the player typed, or `null` for one generated from the seat. */
  names: (string | null)[];
  params: string;
  rolloutLookahead: number | null;
}

/**
 * A stored setup with anything this version no longer offers, or that is not a
 * setup at all, replaced by its default; `restoreSetup(null)` is the defaults.
 */
export function restoreSetup(stored: unknown): GameSetup {
  const setup: Record<string, unknown> = typeof stored === 'object' && stored !== null ? { ...stored } : {};
  const perSeat = <T>(key: string, restore: (value: unknown, seat: number) => T): T[] => {
    // A setup saved while bots were still called AI keeps its seats.
    const list = setup[key] ?? setup[key === 'isBot' ? 'isAI' : key.replace('botIterations', 'aiIterations')];
    const values: unknown[] = Array.isArray(list) ? list : [];
    return Array.from({ length: MAX_PLAYERS }, (_, seat) => restore(values[seat], seat));
  };
  const lookahead = setup.rolloutLookahead;
  return {
    playerCount: PLAYER_COUNTS.find(count => count === setup.playerCount) ?? PLAYER_COUNTS[0],
    // Seat 1 starts as the human; the rest are bots until told otherwise.
    isBot: perSeat('isBot', (value, seat) => (typeof value === 'boolean' ? value : seat > 0)),
    botIterations: perSeat('botIterations', value =>
      Number.isFinite(value) ? botDifficultyLevel(value as number).iterations : DEFAULT_BOT_ITERATIONS
    ),
    names: perSeat('names', value => (typeof value === 'string' ? value : null)),
    params: BOT_PARAMS_CHOICES.find(choice => choice.value === setup.params)?.value ?? 'default',
    rolloutLookahead:
      Number.isInteger(lookahead) && (lookahead as number) >= 0 && (lookahead as number) <= END_OF_GAME_LOOKAHEAD
        ? (lookahead as number)
        : null,
  };
}
