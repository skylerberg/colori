import init, { run_ismcts } from './wasm-pkg/colori_ai_wasm.js';
import type { GameState, CardInstance } from '../data/types';

export interface AIWorkerRequest {
  gameState: GameState;
  playerIndex: number;
  iterations: number;
  seenHands?: CardInstance[][];
}

let wasmReady: Promise<unknown> | null = null;

function ensureInit(): Promise<unknown> {
  if (!wasmReady) {
    wasmReady = init();
  }
  return wasmReady;
}

self.onmessage = async (event: MessageEvent<AIWorkerRequest>) => {
  await ensureInit();
  const { gameState, playerIndex, iterations, seenHands } = event.data;
  const gameStateJson = JSON.stringify(gameState);
  const seenHandsJson = seenHands ? JSON.stringify(seenHands) : '';
  const resultJson = run_ismcts(gameStateJson, playerIndex, iterations, seenHandsJson);
  const choice = JSON.parse(resultJson);
  self.postMessage(choice);
};
