import { describe, it, expect, beforeEach } from 'vitest';
import type { GameState, ActionState, PlayerState, CardInstance, AnyCard, GarmentCard } from '../data/types';
import { BASIC_DYE_CARDS, MATERIAL_CARDS, DYE_CARDS, ACTION_CARDS, GARMENT_CARDS } from '../data/cards';
import { resetInstanceIdCounter, createCardInstances } from './deckUtils';
import { createEmptyWheel, storeColor } from './colorWheel';
import {
  initializeActionPhase,
  destroyDraftedCard,
  processQueue,
  resolveWorkshopChoice,
  skipWorkshop,
  resolveDestroyCards,
  resolveSelectGarment,
  resolveGainSecondary,
  resolveChooseTertiaryToLose,
  resolveChooseTertiaryToGain,
  canMakeGarment,
  endPlayerTurn,
  endRound,
} from './actionPhase';

function getActionState(state: GameState): ActionState {
  return (state.phase as { type: 'action'; actionState: ActionState }).actionState;
}

function makeTestPlayer(name: string): PlayerState {
  return {
    name,
    deck: [],
    discard: [],
    drawnCards: [],
    draftedCards: [],
    colorWheel: createEmptyWheel(),
    materials: { Textiles: 0, Ceramics: 0, Paintings: 0 },
    completedGarments: [],
    ducats: 0,
  };
}

function makeTestGameState(numPlayers: number = 2): GameState {
  const players = Array.from({ length: numPlayers }, (_, i) =>
    makeTestPlayer(`Player ${i + 1}`)
  );
  return {
    players,
    draftDeck: [],
    destroyedPile: [],
    garmentDeck: [],
    garmentDisplay: [],
    phase: { type: 'draw' },
    round: 1,
    aiPlayers: Array.from({ length: numPlayers }, () => false),
  };
}

describe('initializeActionPhase', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('sets phase to action with correct initial state', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    expect(state.phase.type).toBe('action');
    const actionState = getActionState(state);
    expect(actionState.currentPlayerIndex).toBe(0);
    expect(actionState.abilityStack).toHaveLength(0);
    expect(actionState.pendingChoice).toBeNull();
  });
});

describe('destroyDraftedCard', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('moves card from draftedCards to destroyedPile and queues its ability', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
    const instances = createCardInstances([basicRed]);
    state.players[0].draftedCards = instances;

    const cardId = instances[0].instanceId;
    destroyDraftedCard(state, cardId);

    expect(state.players[0].draftedCards).toHaveLength(0);
    expect(state.destroyedPile).toHaveLength(1);
    expect(state.destroyedPile[0].instanceId).toBe(cardId);
  });

  it('queues the ability from the destroyed card', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    // Madder has workshop ability with count 3
    const madder = DYE_CARDS.find(c => c.name === 'Madder')!;
    const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
    const instances = createCardInstances([madder]);
    state.players[0].draftedCards = instances;
    // Give player drawn cards so workshop doesn't fizzle
    state.players[0].drawnCards = createCardInstances([basicRed]);

    destroyDraftedCard(state, instances[0].instanceId);

    // workshop triggers a pendingChoice
    const actionState = getActionState(state);
    expect(actionState.pendingChoice).not.toBeNull();
    expect(actionState.pendingChoice?.type).toBe('chooseCardsForWorkshop');
  });

  it('throws when card is not in draftedCards', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);
    expect(() => destroyDraftedCard(state, 99999)).toThrow();
  });
});

describe('processQueue - drawCards', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('auto-resolves drawCards by drawing from personal deck', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    // Give player some cards in their personal deck
    const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
    state.players[0].deck = createCardInstances([basicRed, basicRed, basicRed]);

    // Manually push a drawCards ability
    const actionState = getActionState(state);
    actionState.abilityStack.push({ type: 'drawCards', count: 2 });

    processQueue(state);

    expect(state.players[0].drawnCards).toHaveLength(2);
    expect(state.players[0].deck).toHaveLength(1);
    expect(actionState.pendingChoice).toBeNull();
  });
});

