import type { Ability, AiSettings, GameState, PlayerTurn, StructuredGameLog } from './data/types';
import { type LogEntry, migrateLog } from './logEntries';

const STORAGE_KEY = 'colori-saved-game';

export interface SavedGame {
  gameState: GameState;
  gameStartTime: number;
  gameLog: LogEntry[];
  aiIterations?: number[];
  structuredLog?: StructuredGameLog;
  aiSettings?: AiSettings;
}

export function saveGame(gameState: GameState, gameStartTime: number, gameLog: LogEntry[], aiIterations?: number[], structuredLog?: StructuredGameLog, aiSettings?: AiSettings): void {
  try {
    const data: SavedGame = { gameState, gameStartTime, gameLog, aiIterations, structuredLog, aiSettings };
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
    migrateActionPhase(data.gameState);
    migrateTrash(data.gameState);
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

// A game saved when the action phase still went round the table one turn at a
// time: the seats before the current one, from the round's starting player,
// had ended their turns.
function migrateActionPhase(state: GameState) {
  const phase = state.phase as {
    type: string;
    actionState?: { currentPlayerIndex?: number; abilityStack?: Ability[]; turns?: PlayerTurn[] };
  };
  if (phase.type !== 'action' || !phase.actionState || phase.actionState.turns) return;
  const { currentPlayerIndex = 0, abilityStack = [] } = phase.actionState;
  const count = state.players.length;
  const seatOrder = (i: number) => (i - (state.round - 1) + count * state.round) % count;
  phase.actionState = {
    turns: state.players.map((_, i) => ({
      abilityStack: i === currentPlayerIndex ? abilityStack : [],
      ended: seatOrder(i) < seatOrder(currentPlayerIndex),
    })),
  };
}

// A game saved before the destroyed pile was called the trash.
function migrateTrash(state: GameState) {
  const old = state as GameState & { destroyedPile?: GameState['trash'] };
  if (old.trash === undefined && old.destroyedPile !== undefined) {
    old.trash = old.destroyedPile;
    delete old.destroyedPile;
  }
}
