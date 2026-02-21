import type { GameState, CardInstance } from '../data/types';
import type { ColoriChoice } from './coloriGame';
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
      const plain = JSON.parse(JSON.stringify({ gameState, playerIndex, iterations, seenHands }));
      this.worker.postMessage(plain);
    });
  }

  terminate(): void {
    this.worker.terminate();
  }
}