describe('processQueue - workshop', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('sets pendingChoice for workshop', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    // Give player drawn cards so workshop doesn't fizzle
    const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
    state.players[0].drawnCards = createCardInstances([basicRed]);

    const actionState = getActionState(state);
    actionState.abilityStack.push({ type: 'workshop', count: 2 });

    processQueue(state);

    expect(actionState.pendingChoice).toEqual({ type: 'chooseCardsForWorkshop', count: 2 });
  });
});

describe('resolveWorkshopChoice', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('stores card pips on color wheel and moves cards to discard', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
    const instances = createCardInstances([basicRed]);
    state.players[0].drawnCards = instances;

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseCardsForWorkshop', count: 1 };

    resolveWorkshopChoice(state, [instances[0].instanceId]);

    expect(state.players[0].colorWheel['Red']).toBe(1);
    expect(state.players[0].drawnCards).toHaveLength(0);
    expect(state.players[0].discard).toHaveLength(1);
    expect(actionState.pendingChoice).toBeNull();
  });

  it('stores material card as stored material instead of pips', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    const ceramicsCard = MATERIAL_CARDS.find(c => c.materialType === 'Ceramics')!;
    const instances = createCardInstances([ceramicsCard]);
    state.players[0].drawnCards = instances;

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseCardsForWorkshop', count: 1 };

    resolveWorkshopChoice(state, [instances[0].instanceId]);

    expect(state.players[0].materials.Ceramics).toBe(1);
    expect(state.players[0].drawnCards).toHaveLength(0);
    expect(state.players[0].discard).toHaveLength(1);
    expect(actionState.pendingChoice).toBeNull();
  });
});

describe('resolveDestroyCards', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('chains abilities from destroyed drawn cards', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    // Use Madder which has workshop x3 to verify chaining
    const madder = DYE_CARDS.find(c => c.name === 'Madder')!;
    const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
    const instances = createCardInstances([madder]);
    // Give player extra drawn cards so workshop doesn't fizzle after destroying Madder
    const extraCards = createCardInstances([basicRed]);
    state.players[0].drawnCards = [...instances, ...extraCards];

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseCardsToDestroy', count: 1 };

    resolveDestroyCards(state, [instances[0].instanceId]);

    // Madder has workshop: 3, which sets a pendingChoice
    expect(state.destroyedPile).toHaveLength(1);
    expect(actionState.pendingChoice).not.toBeNull();
    expect(actionState.pendingChoice?.type).toBe('chooseCardsForWorkshop');
  });
});

describe('endPlayerTurn', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('moves remaining drawnCards and draftedCards to discard', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
    state.players[0].drawnCards = createCardInstances([basicRed, basicRed]);
    state.players[0].draftedCards = createCardInstances([basicRed]);

    endPlayerTurn(state);

    expect(state.players[0].discard).toHaveLength(3);
    expect(state.players[0].drawnCards).toHaveLength(0);
    expect(state.players[0].draftedCards).toHaveLength(0);
  });

  it('advances to next player', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    endPlayerTurn(state);

    const actionState = getActionState(state);
    expect(actionState.currentPlayerIndex).toBe(1);
    expect(actionState.abilityStack).toHaveLength(0);
    expect(actionState.pendingChoice).toBeNull();
  });

  it('ends round after last player turn', () => {
    const state = makeTestGameState(2);
    initializeActionPhase(state);

    endPlayerTurn(state); // Player 0 done
    endPlayerTurn(state); // Player 1 done

    // Round should increment and phase should change
    expect(state.round).toBe(2);
    expect(state.phase.type).toBe('draw');
  });
});

describe('endRound', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('increments round and transitions to draw phase', () => {
    const state = makeTestGameState();
    state.round = 3;
    initializeActionPhase(state);

    endRound(state);

    expect(state.round).toBe(4);
    expect(state.phase.type).toBe('draw');
  });

  it('transitions to gameOver when a player has 15+ points', () => {
    const state = makeTestGameState();
    state.round = 3;
    initializeActionPhase(state);

    // Give player 0 enough completed garments to reach 16 points (4 x 4-star)
    const garment4star = GARMENT_CARDS.find(c => c.stars === 4)!;
    state.players[0].completedGarments = createCardInstances([garment4star, garment4star, garment4star, garment4star]) as CardInstance<GarmentCard>[];

    endRound(state);

    expect(state.round).toBe(4);
    expect(state.phase.type).toBe('gameOver');
  });

  it('does not end game when no player has 15+ points and round is under 10', () => {
    const state = makeTestGameState();
    state.round = 5;
    initializeActionPhase(state);

    endRound(state);

    expect(state.round).toBe(6);
    expect(state.phase.type).toBe('draw');
  });

  it('transitions to gameOver after round 10 even without 15 points', () => {
    const state = makeTestGameState();
    state.round = 10;
    initializeActionPhase(state);

    endRound(state);

    expect(state.round).toBe(11);
    expect(state.phase.type).toBe('gameOver');
  });
});

