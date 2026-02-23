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

  it('gives each player a deck of 6 cards', () => {
    const state = createInitialGameState(['Alice', 'Bob', 'Charlie']);
    for (const player of state.players) {
      expect(player.deck).toHaveLength(6);
    }
  });

  it('each player starts with 1 Basic Red, 1 Basic Yellow, 1 Basic Blue, 1 Ceramics, 1 Paintings, 1 Textiles', () => {
    const state = createInitialGameState(['Alice']);
    const player = state.players[0];
    const cardNames = player.deck.map(c => 'name' in c.card ? c.card.name : '');

    expect(cardNames.filter(n => n === 'Basic Red')).toHaveLength(1);
    expect(cardNames.filter(n => n === 'Basic Yellow')).toHaveLength(1);
    expect(cardNames.filter(n => n === 'Basic Blue')).toHaveLength(1);

    const materialTypes = player.deck
      .filter(c => c.card.kind === 'material')
      .map(c => (c.card as { materialType: string }).materialType);
    expect(materialTypes).toContain('Ceramics');
    expect(materialTypes).toContain('Paintings');
    expect(materialTypes).toContain('Textiles');
  });

  it('creates correct draft deck size for 2 players', () => {
    // 4 copies * 15 dyes = 60
    // 5 copies * 3 materials = 15
    // Total = 60 + 15 = 75
    const state = createInitialGameState(['Alice', 'Bob']);
    expect(state.draftDeck).toHaveLength(75);
  });

  it('creates correct draft deck size for 3 players', () => {
    // 4 copies * 15 dyes = 60
    // 5 copies * 3 materials = 15
    // Total = 60 + 15 = 75
    const state = createInitialGameState(['Alice', 'Bob', 'Charlie']);
    expect(state.draftDeck).toHaveLength(75);
  });

  it('creates garment deck with correct size', () => {
    // 51 total garments minus 6 in display = 45
    const state = createInitialGameState(['Alice', 'Bob']);
    expect(state.garmentDeck).toHaveLength(45);
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

  it('each player starts with 1 Red, 1 Yellow, and 1 Blue on their color wheel', () => {
    const state = createInitialGameState(['Alice', 'Bob']);
    for (const player of state.players) {
      expect(player.colorWheel['Red']).toBe(1);
      expect(player.colorWheel['Yellow']).toBe(1);
      expect(player.colorWheel['Blue']).toBe(1);
    }
  });

  it('destroyed pile starts empty', () => {
    const state = createInitialGameState(['Alice', 'Bob']);
    expect(state.destroyedPile).toHaveLength(0);
  });
});
