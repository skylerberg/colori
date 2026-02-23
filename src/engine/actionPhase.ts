import type { GameState, ActionState, Ability, Color, CardInstance } from '../data/types';
import { getCardPips, SECONDARIES, TERTIARIES } from '../data/cards';
import { drawFromDeck } from './deckUtils';
import { storeColor, removeColor, performMix, canPayCost, payCost } from './colorWheel';
import { calculateScore } from './scoring';

/** Helper to extract ActionState from GameState with type narrowing. */
function getActionState(state: GameState): ActionState {
  if (state.phase.type !== 'action') {
    throw new Error(`Expected action phase, got ${state.phase.type}`);
  }
  return (state.phase as { type: 'action'; actionState: ActionState }).actionState;
}

/** Helper to extract ability from a non-buyer card. */
function getCardAbility(card: CardInstance): Ability {
  const c = card.card;
  if (c.kind === 'buyer') {
    throw new Error('Buyer cards do not have abilities');
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
    abilityStack: [],
    pendingChoice: null,
  };
  state.phase = { type: 'action', actionState };
}

/**
 * Destroy a drafted card: remove it from the current player's draftedCards,
 * move to destroyedPile, push its ability to the back of abilityStack.
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
  actionState.abilityStack.push(getCardAbility(card));

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
  if (actionState.abilityStack.length === 0) return;

  const ability = actionState.abilityStack.pop()!;
  const player = state.players[actionState.currentPlayerIndex];

  switch (ability.type) {
    case 'drawCards': {
      // Auto-resolve: draw N from personal deck into drawnCards
      const drawn = drawFromDeck(player, ability.count);
      player.drawnCards.push(...drawn);
      processQueue(state);
      break;
    }
    case 'workshop': {
      if (player.drawnCards.length === 0) {
        // Fizzle: no drawn cards
        processQueue(state);
        break;
      }
      actionState.pendingChoice = { type: 'chooseCardsForWorkshop', count: ability.count };
      break;
    }
    case 'gainDucats': {
      player.ducats += ability.count;
      processQueue(state);
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
    case 'sell': {
      // Check if any sale can be made
      if (canSellToAnyBuyer(state)) {
        actionState.pendingChoice = { type: 'chooseBuyer' };
      } else {
        // Fizzle: continue processing queue
        processQueue(state);
      }
      break;
    }
    case 'gainSecondary': {
      actionState.pendingChoice = { type: 'chooseSecondaryColor' };
      break;
    }
    case 'changeTertiary': {
      const hasTertiary = TERTIARIES.some(c => player.colorWheel[c] > 0);
      if (hasTertiary) {
        actionState.pendingChoice = { type: 'chooseTertiaryToLose' };
      } else {
        // Fizzle: no tertiaries to change
        processQueue(state);
      }
      break;
    }
  }
}

/**
 * Check if the current player can afford a specific buyer from the display.
 * Player needs stored material of the required type and can pay the color cost
 * from the color wheel.
 */
export function canSell(state: GameState, buyerInstanceId: number): boolean {
  const actionState = getActionState(state);
  const player = state.players[actionState.currentPlayerIndex];
  const buyerInstance = state.buyerDisplay.find(g => g.instanceId === buyerInstanceId);
  if (!buyerInstance) return false;
  const buyer = buyerInstance.card;
  if (player.materials[buyer.requiredMaterial] <= 0) return false;
  return canPayCost(player.colorWheel, buyer.colorCost);
}

/**
 * Check if the current player can sell to any buyer from the display.
 */
function canSellToAnyBuyer(state: GameState): boolean {
  for (const buyerInstance of state.buyerDisplay) {
    if (canSell(state, buyerInstance.instanceId)) return true;
  }
  return false;
}

/**
 * Resolve workshop choice: handles both action cards and non-action cards.
 * Action cards: consume 1 pick, push workshopAbilities onto stack, remaining picks carry over.
 * Non-action cards: process all selected cards at once (store colors/materials).
 */
