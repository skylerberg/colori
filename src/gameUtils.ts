import type { GameState } from './data/types';

export function getActivePlayerIndex(gs: GameState): number {
  if (gs.phase.type === 'draft') {
    return gs.phase.draftState.currentPlayerIndex;
  }
  if (gs.phase.type === 'action') {
    return gs.phase.actionState.currentPlayerIndex;
  }
  if (gs.phase.type === 'cleanup') {
    return gs.phase.cleanupState.currentPlayerIndex;
  }
  return -1;
}

export function isCurrentPlayerAI(gs: GameState): boolean {
  const idx = getActivePlayerIndex(gs);
  return idx >= 0 && gs.aiPlayers[idx];
}

export function formatTime(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) {
    return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  }
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}
