import { describe, expect, it } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { initSync } from '../wasm-pkg/colori_wasm';
import { applyChoice, createInitialGameState, simultaneousPick, advanceDraft } from './wasmEngine';
import type { Choice, GameState } from '../data/types';

// Initialise WASM synchronously, as the bench does, for the Node environment.
const wasmPath = resolve(__dirname, '../wasm-pkg/colori_wasm_bg.wasm');
initSync({ module: new WebAssembly.Module(readFileSync(wasmPath)) });

const WORKSHOP_SIZE = 5;

/**
 * There is no draw phase, and the round boundary moved into the last turn of
 * the round. These pin the round life cycle the frontend drives, which the Rust
 * tests cannot see across the wasm boundary.
 */
describe('the round life cycle', () => {
  it('opens a game with a full workshop and the buyers phase', () => {
    const state = createInitialGameState(['a', 'b', 'c']);
    expect(state.phase.type).toBe('buyers');
    expect(state.round).toBe(1);
    for (const player of state.players) {
      expect(player.workshopCards).toHaveLength(WORKSHOP_SIZE);
      expect(player.topCards).toHaveLength(0);
      expect(player.usedCards).toHaveLength(0);
    }
  });

  it('draws a turn\'s workshop as the turn ends, not as a round opens', () => {
    const state = createInitialGameState(['a', 'b']);
    playToActionPhase(state);
    const seat = actingSeat(state, 0);
    endTurn(state, seat);

    expect(state.players[seat].workshopCards).toHaveLength(WORKSHOP_SIZE);
    // The other seat has not taken its turn, so its board is untouched.
    expect(state.players[1 - seat].topCards.length).toBeGreaterThan(0);
  });

  it('plays seven rounds and ends the game', () => {
    const state = createInitialGameState(['a', 'b', 'c']);
    playWholeGame(state);
    expect(state.phase.type).toBe('gameOver');
    expect(state.round).toBe(state.maxRounds + 1);
  });

  it('tops every workshop up as each new round opens', () => {
    const state = createInitialGameState(['a', 'b']);
    let lastRound = state.round;
    const rounds: number[] = [];
    playWholeGame(state, () => {
      if (state.round === lastRound) return;
      lastRound = state.round;
      rounds.push(state.round);
      if (state.phase.type === 'gameOver') return;
      for (const player of state.players) {
        expect(player.workshopCards, `round ${state.round}`).toHaveLength(WORKSHOP_SIZE);
      }
    });
    expect(rounds).toEqual([2, 3, 4, 5, 6, 7, 8]);
  });
});

/** The seat whose action turn is open, or `fallback` when none is. */
function actingSeat(state: GameState, fallback: number): number {
  if (state.phase.type !== 'action') return fallback;
  return state.phase.actionState.turns.findIndex(turn => !turn.ended);
}

function endTurn(state: GameState, seat: number) {
  while (state.phase.type === 'action') {
    const turn = state.phase.actionState.turns[seat];
    if (turn.ended) return;
    const top = state.players[seat].topCards[0];
    // Keep what is left and end the turn: nothing about this card matters.
    applyChoice(state, seat, top ? { type: 'keepCard', card: top.card } : { type: 'endTurn', keepRemaining: true });
  }
}

/** Claim and draft greedily until the action phase opens. */
function playToActionPhase(state: GameState) {
  let guard = 0;
  while (state.phase.type !== 'action' && state.phase.type !== 'gameOver') {
    if (guard++ > 500) throw new Error('the round never reached its action phase');
    if (state.phase.type === 'buyers') {
      const seat = state.phase.buyersState.currentPlayerIndex;
      const buyer = state.buyerDisplay[0];
      if (buyer) applyChoice(state, seat, { type: 'claimBuyer', buyer: buyer.card });
      else applyChoice(state, seat, { type: 'drawBuyer' });
    } else if (state.phase.type === 'draft') {
      for (let seat = 0; seat < state.players.length; seat++) {
        const hand = state.phase.draftState.hands[seat];
        if (hand[0]) simultaneousPick(state, seat, hand[0].card);
      }
      advanceDraft(state);
    }
  }
}

/**
 * Play greedily to the end, calling `onStep` after every choice so a test can
 * watch the round boundaries go by.
 */
function playWholeGame(state: GameState, onStep: () => void = () => {}): void {
  // A round is a handful of claims, picks and turns, so this cap only catches
  // a phase machine that has stopped moving.
  let guard = 0;
  while (state.phase.type !== 'gameOver') {
    if (guard++ > 500) throw new Error('the game never finished');
    const phase = state.phase;
    if (phase.type === 'buyers') {
      const seat = phase.buyersState.currentPlayerIndex;
      const buyer = state.buyerDisplay[0];
      if (buyer) applyChoice(state, seat, { type: 'claimBuyer', buyer: buyer.card });
      else applyChoice(state, seat, { type: 'drawBuyer' });
    } else if (phase.type === 'draft') {
      const hands = phase.draftState.hands;
      for (let seat = 0; seat < state.players.length; seat++) {
        if (hands[seat][0]) simultaneousPick(state, seat, hands[seat][0].card);
      }
      advanceDraft(state);
    } else {
      endTurn(state, actingSeat(state, 0));
    }
    onStep();
  }
}
