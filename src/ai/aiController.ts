import type { GameState, Choice } from '../data/types';
import type { AIWorkerRequest, AIWorkerResponse } from './aiWorker';
import AIWorkerModule from './aiWorker?worker';

export interface DraftPicker {
  playerIndex: number;
  iterations: number;
}

export class AIController {
  // One per seat, so seats can think at the same time: a worker serves one
  // search at a time, and a second would take over the first's reply.
  private workers = new Map<number, Worker>();
  aiStyle: string = 'ga';

  private draftWorkers = new Set<Worker>();
  private startedDraftPicks = new Map<string, Promise<Choice>>();

  getAIChoice(
    gameState: GameState,
    playerIndex: number,
    iterations: number,
  ): Promise<Choice> {
    const worker = this.workerFor(playerIndex);
    return new Promise((resolve, reject) => {
      worker.onmessage = (event: MessageEvent<AIWorkerResponse>) => {
        if (event.data.type === 'error') {
          reject(new Error(event.data.message));
        } else {
          resolve(event.data.choice as Choice);
        }
      };
      worker.postMessage(this.request(gameState, playerIndex, iterations));
    });
  }

  /** Starts searching each picker's draft pick, replacing any not yet claimed. */
  startDraftPicks(gameState: GameState, pickers: DraftPicker[]): void {
    this.cancelDraftPicks();
    for (const picker of pickers) {
      this.startedDraftPicks.set(draftPickKey(gameState, picker.playerIndex), this.searchDraftPick(gameState, picker));
    }
  }

  /** The pick started for this player at this point of the draft, or a new search. */
  draftPick(gameState: GameState, picker: DraftPicker): Promise<Choice> {
    const key = draftPickKey(gameState, picker.playerIndex);
    const started = this.startedDraftPicks.get(key);
    this.startedDraftPicks.delete(key);
    return started ?? this.searchDraftPick(gameState, picker);
  }

  cancelDraftPicks(): void {
    for (const worker of this.draftWorkers) {
      worker.terminate();
    }
    this.draftWorkers.clear();
    this.startedDraftPicks.clear();
  }

  terminate(): void {
    for (const worker of this.workers.values()) {
      worker.terminate();
    }
    this.workers.clear();
    this.cancelDraftPicks();
  }

  private workerFor(playerIndex: number): Worker {
    let worker = this.workers.get(playerIndex);
    if (!worker) {
      worker = new AIWorkerModule();
      this.workers.set(playerIndex, worker);
    }
    return worker;
  }

  // Each draft pick gets its own worker so every AI's pick is searched at once.
  private searchDraftPick(gameState: GameState, { playerIndex, iterations }: DraftPicker): Promise<Choice> {
    const request = this.request(gameState, playerIndex, iterations);
    // The search picks from the current player's hand.
    if (request.gameState.phase.type === 'draft') {
      request.gameState.phase.draftState.currentPlayerIndex = playerIndex;
    }
    const worker = new AIWorkerModule();
    this.draftWorkers.add(worker);
    return new Promise((resolve, reject) => {
      worker.onmessage = (event: MessageEvent<AIWorkerResponse>) => {
        worker.terminate();
        this.draftWorkers.delete(worker);
        if (event.data.type === 'error') {
          reject(new Error(event.data.message));
        } else {
          resolve(event.data.choice as Choice);
        }
      };
      worker.postMessage(request);
    });
  }

  // A plain copy: Svelte state proxies can't be posted to a worker.
  private request(gameState: GameState, playerIndex: number, iterations: number): AIWorkerRequest {
    return JSON.parse(JSON.stringify({ gameState, playerIndex, iterations, aiStyle: this.aiStyle }));
  }
}

function draftPickKey(gameState: GameState, playerIndex: number): string {
  const pickNumber = gameState.phase.type === 'draft' ? gameState.phase.draftState.pickNumber : -1;
  return `${gameState.round}:${pickNumber}:${playerIndex}`;
}
