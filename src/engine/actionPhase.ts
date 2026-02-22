import type { GameState, ActionState, Ability, Color, CardInstance } from '../data/types';
import { getCardPips } from '../data/cards';
import { drawFromDeck } from './deckUtils';
import { storeColor, performMix, canPayCost, payCost } from './colorWheel';
import { calculateScore } from './scoring';

/** Helper to extract ActionState from GameState with type narrowing. */
function getActionState(state: GameState): ActionState {
  if (state.phase.type !== 'action') {
    throw new Error(`Expected action phase, got ${state.phase.type}`);
  }
  return (state.phase as { type: 'action'; actionState: ActionState }).actionState;
}

/** Helper to extract ability from a non-garment card. */
function getCardAbility(card: CardInstance): Ability {
  const c = card.card;
  if (c.kind === 'garment') {
    throw new Error('Garment cards do not have abilities');
  }
  return c.ability;
}

/**
 * Initialize the action phase. Set phase to action, currentPlayerIndex = 0,
 * empty ability queue, null pendingChoice.
 */
export function initializeActionPhase(state: GameState): void {
  const actionState: ActionState = {
    currentPlayerIndex: (state.round - 1) % state.players.length,
    abilityQueue: [],
    pendingChoice: null,
  };
  state.phase = { type: 'action', actionState };
}

/**
 * Destroy a drafted card: remove it from the current player's draftedCards,
 * move to destroyedPile, push its ability to the back of abilityQueue.
 * Then process queue.
 */
export function destroyDraftedCard(state: GameState, cardInstanceId: number): void {
  const actionState = getActionState(state);
  const player = state.players[actionState.currentPlayerIndex];

  const cardIndex = player.draftedCards.findIndex(c => c.instanceId === cardInstanceId);
  if (cardIndex === -1) {
    throw new Error(`Card ${cardInstanceId} not found in player's draftedCards`);
  }

  const [card] = player.draftedCards.splice(cardIndex, 1);
  state.destroyedPile.push(card);

  // Push the card's ability to the queue
  actionState.abilityQueue.push(getCardAbility(card));

  processQueue(state);
}

/**
 * Process the ability queue. If there's a pending choice, wait. If the queue
 * is empty, return (player can destroy more cards or end turn). Otherwise,
 * dequeue front ability and resolve.
 */
export function processQueue(state: GameState): void {
  const actionState = getActionState(state);

  if (actionState.pendingChoice !== null) return;
  if (actionState.abilityQueue.length === 0) return;

  const ability = actionState.abilityQueue.shift()!;
  const player = state.players[actionState.currentPlayerIndex];

  switch (ability.type) {
    case 'drawCards': {
      // Auto-resolve: draw N from personal deck into drawnCards
      const drawn = drawFromDeck(player, ability.count);
      player.drawnCards.push(...drawn);
      processQueue(state);
      break;
    }
    case 'makeMaterials': {
      actionState.pendingChoice = { type: 'chooseCardsForMaterials', count: ability.count };
      break;
    }
    case 'mixColors': {
      actionState.pendingChoice = { type: 'chooseMix', remaining: ability.count };
      break;
    }
    case 'destroyCards': {
      actionState.pendingChoice = { type: 'chooseCardsToDestroy', count: ability.count };
      break;
    }
    case 'makeGarment': {
      // Check if any garment can be made
      if (canMakeAnyGarment(state)) {
        actionState.pendingChoice = { type: 'chooseGarment' };
      } else {
        // Fizzle: continue processing queue
        processQueue(state);
      }
      break;
    }
  }
}

/**
 * Check if the current player can afford a specific garment from the display.
 * Player needs stored material of the required type and can pay the color cost
 * from the color wheel.
 */
export function canMakeGarment(state: GameState, garmentInstanceId: number): boolean {
  const actionState = getActionState(state);
  const player = state.players[actionState.currentPlayerIndex];
  const garmentInstance = state.garmentDisplay.find(g => g.instanceId === garmentInstanceId);
  if (!garmentInstance) return false;
  const garment = garmentInstance.card;
  if (player.materials[garment.requiredMaterial] <= 0) return false;
  return canPayCost(player.colorWheel, garment.colorCost);
}

/**
 * Check if the current player can make any garment from the display.
 */
function canMakeAnyGarment(state: GameState): boolean {
  for (const garmentInstance of state.garmentDisplay) {
    if (canMakeGarment(state, garmentInstance.instanceId)) return true;
  }
  return false;
}

/**
 * Resolve make materials: for each selected card (from drawnCards), if it's a
 * material card, increment the player's stored materials; otherwise get its pips
 * and store each pip on the player's colorWheel. Move those cards to player's
 * discard. Clear pendingChoice. Process queue.
 */
export function resolveMakeMaterials(state: GameState, selectedCardIds: number[]): void {
  const actionState = getActionState(state);
  const player = state.players[actionState.currentPlayerIndex];

  for (const cardId of selectedCardIds) {
    const cardIndex = player.drawnCards.findIndex(c => c.instanceId === cardId);
    if (cardIndex === -1) {
      throw new Error(`Card ${cardId} not found in player's drawnCards`);
    }

    const [card] = player.drawnCards.splice(cardIndex, 1);
    if (card.card.kind === 'material') {
      player.materials[card.card.materialType]++;
    } else {
      const pips = getCardPips(card.card);
      for (const pip of pips) {
        storeColor(player.colorWheel, pip);
      }
    }
    player.discard.push(card);
  }

  actionState.pendingChoice = null;
  processQueue(state);
}

