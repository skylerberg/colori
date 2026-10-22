import type { BotSettings, GameState, StructuredGameLog } from './data/types';
import { type GameSetup, restoreSetup } from './data/gameSetup';
import { type LogEntry, migrateLog } from './logEntries';

// Bumped when a saved game could no longer be continued: the same game state
// means something else once the rules around drawing the workshop change.
const STORAGE_KEY = 'colori-saved-game-2';
const SETUP_KEY = 'colori-last-setup';

export interface SavedGame {
  gameState: GameState;
  gameStartTime: number;
  gameLog: LogEntry[];
  botIterations?: number[];
  structuredLog?: StructuredGameLog;
  botSettings?: BotSettings;
}

export function saveGame(gameState: GameState, gameStartTime: number, gameLog: LogEntry[], botIterations?: number[], structuredLog?: StructuredGameLog, botSettings?: BotSettings): void {
  try {
    const data: SavedGame = { gameState, gameStartTime, gameLog, botIterations, structuredLog, botSettings };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
  } catch {
    // localStorage full or unavailable — silently ignore
  }
}

export function loadGame(): SavedGame | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === null) return null;
    const data = JSON.parse(raw);
    if (
      !Array.isArray(data.gameState?.players) ||
      !data.gameState?.phase ||
      typeof data.gameState?.round !== 'number' ||
      typeof data.gameStartTime !== 'number' ||
      !Array.isArray(data.gameLog)
    ) {
      clearSavedGame();
      return null;
    }
    data.gameLog = migrateLog(data.gameLog);
    return data as SavedGame;
  } catch {
    clearSavedGame();
    return null;
  }
}

export function clearSavedGame(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    // ignore
  }
}

export function saveSetup(setup: GameSetup): void {
  try {
    localStorage.setItem(SETUP_KEY, JSON.stringify(setup));
  } catch {
    // localStorage full or unavailable — the next game starts from the defaults
  }
}

/** The setup the last local game started from, or the defaults before there was one. */
export function loadSetup(): GameSetup {
  try {
    const raw = localStorage.getItem(SETUP_KEY);
    return restoreSetup(raw === null ? null : JSON.parse(raw));
  } catch {
    return restoreSetup(null);
  }
}