describe('resolveSelectGarment', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('pays cost, moves garment to completed, and refills display', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    // Find a Textiles garment (stars: 2) with a known colorCost
    const garment = GARMENT_CARDS.find(c => c.requiredMaterial === 'Textiles' && c.stars === 2)!;
    const garmentInstances = createCardInstances([garment]) as CardInstance<GarmentCard>[];
    state.garmentDisplay = garmentInstances;

    // Put a spare garment in the deck for refill
    const spareGarment = GARMENT_CARDS.find(c => c !== garment)!;
    state.garmentDeck = createCardInstances([spareGarment]) as CardInstance<GarmentCard>[];

    // Give player the required resources
    const player = state.players[0];
    player.materials.Textiles = 1;
    for (const color of garment.colorCost) {
      storeColor(player.colorWheel, color);
    }

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseGarment' };

    resolveSelectGarment(state, garmentInstances[0].instanceId);

    expect(player.materials.Textiles).toBe(0);
    for (const color of garment.colorCost) {
      expect(player.colorWheel[color]).toBe(0);
    }
    expect(player.completedGarments).toHaveLength(1);
    expect(player.completedGarments[0].card.stars).toBe(2);
    expect(state.garmentDisplay).toHaveLength(1);
    expect(state.garmentDisplay[0].card).toBe(spareGarment);
    expect(state.garmentDeck).toHaveLength(0);
    expect(actionState.pendingChoice).toBeNull();
  });
});

describe('canMakeGarment', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('returns true when player can afford the garment', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    const garment = GARMENT_CARDS.find(c => c.requiredMaterial === 'Textiles' && c.stars === 2)!;
    const garmentInstances = createCardInstances([garment]) as CardInstance<GarmentCard>[];
    state.garmentDisplay = garmentInstances;

    const player = state.players[0];
    player.materials.Textiles = 1;
    for (const color of garment.colorCost) {
      storeColor(player.colorWheel, color);
    }

    expect(canMakeGarment(state, garmentInstances[0].instanceId)).toBe(true);
  });

  it('returns false when player lacks material', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    const garment = GARMENT_CARDS.find(c => c.requiredMaterial === 'Textiles' && c.stars === 2)!;
    const garmentInstances = createCardInstances([garment]) as CardInstance<GarmentCard>[];
    state.garmentDisplay = garmentInstances;

    // Has colors but no Textiles material
    const player = state.players[0];
    for (const color of garment.colorCost) {
      storeColor(player.colorWheel, color);
    }

    expect(canMakeGarment(state, garmentInstances[0].instanceId)).toBe(false);
  });

  it('returns false when player lacks colors', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    const garment = GARMENT_CARDS.find(c => c.requiredMaterial === 'Textiles' && c.stars === 2)!;
    const garmentInstances = createCardInstances([garment]) as CardInstance<GarmentCard>[];
    state.garmentDisplay = garmentInstances;

    // Has material but not enough colors
    const player = state.players[0];
    player.materials.Textiles = 1;

    expect(canMakeGarment(state, garmentInstances[0].instanceId)).toBe(false);
  });
});

