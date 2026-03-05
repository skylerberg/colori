import type { GameState, Choice, StructuredGameLog } from './data/types';
import { calculateScores } from './engine/wasmEngine';

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

  recordChoice(state: GameState, choice: Choice, playerIndex: number) {
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
    this.log.finalScores = calculateScores(state.players, state.playerNames).map((s, i) => ({
      ...s,
      completedBuyers: state.players[i].completedBuyers.length,
      colorWheelTotal: Object.values(state.players[i].colorWheel).reduce((sum, c) => sum + (c as number), 0),
    }));
    this.log.finalPlayerStats = state.players.map((p, i) => ({
      name: state.playerNames[i],
      deckSize: p.deck.length + p.discard.length + p.workshopCards.length + p.draftedCards.length + p.workshoppedCards.length,
      completedBuyers: p.completedBuyers,
      ducats: p.ducats,
      colorWheel: { ...p.colorWheel },
      materials: { ...p.materials },
    }));
  }

  getLog(): StructuredGameLog {
    return this.log;
  }

  static fromLog(log: StructuredGameLog): GameLogAccumulator {
    const acc = Object.create(GameLogAccumulator.prototype) as GameLogAccumulator;
    acc.log = log;
    acc.seq = log.entries.length;
    return acc;
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
