import type { Choice, GameState } from '../data/types';
import { applyChoice, cloneGameState } from '../engine/wasmEngine';
import { isTakingActionTurn } from '../gameUtils';
import type { AIController } from './aiController';

/**
 * Plays AI seats' action turns.
 *
 * Everyone acts at once, so an AI decides on its own view of the phase: the
 * game as it stood when the AI started its turn, plus its own moves since. It
 * never sees anyone else's moves of the phase, which is also how its search
 * assumes everyone else plays.
 */
export class AIActionTurns {
  private playing = new Set<number>();
  private generation = 0;

  constructor(private ai: AIController) {}

  /** Seats whose turn is being played. */
  get thinking(): number[] {
    return [...this.playing];
  }

  /**
   * Start the turn of each of `seats` still taking one and not already being
   * played. `play` puts a choice into the live game, and returns false if
   * that seat's choices are no longer wanted there.
   */
  start(
    state: GameState,
    seats: number[],
    iterations: (seat: number) => number,
    play: (seat: number, choice: Choice) => boolean,
    onChange: () => void,
    onError: (error: unknown) => void,
  ): void {
    const generation = this.generation;
    for (const seat of seats) {
      if (this.playing.has(seat) || !isTakingActionTurn(state, seat)) continue;
      this.playing.add(seat);
      this.playTurn(cloneGameState(state), seat, iterations(seat), play, generation)
        .catch(onError)
        .finally(() => {
          if (generation !== this.generation) return;
          this.playing.delete(seat);
          onChange();
        });
    }
    onChange();
  }

  /** Abandon every turn in progress; their remaining choices are dropped. */
  cancel(): void {
    this.generation++;
    this.playing.clear();
  }

  private async playTurn(
    view: GameState,
    seat: number,
    iterations: number,
    play: (seat: number, choice: Choice) => boolean,
    generation: number,
  ): Promise<void> {
    while (isTakingActionTurn(view, seat)) {
      const choice = await this.ai.getAIChoice(view, seat, iterations);
      if (generation !== this.generation || !play(seat, choice)) return;
      applyChoice(view, seat, choice);
    }
  }
}
