import { describe, it, expect, beforeEach } from 'vitest';
import type { GameState, ActionState, PlayerState, CardInstance, AnyCard, GarmentCard } from '../data/types';
import { BASIC_DYE_CARDS, MATERIAL_CARDS, DYE_CARDS, GARMENT_CARDS } from '../data/cards';
import { resetInstanceIdCounter, createCardInstances } from './deckUtils';
import { createEmptyWheel, storeColor } from './colorWheel';
import {
  initializeActionPhase,
  destroyDraftedCard,
  processQueue,
  resolveMakeMaterials,
  resolveDestroyCards,
  resolveSelectGarment,
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
    materials: { Glass: 0, Textiles: 0, Ceramics: 0, Paintings: 0 },
    completedGarments: [],
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
    expect(actionState.abilityQueue).toHaveLength(0);
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

    // Madder has makeMaterials ability with count 3
    const madder = DYE_CARDS.find(c => c.name === 'Madder')!;
    const instances = createCardInstances([madder]);
    state.players[0].draftedCards = instances;

    destroyDraftedCard(state, instances[0].instanceId);

    // makeMaterials triggers a pendingChoice
    const actionState = getActionState(state);
    expect(actionState.pendingChoice).not.toBeNull();
    expect(actionState.pendingChoice?.type).toBe('chooseCardsForMaterials');
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
    actionState.abilityQueue.push({ type: 'drawCards', count: 2 });

    processQueue(state);

    expect(state.players[0].drawnCards).toHaveLength(2);
    expect(state.players[0].deck).toHaveLength(1);
    expect(actionState.pendingChoice).toBeNull();
  });
});

describe('processQueue - makeMaterials', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('sets pendingChoice for makeMaterials', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    const actionState = getActionState(state);
    actionState.abilityQueue.push({ type: 'makeMaterials', count: 2 });

    processQueue(state);

    expect(actionState.pendingChoice).toEqual({ type: 'chooseCardsForMaterials', count: 2 });
  });
});

describe('resolveMakeMaterials', () => {
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
    actionState.pendingChoice = { type: 'chooseCardsForMaterials', count: 1 };

    resolveMakeMaterials(state, [instances[0].instanceId]);

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
    actionState.pendingChoice = { type: 'chooseCardsForMaterials', count: 1 };

    resolveMakeMaterials(state, [instances[0].instanceId]);

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

    // Use Madder which has makeMaterials x3 to verify chaining
    const madder = DYE_CARDS.find(c => c.name === 'Madder')!;
    const instances = createCardInstances([madder]);
    state.players[0].drawnCards = [...instances];

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseCardsToDestroy', count: 1 };

    resolveDestroyCards(state, [instances[0].instanceId]);

    // Madder has makeMaterials: 3, which sets a pendingChoice
    expect(state.destroyedPile).toHaveLength(1);
    expect(actionState.pendingChoice).not.toBeNull();
    expect(actionState.pendingChoice?.type).toBe('chooseCardsForMaterials');
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
    expect(actionState.abilityQueue).toHaveLength(0);
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

  it('does not end game when no player has 15+ points even at high round numbers', () => {
    const state = makeTestGameState();
    state.round = 20;
    initializeActionPhase(state);

    endRound(state);

    expect(state.round).toBe(21);
    expect(state.phase.type).toBe('draw');
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
