import type { GameState, DraftState, CardInstance } from '../data/types';
import { shuffle } from './deckUtils';
import { initializeActionPhase } from './actionPhase';

/**
 * Deal 5 cards per player from the draft deck. If the draft deck runs out,
 * shuffle all cards from destroyedPile back into the draft deck and continue
 * dealing. Set direction based on round parity.
 */
export function initializeDraft(state: GameState): void {
  const numPlayers = state.players.length;
  const hands: CardInstance[][] = [];

  for (let p = 0; p < numPlayers; p++) {
    const hand: CardInstance[] = [];
    for (let i = 0; i < 5; i++) {
      if (state.draftDeck.length === 0) {
        if (state.destroyedPile.length > 0) {
          state.draftDeck = shuffle(state.destroyedPile);
          state.destroyedPile = [];
        } else {
          break;
        }
      }
      hand.push(state.draftDeck.pop()!);
    }
    hands.push(hand);
  }

  const draftState: DraftState = {
    pickNumber: 0,
    currentPlayerIndex: 0,
    hands,
    direction: state.round % 2 === 1 ? 1 : -1,
    waitingForPass: false,
  };

  state.phase = { type: 'draft', draftState };
}

/**
 * Current player picks a card from their draft hand.
 * After picking, advance currentPlayerIndex. If all players have picked for
 * this pick round, rotate hands via advanceDraft. Otherwise, set
 * waitingForPass = true.
 */
export function playerPick(state: GameState, cardInstanceId: number): void {
  const draftState = getDraftState(state);
  const playerIndex = draftState.currentPlayerIndex;
  const hand = draftState.hands[playerIndex];

  const cardIndex = hand.findIndex(c => c.instanceId === cardInstanceId);
  if (cardIndex === -1) {
    throw new Error(`Card ${cardInstanceId} not found in player ${playerIndex}'s draft hand`);
  }

  const [card] = hand.splice(cardIndex, 1);
  state.players[playerIndex].draftedCards.push(card);

  draftState.currentPlayerIndex++;

  if (draftState.currentPlayerIndex >= state.players.length) {
    // All players have picked for this round
    advanceDraft(state);
  } else {
    draftState.waitingForPass = true;
  }
}

/**
 * Rotate hands in the draft direction. Increment pickNumber.
 * If pickNumber >= 5, transition to action phase.
 * Otherwise, set currentPlayerIndex = 0 and waitingForPass = true.
 */
export function advanceDraft(state: GameState): void {
  const draftState = getDraftState(state);
  const numPlayers = state.players.length;

  // Rotate hands
  if (draftState.direction === 1) {
    // Forward: each player gets the hand of the player before them
    const last = draftState.hands[numPlayers - 1];
    for (let i = numPlayers - 1; i > 0; i--) {
      draftState.hands[i] = draftState.hands[i - 1];
    }
    draftState.hands[0] = last;
  } else {
    // Backward: each player gets the hand of the player after them
    const first = draftState.hands[0];
    for (let i = 0; i < numPlayers - 1; i++) {
      draftState.hands[i] = draftState.hands[i + 1];
    }
    draftState.hands[numPlayers - 1] = first;
  }

  draftState.pickNumber++;

  if (draftState.pickNumber >= 5) {
    // Drafting is complete, transition to action phase
    initializeActionPhase(state);
  } else {
    draftState.currentPlayerIndex = 0;
    draftState.waitingForPass = true;
  }
}

/**
 * The next player acknowledges the pass screen.
 */
export function confirmPass(state: GameState): void {
  const draftState = getDraftState(state);
  draftState.waitingForPass = false;
}

export function isDraftComplete(draftState: DraftState): boolean {
  return draftState.pickNumber >= 5;
}

/** Helper to extract DraftState from GameState with type narrowing. */
function getDraftState(state: GameState): DraftState {
  if (state.phase.type !== 'draft') {
    throw new Error(`Expected draft phase, got ${state.phase.type}`);
  }
  return (state.phase as { type: 'draft'; draftState: DraftState }).draftState;
}
