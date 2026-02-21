import type { GameState, CardInstance } from '../data/types';
import { cloneGameState, type ColoriChoice } from './coloriGame';
import AIWorkerModule from './aiWorker?worker';

export class AIController {
  private worker: Worker;

  constructor() {
    this.worker = new AIWorkerModule();
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
      const plainState = cloneGameState(gameState);
      const plainSeenHands = seenHands?.map(hand => [...hand]);
      this.worker.postMessage({ gameState: plainState, playerIndex, iterations, seenHands: plainSeenHands });
    });
  }

  terminate(): void {
    this.worker.terminate();
  }
}
