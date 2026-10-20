import { describe, expect, it } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { initSync, wasm_run_ismcts } from '../wasm-pkg/colori_wasm';
import { setupDraftGame, setupActionGame } from './benchHelper';
import type { AiSettings } from '../data/types';

// Initialise WASM synchronously, as the bench does, for the Node environment.
const wasmPath = resolve(__dirname, '../wasm-pkg/colori_wasm_bg.wasm');
initSync({ module: new WebAssembly.Module(readFileSync(wasmPath)) });

function runSearch(state: object, playerIndex: number, iterations: number, aiSettings: AiSettings = {}) {
  return JSON.parse(wasm_run_ismcts(JSON.stringify(state), playerIndex, iterations, JSON.stringify(aiSettings)));
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

  it('returns a choice for any player during the action phase', () => {
    const state = setupActionGame(3);
    const phase = (state as { phase: { type: string } }).phase;
    expect(phase.type).toBe('action');
    // Everyone acts at once, so any seat can be searched for.
    for (const player of [0, 1, 2]) {
      expect(runSearch(state, player, 200)).toHaveProperty('type');
    }
  });

  it('searches with each kind of advanced AI setting', () => {
    const state = setupDraftGame(2);
    const settings: AiSettings[] = [
      { params: '9iii4n-gen-19' },
      { params: '9iii4n-gen-19-avoid-alum-warehouse-gondola' },
      { params: 'alobyo-gen-45' },
      { rolloutLookahead: 0 },
      { rolloutLookahead: 6 },
    ];
    for (const aiSettings of settings) {
      expect(runSearch(state, 0, 200, aiSettings).type).toBe('draftPick');
    }
  });

  it('is unaffected by the iteration budget being tiny', () => {
    const state = setupDraftGame(2);
    expect(runSearch(state, 0, 1)).toHaveProperty('type');
  });
});
