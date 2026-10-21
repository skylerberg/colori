import init, { AiSearcher } from '../wasm-pkg/colori_wasm.js';
import type { AiSettings, GameState } from '../data/types';
import { engineCall } from '../engine/engineCall';

export interface AIWorkerRequest {
  gameState: GameState;
  playerIndex: number;
  iterations: number;
  aiSettings: AiSettings;
}

export interface AIWorkerSuccess {
  type: 'success';
  choice: unknown;
}

export interface AIWorkerError {
  type: 'error';
  message: string;
}

export type AIWorkerResponse = AIWorkerSuccess | AIWorkerError;

let wasmReady: Promise<unknown> | null = null;
// Held for the worker's lifetime, so a turn's decisions share one search tree.
let searcher: AiSearcher | null = null;

function ensureInit(): Promise<unknown> {
  if (!wasmReady) {
    wasmReady = init();
  }
  return wasmReady;
}

self.onmessage = async (event: MessageEvent<AIWorkerRequest>) => {
  try {
    await ensureInit();
    const { gameState, playerIndex, iterations, aiSettings } = event.data;
    const gameStateJson = JSON.stringify(gameState);

    const aiSettingsJson = JSON.stringify(aiSettings);
    const ai = (searcher ??= new AiSearcher());
    const resultJson = engineCall(() => ai.search(gameStateJson, playerIndex, iterations, aiSettingsJson));

    const choice = JSON.parse(resultJson);
    self.postMessage({ type: 'success', choice } satisfies AIWorkerSuccess);
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    self.postMessage({ type: 'error', message } satisfies AIWorkerError);
  }
};
