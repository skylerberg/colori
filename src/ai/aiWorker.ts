import init, { run_ismcts, wasm_run_nn_mcts } from '../wasm-pkg/colori_wasm.js';
import type { GameState, CardInstance } from '../data/types';

export interface AIWorkerRequest {
  gameState: GameState;
  playerIndex: number;
  iterations: number;
  seenHands?: CardInstance[][];
  useNnMcts?: boolean;
  cPuct?: number;
}

let wasmReady: Promise<unknown> | null = null;

function ensureInit(): Promise<unknown> {
  if (!wasmReady) {
    wasmReady = init();
  }
  return wasmReady;
}

function nnEvaluate(
  _stateEncoding: Float32Array,
  actionEncodings: Float32Array[],
): { priors: Float32Array; value: Float32Array } {
  // Fallback: uniform priors and neutral value.
  // A real implementation would run ONNX Runtime Web inference here.
  const n = actionEncodings.length;
  const priors = new Float32Array(n).fill(1.0 / n);
  const value = new Float32Array(3).fill(1.0 / 3);
  return { priors, value };
}

self.onmessage = async (event: MessageEvent<AIWorkerRequest>) => {
  await ensureInit();
  const { gameState, playerIndex, iterations, seenHands, useNnMcts, cPuct } = event.data;
  const gameStateJson = JSON.stringify(gameState);
  const seenHandsJson = seenHands ? JSON.stringify(seenHands) : '';

  let resultJson: string;

  if (useNnMcts) {
    const evalFn = (stateEnc: Float32Array, actionEncs: Float32Array[]) => {
      return nnEvaluate(stateEnc, actionEncs);
    };
    resultJson = wasm_run_nn_mcts(
      gameStateJson,
      playerIndex,
      iterations,
      cPuct ?? 1.5,
      evalFn,
      seenHandsJson,
    );
  } else {
    resultJson = run_ismcts(gameStateJson, playerIndex, iterations, seenHandsJson);
  }

  const choice = JSON.parse(resultJson);
  self.postMessage(choice);
};
