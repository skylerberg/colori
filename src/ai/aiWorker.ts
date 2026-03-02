import init, { wasm_run_ismcts } from '../wasm-pkg/colori_wasm.js';
import type { GameState, CardInstance } from '../data/types';

export interface AIWorkerRequest {
  gameState: GameState;
  playerIndex: number;
  iterations: number;
  aiDraftKnowledge?: CardInstance[][];
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
  const { gameState, playerIndex, iterations, aiDraftKnowledge } = event.data;
  const gameStateJson = JSON.stringify(gameState);
  const aiDraftKnowledgeJson = aiDraftKnowledge ? JSON.stringify(aiDraftKnowledge) : '';

  const resultJson = wasm_run_ismcts(gameStateJson, playerIndex, iterations, aiDraftKnowledgeJson);

  const choice = JSON.parse(resultJson);
  self.postMessage(choice);
};
