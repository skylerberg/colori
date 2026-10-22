import init, { BotSearcher } from '../wasm-pkg/colori_wasm.js';
import type { BotSettings, GameState } from '../data/types';
import { engineCall } from '../engine/engineCall';

export interface BotWorkerRequest {
  gameState: GameState;
  playerIndex: number;
  iterations: number;
  botSettings: BotSettings;
}

export interface AIWorkerSuccess {
  type: 'success';
  choice: unknown;
}

export interface AIWorkerError {
  type: 'error';
  message: string;
}

export type BotWorkerResponse = AIWorkerSuccess | AIWorkerError;

let wasmReady: Promise<unknown> | null = null;
// Held for the worker's lifetime, so a turn's decisions share one search tree.
let searcher: BotSearcher | null = null;

function ensureInit(): Promise<unknown> {
  if (!wasmReady) {
    wasmReady = init();
  }
  return wasmReady;
}

self.onmessage = async (event: MessageEvent<BotWorkerRequest>) => {
  try {
    await ensureInit();
    const { gameState, playerIndex, iterations, botSettings } = event.data;
    const gameStateJson = JSON.stringify(gameState);

    const botSettingsJson = JSON.stringify(botSettings);
    const ai = (searcher ??= new BotSearcher());
    const resultJson = engineCall(() => ai.search(gameStateJson, playerIndex, iterations, botSettingsJson));

    const choice = JSON.parse(resultJson);
    self.postMessage({ type: 'success', choice } satisfies AIWorkerSuccess);
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    self.postMessage({ type: 'error', message } satisfies AIWorkerError);
  }
};
