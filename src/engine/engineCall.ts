import { wasm_take_failure } from '../wasm-pkg/colori_wasm.js';

/** An engine call that failed, with what it was given so it can be run again. */
export class EngineError extends Error {
  constructor(message: string, readonly input: unknown) {
    super(message);
  }
}

/**
 * Run a call into the engine, naming what killed it if it fails.
 *
 * A panic or a failed allocation in the engine reaches JS only as
 * `RuntimeError: unreachable`. The engine records the reason before trapping,
 * and the error thrown here carries that reason in place of the bare trap.
 */
export function engineCall<T>(call: () => T, input?: unknown): T {
  try {
    return call();
  } catch (e) {
    const reason = e instanceof WebAssembly.RuntimeError ? wasm_take_failure() : undefined;
    throw new EngineError(reason ?? String(e), input);
  }
}
