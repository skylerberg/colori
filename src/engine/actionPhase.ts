import type { GameState, ActionState, Ability, Color, CardInstance, GarmentCard } from '../data/types';
import { getCardPips } from '../data/cards';
import { drawFromDeck } from './deckUtils';
import { storeColor, performMix, canPayCost, payCost } from './colorWheel';

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
    currentPlayerIndex: 0,
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
    case 'storeColors': {
      actionState.pendingChoice = { type: 'chooseCardsForStore', count: ability.count };
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
 * Check if the current player can make any garment from the display.
 * Player needs a matching fabric in drawnCards and can pay for at least one
 * garment (either via colorWheel or via a matching dye in drawnCards).
 */
function canMakeAnyGarment(state: GameState): boolean {
  const actionState = getActionState(state);
  const player = state.players[actionState.currentPlayerIndex];

  for (const garmentInstance of state.garmentDisplay) {
    const garment = garmentInstance.card;

    // Check if player has the required fabric in drawnCards
    const hasFabric = player.drawnCards.some(
      c => c.card.kind === 'fabric' && c.card.fabricType === garment.requiredFabric
    );
    if (!hasFabric) continue;

    // Check if player can pay via colorWheel
    if (canPayCost(player.colorWheel, garment.colorCost)) return true;

    // Check if player has a matching dye card in drawnCards
    const hasMatchingDye = player.drawnCards.some(
      c => (c.card.kind === 'dye' || c.card.kind === 'basicDye') && c.card.name === garment.matchingDyeName
    );
    if (hasMatchingDye) return true;
  }

  return false;
}

/**
 * Resolve store colors: for each selected card (from drawnCards), get its pips,
 * store each pip on the player's colorWheel. Move those cards to player's
 * discard. Clear pendingChoice. Process queue.
 */
export function resolveStoreColors(state: GameState, selectedCardIds: number[]): void {
  const actionState = getActionState(state);
  const player = state.players[actionState.currentPlayerIndex];

  for (const cardId of selectedCardIds) {
    const cardIndex = player.drawnCards.findIndex(c => c.instanceId === cardId);
    if (cardIndex === -1) {
      throw new Error(`Card ${cardId} not found in player's drawnCards`);
    }

    const [card] = player.drawnCards.splice(cardIndex, 1);
    const pips = getCardPips(card.card);
    for (const pip of pips) {
      storeColor(player.colorWheel, pip);
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
 * Resolve choose garment: set pendingChoice to chooseGarmentPayment.
 */
export function resolveChooseGarment(state: GameState, garmentInstanceId: number): void {
  const actionState = getActionState(state);
  actionState.pendingChoice = { type: 'chooseGarmentPayment', garmentInstanceId };
}

/**
 * Resolve garment payment.
 * - Remove the fabric card from drawnCards, move to discard.
 * - If paymentType === 'colorWheel': pay the garment's colorCost from the wheel.
 * - If paymentType === 'dyeCard': remove the matching dye card from drawnCards, move to discard.
 * - Move garment from garmentDisplay to player's completedGarments.
 * - Refill garment display from garment deck (if available).
 * - Clear pendingChoice. Process queue.
 */
export function resolveGarmentPayment(
  state: GameState,
  fabricCardId: number,
  paymentType: 'colorWheel' | 'dyeCard',
  dyeCardId?: number,
): void {
  const actionState = getActionState(state);
  const player = state.players[actionState.currentPlayerIndex];

  if (actionState.pendingChoice?.type !== 'chooseGarmentPayment') {
    throw new Error('No garment payment pending');
  }
  const garmentInstanceId = actionState.pendingChoice.garmentInstanceId;

  // Remove fabric card from drawnCards, move to discard
  const fabricIndex = player.drawnCards.findIndex(c => c.instanceId === fabricCardId);
  if (fabricIndex === -1) {
    throw new Error(`Fabric card ${fabricCardId} not found in player's drawnCards`);
  }
  const [fabricCard] = player.drawnCards.splice(fabricIndex, 1);
  player.discard.push(fabricCard);

  // Find the garment in the display
  const garmentIndex = state.garmentDisplay.findIndex(c => c.instanceId === garmentInstanceId);
  if (garmentIndex === -1) {
    throw new Error(`Garment ${garmentInstanceId} not found in garment display`);
  }
  const garment = state.garmentDisplay[garmentIndex];

  // Pay for the garment
  if (paymentType === 'colorWheel') {
    const success = payCost(player.colorWheel, garment.card.colorCost);
    if (!success) {
      throw new Error('Cannot pay garment color cost from color wheel');
    }
  } else if (paymentType === 'dyeCard') {
    if (dyeCardId === undefined) {
      throw new Error('dyeCardId required when paying with dyeCard');
    }
    const dyeIndex = player.drawnCards.findIndex(c => c.instanceId === dyeCardId);
    if (dyeIndex === -1) {
      throw new Error(`Dye card ${dyeCardId} not found in player's drawnCards`);
    }
    const [dyeCard] = player.drawnCards.splice(dyeIndex, 1);
    player.discard.push(dyeCard);
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

  actionState.currentPlayerIndex++;

  if (actionState.currentPlayerIndex >= state.players.length) {
    endRound(state);
  } else {
    actionState.abilityQueue = [];
    actionState.pendingChoice = null;
  }
}

/**
 * End the current round. Increment round. If round > 8, set phase to
 * 'gameOver'. Otherwise, set phase to 'draw'.
 */
export function endRound(state: GameState): void {
  state.round++;
  if (state.round > 8) {
    state.phase = { type: 'gameOver' };
  } else {
    state.phase = { type: 'draw' };
  }
}
