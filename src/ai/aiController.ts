import type { GameState, Choice } from '../data/types';
import type { AIWorkerResponse } from './aiWorker';
import AIWorkerModule from './aiWorker?worker';

export interface PrecomputeRequest {
  gameState: GameState;
  playerIndex: number;
  pickNumber: number;
  iterations: number;
}

interface PrecomputeEntry {
  worker: Worker;
  result: Choice | null;
  resolve: ((choice: Choice) => void) | null;
  reject: ((error: Error) => void) | null;
}

export class AIController {
  private worker: Worker;
  aiStyle: string = 'ga';

  private precomputeMap = new Map<string, PrecomputeEntry>();
  private generationId = 0;

  constructor() {
    this.worker = new AIWorkerModule();
  }

  getAIChoice(
    gameState: GameState,
    playerIndex: number,
    iterations: number,
  ): Promise<Choice> {
    return new Promise((resolve, reject) => {
      this.worker.onmessage = (event: MessageEvent<AIWorkerResponse>) => {
        if (event.data.type === 'error') {
          reject(new Error(event.data.message));
        } else {
          resolve(event.data.choice as Choice);
        }
      };
      const plain = JSON.parse(JSON.stringify({
        gameState,
        playerIndex,
        iterations,
        aiStyle: this.aiStyle,
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
      const entry: PrecomputeEntry = { worker, result: null, resolve: null, reject: null };

      worker.onmessage = (event: MessageEvent<AIWorkerResponse>) => {
        if (gen !== this.generationId) return;
        if (event.data.type === 'error') {
          console.error(`AI precompute error for player ${req.playerIndex}:`, event.data.message);
          entry.result = null;
          if (entry.reject) {
            entry.reject(new Error(event.data.message));
            entry.reject = null;
            entry.resolve = null;
          }
        } else {
          const choice = event.data.choice as Choice;
          entry.result = choice;
          if (entry.resolve) {
            entry.resolve(choice);
            entry.resolve = null;
          }
        }
      };

      const plain = JSON.parse(JSON.stringify({
        gameState: req.gameState,
        playerIndex: req.playerIndex,
        iterations: req.iterations,
        aiStyle: this.aiStyle,
      }));
      worker.postMessage(plain);
      this.precomputeMap.set(key, entry);
    }
  }

  waitForPrecomputedChoice(playerIndex: number, pickNumber: number): Promise<Choice> | null {
    const key = `${playerIndex}:${pickNumber}`;
    const entry = this.precomputeMap.get(key);
    if (!entry) return null;

    if (entry.result !== null) {
      const choice = entry.result;
      entry.worker.terminate();
      this.precomputeMap.delete(key);
      return Promise.resolve(choice);
    }

    return new Promise((resolve, reject) => {
      entry.resolve = (choice: Choice) => {
        entry.worker.terminate();
        this.precomputeMap.delete(key);
        resolve(choice);
      };
      entry.reject = (error: Error) => {
        entry.worker.terminate();
        this.precomputeMap.delete(key);
        reject(error);
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
