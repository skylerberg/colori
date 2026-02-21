import { describe, it, expect, beforeEach } from 'vitest';
import type { GameState, ActionState, PlayerState, CardInstance, AnyCard } from '../data/types';
import { BASIC_DYE_CARDS, FABRIC_CARDS, DYE_CARDS } from '../data/cards';
import { resetInstanceIdCounter, createCardInstances } from './deckUtils';
import { createEmptyWheel, storeColor } from './colorWheel';
import {
  initializeActionPhase,
  destroyDraftedCard,
  processQueue,
  resolveMakeMaterials,
  resolveDestroyCards,
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
    fabrics: { Wool: 0, Silk: 0, Linen: 0, Cotton: 0 },
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

    // Basic Red has makeMaterials ability with count 2
    const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
    const instances = createCardInstances([basicRed]);
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

  it('stores fabric card as stored fabric instead of pips', () => {
    const state = makeTestGameState();
    initializeActionPhase(state);

    const woolCard = FABRIC_CARDS.find(c => c.fabricType === 'Wool')!;
    const instances = createCardInstances([woolCard]);
    state.players[0].drawnCards = instances;

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseCardsForMaterials', count: 1 };

    resolveMakeMaterials(state, [instances[0].instanceId]);

    expect(state.players[0].fabrics.Wool).toBe(1);
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

    // Use a card that has drawCards ability so we can verify chaining
    const brazilwood = DYE_CARDS.find(c => c.name === 'Brazilwood')!;
    const instances = createCardInstances([brazilwood]);
    state.players[0].drawnCards = [...instances];

    // Give player cards in their personal deck to draw from
    const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
    state.players[0].deck = createCardInstances([basicRed, basicRed]);

    const actionState = getActionState(state);
    actionState.pendingChoice = { type: 'chooseCardsToDestroy', count: 1 };

    resolveDestroyCards(state, [instances[0].instanceId]);

    // Brazilwood has drawCards: 1, which auto-resolves
    expect(state.destroyedPile).toHaveLength(1);
    // The drawCards ability should have drawn 1 card
    expect(state.players[0].drawnCards).toHaveLength(1);
    expect(state.players[0].deck).toHaveLength(1);
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

  it('transitions to gameOver after round 8', () => {
    const state = makeTestGameState();
    state.round = 8;
    initializeActionPhase(state);

    endRound(state);

    expect(state.round).toBe(9);
    expect(state.phase.type).toBe('gameOver');
  });
});
