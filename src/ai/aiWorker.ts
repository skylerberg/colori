import { ColoriGame } from './coloriGame';
import type { SeenHands } from './coloriGame';
import { ismcts } from './ismcts';
import type { GameState, CardInstance } from '../data/types';

export interface AIWorkerRequest {
  gameState: GameState;
  playerIndex: number;
  iterations: number;
  seenHands?: CardInstance[][];
}

self.onmessage = (event: MessageEvent<AIWorkerRequest>) => {
  const { gameState, playerIndex, iterations, seenHands } = event.data;
  const game = new ColoriGame(gameState, seenHands);
  const choice = ismcts(game, iterations);
  self.postMessage(choice);
};
