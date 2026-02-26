import { describe, bench } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { initSync, run_ismcts } from '../wasm-pkg/colori_wasm';
import { setupDraftGame, setupActionGame } from './benchHelper';

// Initialize WASM synchronously for Node.js (vitest) environment
const wasmPath = resolve(__dirname, '../wasm-pkg/colori_wasm_bg.wasm');
const wasmBytes = readFileSync(wasmPath);
initSync({ module: new WebAssembly.Module(wasmBytes) });

function runWasm(state: object, playerIndex: number, iterations: number): object {
  const json = JSON.stringify(state);
  const resultJson = run_ismcts(json, playerIndex, iterations, '');
  return JSON.parse(resultJson);
}

describe('Draft Phase', () => {
  const state = setupDraftGame(3);

  bench('run_ismcts 1 iteration', () => {
    runWasm(state, 0, 1);
  });

  bench('run_ismcts 100 iterations', () => {
    runWasm(state, 0, 100);
  });

  bench('run_ismcts 1000 iterations', () => {
    runWasm(state, 0, 1000);
  });

  bench('run_ismcts 10000 iterations', () => {
    runWasm(state, 0, 10000);
  });
});

describe('Action Phase', () => {
  const state = setupActionGame(3);

  bench('run_ismcts 1 iteration', () => {
    runWasm(state, 0, 1);
  });

  bench('run_ismcts 100 iterations', () => {
    runWasm(state, 0, 100);
  });

  bench('run_ismcts 1000 iterations', () => {
    runWasm(state, 0, 1000);
  });

  bench('run_ismcts 10000 iterations', () => {
    runWasm(state, 0, 10000);
  });
});
