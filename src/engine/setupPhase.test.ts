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

  it('each player starts with 2 Basic Red, 2 Basic Yellow, 2 Basic Blue, 1 Ceramics, 1 Paintings, 1 Textiles, 1 Glass', () => {
    const state = createInitialGameState(['Alice']);
    const player = state.players[0];
    const cardNames = player.deck.map(c => 'name' in c.card ? c.card.name : '');

    expect(cardNames.filter(n => n === 'Basic Red')).toHaveLength(2);
    expect(cardNames.filter(n => n === 'Basic Yellow')).toHaveLength(2);
    expect(cardNames.filter(n => n === 'Basic Blue')).toHaveLength(2);

    const materialTypes = player.deck
      .filter(c => c.card.kind === 'material')
      .map(c => (c.card as { materialType: string }).materialType);
    expect(materialTypes).toContain('Ceramics');
    expect(materialTypes).toContain('Paintings');
    expect(materialTypes).toContain('Textiles');
    expect(materialTypes).toContain('Glass');
  });

  it('creates correct draft deck size for 2 players', () => {
    // 4 copies * 15 dyes = 60
    // 4 copies * 4 materials = 16
    // Total = 60 + 16 = 76
    const state = createInitialGameState(['Alice', 'Bob']);
    expect(state.draftDeck).toHaveLength(76);
  });

  it('creates correct draft deck size for 3 players', () => {
    // 4 copies * 15 dyes = 60
    // 4 copies * 4 materials = 16
    // Total = 60 + 16 = 76
    const state = createInitialGameState(['Alice', 'Bob', 'Charlie']);
    expect(state.draftDeck).toHaveLength(76);
  });

  it('creates garment deck with correct size', () => {
    // 60 total garments minus 6 in display = 54
    const state = createInitialGameState(['Alice', 'Bob']);
    expect(state.garmentDeck).toHaveLength(54);
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