export function resolveWorkshopChoice(state: GameState, selectedCardIds: number[]): void {
  const actionState = getActionState(state);
  const player = state.players[actionState.currentPlayerIndex];
  const choice = actionState.pendingChoice;

  if (!choice || choice.type !== 'chooseCardsForWorkshop') {
    throw new Error('No pending workshop choice');
  }

  // Check if selection contains an action card
  const selectedCards = selectedCardIds.map(id => {
    const card = player.drawnCards.find(c => c.instanceId === id);
    if (!card) throw new Error(`Card ${id} not found in player's drawnCards`);
    return card;
  });

  const actionCard = selectedCards.find(c => c.card.kind === 'action');

  if (actionCard) {
    // Action card selected: consume 1 pick
    const cardIndex = player.drawnCards.findIndex(c => c.instanceId === actionCard.instanceId);
    player.drawnCards.splice(cardIndex, 1);

    const remaining = choice.count - 1;
    actionState.pendingChoice = null;

    // Push remaining workshop picks onto stack first (bottom)
    if (remaining > 0) {
      actionState.abilityStack.push({ type: 'workshop', count: remaining });
    }

    // Push workshopAbilities in reverse order so first ability ends up on top
    const abilities = (actionCard.card as import('../data/types').ActionCard).workshopAbilities;
    for (let i = abilities.length - 1; i >= 0; i--) {
      actionState.abilityStack.push({ ...abilities[i] });
    }

    // Move card to discard
    player.discard.push(actionCard);

    processQueue(state);
  } else {
    // Non-action cards: process all at once
    for (const card of selectedCards) {
      const cardIndex = player.drawnCards.findIndex(c => c.instanceId === card.instanceId);
      player.drawnCards.splice(cardIndex, 1);

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
}

/**
 * Skip the current workshop choice. Clear pendingChoice and process stack.
 */
export function skipWorkshop(state: GameState): void {
  const actionState = getActionState(state);
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
 * drawnCards, move to destroyedPile, push its ability to back of abilityStack.
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
    actionState.abilityStack.push(getCardAbility(card));
  }

  actionState.pendingChoice = null;
  processQueue(state);
}

/**
 * Select and pay for a buyer in one step.
 * - Validates the player can afford the buyer.
 * - Decrements the required material from player's stored materials.
 * - Pays the buyer's colorCost from the wheel.
 * - Moves buyer from buyerDisplay to player's completedBuyers.
 * - Refills buyer display from buyer deck (if available).
 * - Clears pendingChoice. Process queue.
 */
export function resolveSelectBuyer(state: GameState, buyerInstanceId: number): void {
  const actionState = getActionState(state);
  const player = state.players[actionState.currentPlayerIndex];

  // Find the buyer in the display
  const buyerIndex = state.buyerDisplay.findIndex(c => c.instanceId === buyerInstanceId);
  if (buyerIndex === -1) {
    throw new Error(`Buyer ${buyerInstanceId} not found in buyer display`);
  }
  const buyer = state.buyerDisplay[buyerIndex];

  // Decrement stored material
  if (player.materials[buyer.card.requiredMaterial] <= 0) {
    throw new Error(`No stored ${buyer.card.requiredMaterial} material`);
  }
  player.materials[buyer.card.requiredMaterial]--;

  // Pay the color cost from the wheel
  const success = payCost(player.colorWheel, buyer.card.colorCost);
  if (!success) {
    throw new Error('Cannot pay buyer color cost from color wheel');
  }

  // Move buyer from display to completed buyers
  state.buyerDisplay.splice(buyerIndex, 1);
  player.completedBuyers.push(buyer);

  // Refill buyer display
  if (state.buyerDeck.length > 0) {
    state.buyerDisplay.push(state.buyerDeck.pop()!);
  }

  actionState.pendingChoice = null;
  processQueue(state);
}

/**
 * Resolve gain secondary: store the chosen secondary color on the player's wheel.
 */
export function resolveGainSecondary(state: GameState, color: Color): void {
  const actionState = getActionState(state);
  const player = state.players[actionState.currentPlayerIndex];

  if (!(SECONDARIES as Color[]).includes(color)) {
    throw new Error(`${color} is not a secondary color`);
  }

  storeColor(player.colorWheel, color);
  actionState.pendingChoice = null;
  processQueue(state);
}

/**
 * Resolve choosing a tertiary to lose: remove it from the wheel,
 * then set pending choice to gain a different tertiary.
 */
export function resolveChooseTertiaryToLose(state: GameState, color: Color): void {
  const actionState = getActionState(state);
  const player = state.players[actionState.currentPlayerIndex];

  if (!(TERTIARIES as Color[]).includes(color)) {
    throw new Error(`${color} is not a tertiary color`);
  }
  if (player.colorWheel[color] <= 0) {
    throw new Error(`Player does not have ${color} on their wheel`);
  }

  removeColor(player.colorWheel, color);
  actionState.pendingChoice = { type: 'chooseTertiaryToGain', lostColor: color };
}

/**
 * Resolve choosing a tertiary to gain: store it on the wheel.
 * Must be different from the lost color.
 */
export function resolveChooseTertiaryToGain(state: GameState, color: Color): void {
  const actionState = getActionState(state);
  const player = state.players[actionState.currentPlayerIndex];
  const pending = actionState.pendingChoice;

  if (!pending || pending.type !== 'chooseTertiaryToGain') {
    throw new Error('No pending chooseTertiaryToGain choice');
  }
  if (!(TERTIARIES as Color[]).includes(color)) {
    throw new Error(`${color} is not a tertiary color`);
  }
  if (color === pending.lostColor) {
    throw new Error(`Cannot gain the same tertiary that was lost`);
  }

  storeColor(player.colorWheel, color);
  actionState.pendingChoice = null;
  processQueue(state);
}

/**
 * End the current player's turn. Move all remaining drawnCards + draftedCards
 * to player's discard. Advance currentPlayerIndex. If all players have gone,
 * call endRound. Otherwise, reset abilityStack and pendingChoice for next player.
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
    actionState.abilityStack = [];
    actionState.pendingChoice = null;
  }
}

/**
 * End the current round. Increment round. If any player has 15+ points
 * or round exceeds 10, set phase to 'gameOver'. Otherwise, set phase to 'draw'.
 */
export function endRound(state: GameState): void {
  state.round++;
  const anyPlayerReached15 = state.players.some(p => calculateScore(p) >= 15);
  if (anyPlayerReached15 || state.round > 10) {
    state.phase = { type: 'gameOver' };
  } else {
    state.phase = { type: 'draw' };
  }
}