/**
 * Resolve mix colors: perform the mix on the player's colorWheel.
 * Decrement remaining. If remaining > 0, keep pendingChoice. If 0, clear it.
 * Process queue.
 */
export function resolveMixColors(state: GameState, colorA: Color, colorB: Color): void {
  const actionState = getActionState(state);
  const player = state.players[actionState.currentPlayerIndex];

  const success = performMix(player.colorWheel, colorA, colorB);
  if (!success) {
    throw new Error(`Cannot mix ${colorA} and ${colorB}`);
  }

  const choice = actionState.pendingChoice;
  if (choice && choice.type === 'chooseMix') {
    const newRemaining = choice.remaining - 1;
    if (newRemaining > 0) {
      actionState.pendingChoice = { type: 'chooseMix', remaining: newRemaining };
    } else {
      actionState.pendingChoice = null;
    }
  }

  processQueue(state);
}

/**
 * Skip remaining mixes. Clear pendingChoice and process queue.
 */
export function skipMix(state: GameState): void {
  const actionState = getActionState(state);
  actionState.pendingChoice = null;
  processQueue(state);
}

/**
 * Resolve destroy cards: for each selected card (from drawnCards), remove from
 * drawnCards, move to destroyedPile, push its ability to back of abilityQueue.
 * Clear pendingChoice. Process queue.
 */
export function resolveDestroyCards(state: GameState, selectedCardIds: number[]): void {
  const actionState = getActionState(state);
  const player = state.players[actionState.currentPlayerIndex];

  for (const cardId of selectedCardIds) {
    const cardIndex = player.drawnCards.findIndex(c => c.instanceId === cardId);
    if (cardIndex === -1) {
      throw new Error(`Card ${cardId} not found in player's drawnCards`);
    }

    const [card] = player.drawnCards.splice(cardIndex, 1);
    state.destroyedPile.push(card);
    actionState.abilityQueue.push(getCardAbility(card));
  }

  actionState.pendingChoice = null;
  processQueue(state);
}

/**
 * Select and pay for a garment in one step.
 * - Validates the player can afford the garment.
 * - Decrements the required material from player's stored materials.
 * - Pays the garment's colorCost from the wheel.
 * - Moves garment from garmentDisplay to player's completedGarments.
 * - Refills garment display from garment deck (if available).
 * - Clears pendingChoice. Process queue.
 */
export function resolveSelectGarment(state: GameState, garmentInstanceId: number): void {
  const actionState = getActionState(state);
  const player = state.players[actionState.currentPlayerIndex];

  // Find the garment in the display
  const garmentIndex = state.garmentDisplay.findIndex(c => c.instanceId === garmentInstanceId);
  if (garmentIndex === -1) {
    throw new Error(`Garment ${garmentInstanceId} not found in garment display`);
  }
  const garment = state.garmentDisplay[garmentIndex];

  // Decrement stored material
  if (player.materials[garment.card.requiredMaterial] <= 0) {
    throw new Error(`No stored ${garment.card.requiredMaterial} material`);
  }
  player.materials[garment.card.requiredMaterial]--;

  // Pay the color cost from the wheel
  const success = payCost(player.colorWheel, garment.card.colorCost);
  if (!success) {
    throw new Error('Cannot pay garment color cost from color wheel');
  }

  // Move garment from display to completed garments
  state.garmentDisplay.splice(garmentIndex, 1);
  player.completedGarments.push(garment);

  // Refill garment display
  if (state.garmentDeck.length > 0) {
    state.garmentDisplay.push(state.garmentDeck.pop()!);
  }

  actionState.pendingChoice = null;
  processQueue(state);
}

/**
 * End the current player's turn. Move all remaining drawnCards + draftedCards
 * to player's discard. Advance currentPlayerIndex. If all players have gone,
 * call endRound. Otherwise, reset abilityQueue and pendingChoice for next player.
 */
export function endPlayerTurn(state: GameState): void {
  const actionState = getActionState(state);
  const player = state.players[actionState.currentPlayerIndex];

  // Move remaining cards to discard
  player.discard.push(...player.drawnCards);
  player.drawnCards = [];
  player.discard.push(...player.draftedCards);
  player.draftedCards = [];

  const startingPlayer = (state.round - 1) % state.players.length;
  actionState.currentPlayerIndex = (actionState.currentPlayerIndex + 1) % state.players.length;

  if (actionState.currentPlayerIndex === startingPlayer) {
    endRound(state);
  } else {
    actionState.abilityQueue = [];
    actionState.pendingChoice = null;
  }
}

/**
 * End the current round. Increment round. If any player has 15+ points,
 * set phase to 'gameOver'. Otherwise, set phase to 'draw'.
 */
export function endRound(state: GameState): void {
  state.round++;
  const anyPlayerReached15 = state.players.some(p => calculateScore(p) >= 15);
  if (anyPlayerReached15) {
    state.phase = { type: 'gameOver' };
  } else {
    state.phase = { type: 'draw' };
  }
}
