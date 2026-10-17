import { describe, expect, it } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { initSync } from '../wasm-pkg/colori_wasm';
import { calculateScores } from './wasmEngine';
import { EngineError } from './engineCall';

const wasmPath = resolve(__dirname, '../wasm-pkg/colori_wasm_bg.wasm');
initSync({ module: new WebAssembly.Module(readFileSync(wasmPath)) });

const notPlayers = 'not players' as unknown as Parameters<typeof calculateScores>[0];

describe('an engine call that panics', () => {
  it('throws an error naming the panic, with the input it failed on', () => {
    let error: unknown;
    try {
      calculateScores(notPlayers, []);
    } catch (e) {
      error = e;
    }
    expect(error).toBeInstanceOf(EngineError);
    expect((error as EngineError).message).toMatch(/^colori wasm panicked: .*Failed to parse players JSON/s);
    expect((error as EngineError).input).toEqual({ call: 'calculateScores', players: '"not players"' });
  });

  // The instance lives on after a trap, and the next panic must be named too.
  it('names every panic on the same instance, not just the first', () => {
    expect(() => calculateScores(notPlayers, [])).toThrow(/colori wasm panicked/);
    expect(() => calculateScores(notPlayers, [])).toThrow(/colori wasm panicked/);
    expect(calculateScores([], [])).toEqual([]);
  });
});
