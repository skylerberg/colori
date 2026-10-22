import { describe, expect, it } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { initSync, BotSearcher } from '../wasm-pkg/colori_wasm';
import { setupDraftGame, setupActionGame } from './benchHelper';
import { BOT_PARAMS_CHOICES } from '../data/gameSetup';
import { applyChoice, cloneGameState } from '../engine/wasmEngine';
import { isTakingActionTurn } from '../gameUtils';
import type { BotSettings, Choice, GameState } from '../data/types';

// Initialise WASM synchronously, as the bench does, for the Node environment.
const wasmPath = resolve(__dirname, '../wasm-pkg/colori_wasm_bg.wasm');
initSync({ module: new WebAssembly.Module(readFileSync(wasmPath)) });

function search(searcher: BotSearcher, state: GameState, playerIndex: number, iterations: number, botSettings: BotSettings = {}): Choice {
  return JSON.parse(searcher.search(JSON.stringify(state), playerIndex, iterations, JSON.stringify(botSettings)));
}

function runSearch(state: GameState, playerIndex: number, iterations: number, botSettings: BotSettings = {}): Choice {
  const searcher = new BotSearcher();
  try {
    return search(searcher, state, playerIndex, iterations, botSettings);
  } finally {
    searcher.free();
  }
}

/**
 * The browser game reaches the search through this one export, and nothing else
 * covered it. The Rust tests exercise the search thoroughly but stop at the
 * crate boundary, so a break in serialisation, in wasm-bindgen glue, or in a
 * feature that is only disabled for wasm would not have shown up anywhere.
 */
describe('BotSearcher', () => {
  it('returns a legal-looking choice during the draft', () => {
    const state = setupDraftGame(3);
    const choice = runSearch(state, 0, 200);
    expect(choice).toHaveProperty('type');
    expect(choice.type).toBe('draftPick');
  });

  it('returns a choice for any player during the action phase', () => {
    const state = setupActionGame(3);
    expect(state.phase.type).toBe('action');
    // Everyone acts at once, so any seat can be searched for.
    for (const player of [0, 1, 2]) {
      expect(runSearch(state, player, 200)).toHaveProperty('type');
    }
  });

  it('searches with each kind of advanced bot setting', () => {
    const state = setupDraftGame(2);
    const settings: BotSettings[] = [
      ...BOT_PARAMS_CHOICES.map(choice => ({ params: choice.value })),
      { rolloutLookahead: 0 },
      { rolloutLookahead: 6 },
    ];
    for (const botSettings of settings) {
      expect(runSearch(state, 0, 200, botSettings).type).toBe('draftPick');
    }
  });

  it('is unaffected by the iteration budget being tiny', () => {
    const state = setupDraftGame(2);
    expect(runSearch(state, 0, 1)).toHaveProperty('type');
  });

  // Each seat plays its turn on its own view, applying its choices as
  // `BotActionTurns` does, so the next request has been through the same JSON
  // round trip as in the browser.
  it('carries its tree from one decision of a turn to the next', () => {
    const state = setupActionGame(3);
    const searcher = new BotSearcher();
    const firsts: number[] = [];
    const laters: number[] = [];
    try {
      for (const seat of [0, 1, 2]) {
        const view = cloneGameState(state);
        let first = true;
        while (isTakingActionTurn(view, seat)) {
          const choice = search(searcher, view, seat, 300);
          (first ? firsts : laters).push(searcher.reused_iterations());
          first = false;
          applyChoice(view, seat, choice);
        }
      }
    } finally {
      searcher.free();
    }
    expect(firsts).toEqual([0, 0, 0]);
    expect(laters.some((reused) => reused > 0)).toBe(true);
  });

  it('starts afresh at any position but the one its last choice led to', () => {
    const state = setupActionGame(3);
    const searcher = new BotSearcher();
    try {
      search(searcher, state, 0, 300);
      search(searcher, state, 0, 300);
      expect(searcher.reused_iterations()).toBe(0);
    } finally {
      searcher.free();
    }
  });
});
