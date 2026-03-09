import type { GameState, CardInstance } from './data/types';

export function getActivePlayerIndex(gs: GameState): number {
  if (gs.phase.type === 'draft') {
    return gs.phase.draftState.currentPlayerIndex;
  }
  if (gs.phase.type === 'action') {
    return gs.phase.actionState.currentPlayerIndex;
  }
  return -1;
}

export function isCurrentPlayerAI(gs: GameState): boolean {
  const idx = getActivePlayerIndex(gs);
  return idx >= 0 && gs.aiPlayers[idx];
}

export function orderByDraftOrder(cards: CardInstance[], draftOrder: number[]): CardInstance[] {
  if (draftOrder.length === 0) return cards;
  const byId = new Map(cards.map(c => [c.instanceId, c]));
  const ordered = draftOrder.filter(id => byId.has(id)).map(id => byId.get(id)!);
  const orderedIds = new Set(draftOrder);
  const remaining = cards.filter(c => !orderedIds.has(c.instanceId));
  return [...ordered, ...remaining];
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
