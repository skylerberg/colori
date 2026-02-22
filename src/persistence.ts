import type { GameState } from './data/types';

const STORAGE_KEY = 'colori-saved-game';

export interface SavedGame {
  gameState: GameState;
  gameStartTime: number;
  gameLog: string[];
}

export function saveGame(gameState: GameState, gameStartTime: number, gameLog: string[]): void {
  try {
    const data: SavedGame = { gameState, gameStartTime, gameLog };
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
