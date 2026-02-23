import type { PlayerState } from '../data/types';

export function calculateScore(player: PlayerState): number {
  return player.completedGarments.reduce((sum, g) => sum + g.card.stars, 0) + player.ducats;
}

export function calculateScores(players: PlayerState[]): { name: string; score: number }[] {
  return players.map(p => ({ name: p.name, score: calculateScore(p) }));
}

export function determineWinner(players: PlayerState[]): string {
  const scores = calculateScores(players);
  scores.sort((a, b) => b.score - a.score);
  return scores[0].name;
}
