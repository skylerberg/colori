import type { GameState, BuyerInstance, Color, MaterialType } from './data/types';
import type { ColoriChoice } from './data/types';
import { calculateScores } from './engine/wasmEngine';

export interface StructuredLogEntry {
  seq: number;
  timestamp: number;
  round: number;
  phase: string;
  playerIndex: number;
  choice: ColoriChoice;
}

export interface PlayerVariant {
  name?: string;
  iterations: number;
  explorationConstant?: number;
  maxRolloutSteps?: number;
  compoundDestroy?: boolean;
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
  durationMs?: number;
  iterations?: number;
  playerVariants?: PlayerVariant[];
  note?: string;
}

export interface FinalPlayerStats {
  name: string;
  deckSize: number;
  completedBuyers: BuyerInstance[];
  ducats: number;
  colorWheel: Record<Color, number>;
  materials: Record<MaterialType, number>;
}

export class GameLogAccumulator {
  private log: StructuredGameLog;
  private seq = 0;

  constructor(initialState: GameState, aiIterations?: number[]) {
    this.log = {
      version: 1,
      gameStartedAt: new Date().toISOString(),
      gameEndedAt: null,
      playerNames: [...initialState.playerNames],
      aiPlayers: [...initialState.aiPlayers],
      initialState: structuredClone(initialState),
      finalScores: null,
      finalPlayerStats: null,
      entries: [],
    };
    if (aiIterations) {
      this.log.playerVariants = aiIterations.map(iterations => ({ iterations }));
    }
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
    this.log.finalScores = calculateScores(state.players, state.playerNames);
    this.log.finalPlayerStats = state.players.map((p, i) => ({
      name: state.playerNames[i],
      deckSize: p.deck.length + p.discard.length + p.workshopCards.length + p.draftedCards.length + p.usedCards.length,
      completedBuyers: p.completedBuyers,
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
