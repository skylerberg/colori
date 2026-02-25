import type { GameState, CardInstance, BuyerCard, Color, MaterialType } from './data/types';
import type { ColoriChoice } from './data/types';
import { calculateScores } from './engine/scoring';

export interface StructuredLogEntry {
  seq: number;
  timestamp: number;
  round: number;
  phase: string;
  playerIndex: number;
  choice: ColoriChoice;
}

export interface StructuredGameLog {
  version: 1;
  gameStartedAt: string;
  gameEndedAt: string | null;
  playerNames: string[];
  aiPlayers: boolean[];
  initialState: GameState;
  finalScores: { name: string; score: number }[] | null;
  finalPlayerStats: FinalPlayerStats[] | null;
  entries: StructuredLogEntry[];
}

export interface FinalPlayerStats {
  name: string;
  deckSize: number;
  completedBuyers: CardInstance<BuyerCard>[];
  ducats: number;
  colorWheel: Record<Color, number>;
  materials: Record<MaterialType, number>;
}

export class GameLogAccumulator {
  private log: StructuredGameLog;
  private seq = 0;

  constructor(initialState: GameState) {
    this.log = {
      version: 1,
      gameStartedAt: new Date().toISOString(),
      gameEndedAt: null,
      playerNames: initialState.players.map(p => p.name),
      aiPlayers: [...initialState.aiPlayers],
      initialState: structuredClone(initialState),
      finalScores: null,
      finalPlayerStats: null,
      entries: [],
    };
  }

  recordChoice(state: GameState, choice: ColoriChoice, playerIndex: number) {
    let phase: string;
    if (state.phase.type === 'draft') {
      phase = 'draft';
    } else if (state.phase.type === 'action') {
      phase = 'action';
    } else {
      phase = state.phase.type;
    }

    this.log.entries.push({
      seq: this.seq++,
      timestamp: Date.now(),
      round: state.round,
      phase,
      playerIndex,
      choice,
    });
  }

  finalize(state: GameState) {
    this.log.gameEndedAt = new Date().toISOString();
    this.log.finalScores = calculateScores(state.players);
    this.log.finalPlayerStats = state.players.map(p => ({
      name: p.name,
      deckSize: p.deck.length + p.discard.length + p.workshopCards.length + p.draftedCards.length,
      completedBuyers: p.completedBuyers as CardInstance<BuyerCard>[],
      ducats: p.ducats,
      colorWheel: { ...p.colorWheel },
      materials: { ...p.materials },
    }));
  }

  getLog(): StructuredGameLog {
    return this.log;
  }
}

export function downloadGameLog(log: StructuredGameLog) {
  const json = JSON.stringify(log, null, 2);
  const blob = new Blob([json], { type: 'application/json' });
  const url = URL.createObjectURL(blob);

  const date = log.gameStartedAt.slice(0, 10);
  const names = log.playerNames.join('-');
  const filename = `colori-log-${date}-${names}.json`;

  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
