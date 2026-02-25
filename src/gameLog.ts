import type { GameState } from './data/types';
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
  entries: StructuredLogEntry[];
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