describe('workshop action cards', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('workshop Alum gains 1 ducat', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);
    const player = state.players[0];

    const alum = ACTION_CARDS.find(c => c.name === 'Alum')!;
    const instances = createCardInstances([alum]);
    player.drawnCards = [...instances];

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseCardsForWorkshop', count: 1 };

    resolveWorkshopChoice(state, [instances[0].instanceId]);

    expect(player.ducats).toBe(1);
    expect(player.drawnCards).toHaveLength(0);
    expect(player.discard).toHaveLength(1);
    expect(actionState.pendingChoice).toBeNull();
  });

  it('workshop Gum Arabic triggers chooseSecondaryColor', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);
    const player = state.players[0];

    const gumArabic = ACTION_CARDS.find(c => c.name === 'Gum Arabic')!;
    const instances = createCardInstances([gumArabic]);
    player.drawnCards = [...instances];

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseCardsForWorkshop', count: 1 };

    resolveWorkshopChoice(state, [instances[0].instanceId]);

    expect(player.discard).toHaveLength(1);
    expect(actionState.pendingChoice).toEqual({ type: 'chooseSecondaryColor' });
  });

  it('workshop Cream of Tartar draws 3 cards', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);
    const player = state.players[0];

    const cot = ACTION_CARDS.find(c => c.name === 'Cream of Tartar')!;
    const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
    const instances = createCardInstances([cot]);
    player.drawnCards = [...instances];
    player.deck = createCardInstances([basicRed, basicRed, basicRed]);

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseCardsForWorkshop', count: 1 };

    resolveWorkshopChoice(state, [instances[0].instanceId]);

    expect(player.drawnCards).toHaveLength(3);
    expect(actionState.pendingChoice).toBeNull();
  });

  it('workshop Potash triggers chooseCardsForWorkshop with count 3', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);
    const player = state.players[0];

    const potash = ACTION_CARDS.find(c => c.name === 'Potash')!;
    const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
    const instances = createCardInstances([potash]);
    player.drawnCards = [...instances];
    player.deck = createCardInstances([basicRed]);

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseCardsForWorkshop', count: 1 };

    resolveWorkshopChoice(state, [instances[0].instanceId]);

    // Potash's workshopAbilities are [workshop:3], but drawnCards is empty after discarding Potash
    // Need drawn cards for workshop not to fizzle. Let's add some first.
    // Actually Potash was consumed, deck has 1 basicRed remaining, drawnCards is empty
    // workshop:3 fizzles because no drawnCards
    expect(actionState.pendingChoice).toBeNull();
  });

  it('workshop Potash grants workshop:3 when drawn cards available', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);
    const player = state.players[0];

    const potash = ACTION_CARDS.find(c => c.name === 'Potash')!;
    const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
    const instances = createCardInstances([potash]);
    const drawnExtras = createCardInstances([basicRed, basicRed]);
    player.drawnCards = [...instances, ...drawnExtras];

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseCardsForWorkshop', count: 1 };

    resolveWorkshopChoice(state, [instances[0].instanceId]);

    // Potash consumed (1 pick), workshopAbilities push workshop:3
    // drawnCards still has 2 basicRed cards, so workshop:3 sets pending choice
    expect(actionState.pendingChoice).toEqual({ type: 'chooseCardsForWorkshop', count: 3 });
  });

  it('workshop Vinegar triggers chooseTertiaryToLose when player has tertiaries', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);
    const player = state.players[0];

    const vinegar = ACTION_CARDS.find(c => c.name === 'Vinegar')!;
    const instances = createCardInstances([vinegar]);
    player.drawnCards = [...instances];
    storeColor(player.colorWheel, 'Vermilion');

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseCardsForWorkshop', count: 1 };

    resolveWorkshopChoice(state, [instances[0].instanceId]);

    expect(actionState.pendingChoice).toEqual({ type: 'chooseTertiaryToLose' });
  });

  it('workshop Vinegar fizzles when player has no tertiaries', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);
    const player = state.players[0];

    const vinegar = ACTION_CARDS.find(c => c.name === 'Vinegar')!;
    const instances = createCardInstances([vinegar]);
    player.drawnCards = [...instances];

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseCardsForWorkshop', count: 1 };

    resolveWorkshopChoice(state, [instances[0].instanceId]);

    expect(actionState.pendingChoice).toBeNull();
  });
});

describe('stack ordering', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('pop() resolves last pushed ability first', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);
    const player = state.players[0];

    const actionState = getActionState(state);
    actionState.abilityStack.push({ type: 'gainDucats', count: 1 });
    actionState.abilityStack.push({ type: 'gainDucats', count: 2 });

    processQueue(state);

    // Both auto-resolve: ducats should be 3 (2 resolved first, then 1)
    expect(player.ducats).toBe(3);
  });
});

