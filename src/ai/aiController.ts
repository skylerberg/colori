import type { GameState, CardInstance } from '../data/types';
import type { ColoriChoice } from './coloriGame';
import AIWorkerModule from './aiWorker?worker';

export class AIController {
  private worker: Worker;
  private precomputeWorker: Worker;

  private precomputeId = 0;
  private precomputedResult: { playerIndex: number; pickNumber: number; choice: ColoriChoice } | null = null;
  private pendingPrecomputeContext: { playerIndex: number; pickNumber: number } | null = null;
  private precomputeResolve: ((choice: ColoriChoice) => void) | null = null;

  constructor() {
    this.worker = new AIWorkerModule();
    this.precomputeWorker = new AIWorkerModule();
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
      const plain = JSON.parse(JSON.stringify({ gameState, playerIndex, iterations, seenHands }));
      this.worker.postMessage(plain);
    });
  }

  precomputeDraftPick(
    gameState: GameState,
    playerIndex: number,
    pickNumber: number,
    iterations: number,
    seenHands?: CardInstance[][],
  ): void {
    const currentId = ++this.precomputeId;
    this.precomputedResult = null;
    this.pendingPrecomputeContext = { playerIndex, pickNumber };
    this.precomputeResolve = null;

    this.precomputeWorker.onmessage = (event: MessageEvent<ColoriChoice>) => {
      if (currentId !== this.precomputeId) return;
      const result = { playerIndex, pickNumber, choice: event.data };
      this.precomputedResult = result;
      if (this.precomputeResolve) {
        this.precomputeResolve(result.choice);
        this.precomputeResolve = null;
      }
    };

    const plain = JSON.parse(JSON.stringify({ gameState, playerIndex, iterations, seenHands }));
    this.precomputeWorker.postMessage(plain);
  }

  waitForPrecomputedChoice(playerIndex: number, pickNumber: number): Promise<ColoriChoice> | null {
    // Check cached result
    if (
      this.precomputedResult &&
      this.precomputedResult.playerIndex === playerIndex &&
      this.precomputedResult.pickNumber === pickNumber
    ) {
      const choice = this.precomputedResult.choice;
      this.precomputedResult = null;
      this.pendingPrecomputeContext = null;
      return Promise.resolve(choice);
    }

    // Check in-progress computation
    if (
      this.pendingPrecomputeContext &&
      this.pendingPrecomputeContext.playerIndex === playerIndex &&
      this.pendingPrecomputeContext.pickNumber === pickNumber &&
      !this.precomputedResult
    ) {
      return new Promise((resolve) => {
        this.precomputeResolve = resolve;
      });
    }

    // No matching precomputation
    return null;
  }

  cancelPrecomputation(): void {
    this.precomputeId++;
    this.precomputedResult = null;
    this.pendingPrecomputeContext = null;
    this.precomputeResolve = null;
  }

  terminate(): void {
    this.worker.terminate();
    this.precomputeWorker.terminate();
  }
}
