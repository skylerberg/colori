import { describe, expect, it } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { initSync, wasm_run_ismcts } from '../wasm-pkg/colori_wasm';
import { setupDraftGame, setupActionGame } from './benchHelper';

// Initialise WASM synchronously, as the bench does, for the Node environment.
const wasmPath = resolve(__dirname, '../wasm-pkg/colori_wasm_bg.wasm');
initSync({ module: new WebAssembly.Module(readFileSync(wasmPath)) });

function runSearch(state: object, playerIndex: number, iterations: number) {
  return JSON.parse(wasm_run_ismcts(JSON.stringify(state), playerIndex, iterations, ''));
}

/**
 * The browser game reaches the search through this one export, and nothing else
 * covered it. The Rust tests exercise the search thoroughly but stop at the
 * crate boundary, so a break in serialisation, in wasm-bindgen glue, or in a
 * feature that is only disabled for wasm would not have shown up anywhere.
 */
describe('wasm_run_ismcts', () => {
  it('returns a legal-looking choice during the draft', () => {
    const state = setupDraftGame(3);
    const choice = runSearch(state, 0, 200);
    expect(choice).toHaveProperty('type');
    expect(choice.type).toBe('draftPick');
  });

  it('returns a choice during the action phase', () => {
    const state = setupActionGame(3);
    const phase = (state as { phase: { type: string; actionState?: { currentPlayerIndex: number } } }).phase;
    expect(phase.type).toBe('action');
    const choice = runSearch(state, phase.actionState!.currentPlayerIndex, 200);
    expect(choice).toHaveProperty('type');
  });

  it('is unaffected by the iteration budget being tiny', () => {
    const state = setupDraftGame(2);
    expect(runSearch(state, 0, 1)).toHaveProperty('type');
  });
});
