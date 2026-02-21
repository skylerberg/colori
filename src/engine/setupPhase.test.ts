import { describe, it, expect, beforeEach } from 'vitest';
import { resetInstanceIdCounter } from './deckUtils';
import { createInitialGameState } from './setupPhase';

describe('createInitialGameState', () => {
  beforeEach(() => {
    resetInstanceIdCounter();
  });

  it('creates the correct number of players', () => {
    const state = createInitialGameState(['Alice', 'Bob']);
    expect(state.players).toHaveLength(2);
    expect(state.players[0].name).toBe('Alice');
    expect(state.players[1].name).toBe('Bob');
  });

  it('gives each player a deck of 10 cards', () => {
    const state = createInitialGameState(['Alice', 'Bob', 'Charlie']);
    for (const player of state.players) {
      expect(player.deck).toHaveLength(10);
    }
  });

  it('each player starts with 2 Basic Red, 2 Basic Yellow, 2 Basic Blue, 1 Wool, 1 Silk, 1 Linen, 1 Cotton', () => {
    const state = createInitialGameState(['Alice']);
    const player = state.players[0];
    const cardNames = player.deck.map(c => c.card.name);

    expect(cardNames.filter(n => n === 'Basic Red')).toHaveLength(2);
    expect(cardNames.filter(n => n === 'Basic Yellow')).toHaveLength(2);
    expect(cardNames.filter(n => n === 'Basic Blue')).toHaveLength(2);

    const fabricTypes = player.deck
      .filter(c => c.card.kind === 'fabric')
      .map(c => (c.card as { fabricType: string }).fabricType);
    expect(fabricTypes).toContain('Wool');
    expect(fabricTypes).toContain('Silk');
    expect(fabricTypes).toContain('Linen');
    expect(fabricTypes).toContain('Cotton');
  });

  it('creates correct draft deck size for 2 players', () => {
    // 2 copies * 39 dyes = 78
    // 8 copies * 4 fabrics = 32
    // Total = 78 + 32 = 110
    const state = createInitialGameState(['Alice', 'Bob']);
    expect(state.draftDeck).toHaveLength(110);
  });

  it('creates correct draft deck size for 3 players', () => {
    // 2 copies * 39 dyes = 78
    // 8 copies * 4 fabrics = 32
    // Total = 78 + 32 = 110
    const state = createInitialGameState(['Alice', 'Bob', 'Charlie']);
    expect(state.draftDeck).toHaveLength(110);
  });

  it('creates garment deck with correct size', () => {
    // 39 garments, minus 6 in display = 33
    const state = createInitialGameState(['Alice', 'Bob']);
    expect(state.garmentDeck).toHaveLength(33);
  });

  it('garment display has 6 cards', () => {
    const state = createInitialGameState(['Alice', 'Bob']);
    expect(state.garmentDisplay).toHaveLength(6);
  });

  it('starts at round 1 with draw phase', () => {
    const state = createInitialGameState(['Alice', 'Bob']);
    expect(state.round).toBe(1);
    expect(state.phase).toEqual({ type: 'draw' });
  });

  it('all players start with empty discard, drawnCards, draftedCards', () => {
    const state = createInitialGameState(['Alice', 'Bob']);
    for (const player of state.players) {
      expect(player.discard).toHaveLength(0);
      expect(player.drawnCards).toHaveLength(0);
      expect(player.draftedCards).toHaveLength(0);
      expect(player.completedGarments).toHaveLength(0);
    }
  });

  it('destroyed pile starts empty', () => {
    const state = createInitialGameState(['Alice', 'Bob']);
    expect(state.destroyedPile).toHaveLength(0);
  });
});
