import type { GameState, CardInstance, ColoriChoice } from '../data/types';
import AIWorkerModule from './aiWorker?worker';

export interface PrecomputeRequest {
  gameState: GameState;
  playerIndex: number;
  pickNumber: number;
  iterations: number;
  seenHands?: CardInstance[][];
}

interface PrecomputeEntry {
  worker: Worker;
  result: ColoriChoice | null;
  resolve: ((choice: ColoriChoice) => void) | null;
}

export class AIController {
  private worker: Worker;

  private precomputeMap = new Map<string, PrecomputeEntry>();
  private generationId = 0;
  private useNnMcts = false;
  private cPuct = 1.5;

  constructor() {
    this.worker = new AIWorkerModule();
  }

  setNnMctsMode(enabled: boolean, cPuct = 1.5): void {
    this.useNnMcts = enabled;
    this.cPuct = cPuct;
  }

  getAIChoice(
    gameState: GameState,
    playerIndex: number,
    iterations: number,
    seenHands?: CardInstance[][],
  ): Promise<ColoriChoice> {
    return new Promise((resolve) => {
      this.worker.onmessage = (event: MessageEvent<ColoriChoice>) => {
        resolve(event.data);
      };
      const plain = JSON.parse(JSON.stringify({
        gameState,
        playerIndex,
        iterations,
        seenHands,
        useNnMcts: this.useNnMcts,
        cPuct: this.cPuct,
      }));
      this.worker.postMessage(plain);
    });
  }

  precomputeDraftPicks(requests: PrecomputeRequest[]): void {
    this.cancelPrecomputation();
    const gen = this.generationId;

    for (const req of requests) {
      const key = `${req.playerIndex}:${req.pickNumber}`;
      const worker = new AIWorkerModule();
      const entry: PrecomputeEntry = { worker, result: null, resolve: null };

      worker.onmessage = (event: MessageEvent<ColoriChoice>) => {
        if (gen !== this.generationId) return;
        entry.result = event.data;
        if (entry.resolve) {
          entry.resolve(event.data);
          entry.resolve = null;
        }
      };

      const plain = JSON.parse(JSON.stringify({
        gameState: req.gameState,
        playerIndex: req.playerIndex,
        iterations: req.iterations,
        seenHands: req.seenHands,
      }));
      worker.postMessage(plain);
      this.precomputeMap.set(key, entry);
    }
  }

  waitForPrecomputedChoice(playerIndex: number, pickNumber: number): Promise<ColoriChoice> | null {
    const key = `${playerIndex}:${pickNumber}`;
    const entry = this.precomputeMap.get(key);
    if (!entry) return null;

    if (entry.result !== null) {
      const choice = entry.result;
      entry.worker.terminate();
      this.precomputeMap.delete(key);
      return Promise.resolve(choice);
    }

    return new Promise((resolve) => {
      entry.resolve = (choice: ColoriChoice) => {
        entry.worker.terminate();
        this.precomputeMap.delete(key);
        resolve(choice);
      };
    });
  }

  cancelPrecomputation(): void {
    this.generationId++;
    for (const entry of this.precomputeMap.values()) {
      entry.worker.terminate();
    }
    this.precomputeMap.clear();
  }

  terminate(): void {
    this.worker.terminate();
    this.cancelPrecomputation();
  }
}