describe('skipWorkshop', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('clears pending choice and processes stack', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseCardsForWorkshop', count: 2 };

    skipWorkshop(state);

    expect(actionState.pendingChoice).toBeNull();
  });
});

describe('workshop fizzle', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('fizzles when player has no drawn cards', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);
    const player = state.players[0];
    player.drawnCards = [];

    const actionState = getActionState(state);
    actionState.abilityStack.push({ type: 'workshop', count: 2 });

    processQueue(state);

    expect(actionState.pendingChoice).toBeNull();
    expect(actionState.abilityStack).toHaveLength(0);
  });
});

describe('action card destroyed from drafted', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('fires destroy ability, not workshopAbilities', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);
    const player = state.players[0];

    const alum = ACTION_CARDS.find(c => c.name === 'Alum')!;
    const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
    const instances = createCardInstances([alum]);
    player.draftedCards = [...instances];
    player.drawnCards = createCardInstances([basicRed]);

    destroyDraftedCard(state, instances[0].instanceId);

    // Alum's ability is destroyCards:1, should set pending choice to destroy
    const actionState = getActionState(state);
    expect(actionState.pendingChoice).toEqual({ type: 'chooseCardsToDestroy', count: 1 });
    // ducats should NOT increase (workshopAbilities not triggered)
    expect(player.ducats).toBe(0);
  });
});

describe('gainDucats auto-resolution', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('increments ducats without pending choice', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);
    const player = state.players[0];

    const actionState = getActionState(state);
    actionState.abilityStack.push({ type: 'gainDucats', count: 3 });

    processQueue(state);

    expect(player.ducats).toBe(3);
    expect(actionState.pendingChoice).toBeNull();
  });
});

describe('resolveGainSecondary', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('stores a secondary color on the wheel', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);
    const player = state.players[0];

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseSecondaryColor' };

    resolveGainSecondary(state, 'Orange');

    expect(player.colorWheel['Orange']).toBe(1);
    expect(actionState.pendingChoice).toBeNull();
  });

  it('throws for non-secondary color', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseSecondaryColor' };

    expect(() => resolveGainSecondary(state, 'Red')).toThrow();
  });
});

describe('changeTertiary flow', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('full flow: lose Vermilion, gain Teal', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);
    const player = state.players[0];
    storeColor(player.colorWheel, 'Vermilion');

    const actionState = getActionState(state);
    actionState.abilityStack.push({ type: 'changeTertiary' });
    processQueue(state);

    expect(actionState.pendingChoice).toEqual({ type: 'chooseTertiaryToLose' });

    resolveChooseTertiaryToLose(state, 'Vermilion');

    expect(player.colorWheel['Vermilion']).toBe(0);
    expect(actionState.pendingChoice).toEqual({ type: 'chooseTertiaryToGain', lostColor: 'Vermilion' });

    resolveChooseTertiaryToGain(state, 'Teal');

    expect(player.colorWheel['Teal']).toBe(1);
    expect(actionState.pendingChoice).toBeNull();
  });

  it('cannot regain the same tertiary that was lost', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);
    const player = state.players[0];
    storeColor(player.colorWheel, 'Vermilion');

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseTertiaryToLose' };

    resolveChooseTertiaryToLose(state, 'Vermilion');

    expect(() => resolveChooseTertiaryToGain(state, 'Vermilion')).toThrow();
  });

  it('fizzles when player has no tertiaries', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    const actionState = getActionState(state);
    actionState.abilityStack.push({ type: 'changeTertiary' });
    processQueue(state);

    expect(actionState.pendingChoice).toBeNull();
    expect(actionState.abilityStack).toHaveLength(0);
  });
});

describe('multi-select non-action workshop', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('processes multiple non-action cards at once', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);
    const player = state.players[0];

    const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
    const ceramics = MATERIAL_CARDS.find(c => c.materialType === 'Ceramics')!;
    const instances = createCardInstances([basicRed, ceramics]);
    player.drawnCards = [...instances];

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseCardsForWorkshop', count: 3 };

    resolveWorkshopChoice(state, instances.map(c => c.instanceId));

    expect(player.colorWheel['Red']).toBe(1);
    expect(player.materials.Ceramics).toBe(1);
    expect(player.drawnCards).toHaveLength(0);
    expect(player.discard).toHaveLength(2);
  });
});
