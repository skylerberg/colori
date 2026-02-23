import type { GameState, PlayerState, CardInstance, GarmentCard } from '../data/types';
import { BASIC_DYE_CARDS, MATERIAL_CARDS, DYE_CARDS, ACTION_CARDS, GARMENT_CARDS } from '../data/cards';
import { createCardInstances, shuffle } from './deckUtils';
import { createEmptyWheel, createEmptyMaterials } from './colorWheel';

/**
 * Create the initial game state for a new game.
 *
 * Each player starts with a personal deck of 6 cards:
 *   1 Basic Red, 1 Basic Yellow, 1 Basic Blue, 1 Ceramics, 1 Paintings, 1 Textiles.
 *
 * Draft deck contains:
 *   - 4 copies of each of the 15 dye cards (60 total)
 *   - 5 copies of each of the 3 material types (15 total)
 *
 * Garment deck: all 51 garments shuffled into a single deck.
 * Garment display: 6 cards dealt from the garment deck.
 * Game starts at round 1, phase = 'draw'.
 */
export function createInitialGameState(playerNames: string[], aiPlayers?: boolean[]): GameState {
  const numPlayers = playerNames.length;

  // Find the specific basic dye and material cards by name/type
  const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
  const basicYellow = BASIC_DYE_CARDS.find(c => c.name === 'Basic Yellow')!;
  const basicBlue = BASIC_DYE_CARDS.find(c => c.name === 'Basic Blue')!;
  const ceramicsCard = MATERIAL_CARDS.find(c => c.materialType === 'Ceramics')!;
  const paintingsCard = MATERIAL_CARDS.find(c => c.materialType === 'Paintings')!;
  const textilesCard = MATERIAL_CARDS.find(c => c.materialType === 'Textiles')!;

  // Create players
  const players: PlayerState[] = playerNames.map(name => {
    const personalCards = [
      basicRed,
      basicYellow,
      basicBlue,
      ceramicsCard,
      paintingsCard,
      textilesCard,
    ];
    const deck = shuffle(createCardInstances(personalCards));
    const colorWheel = createEmptyWheel();
    colorWheel['Red'] = 1;
    colorWheel['Yellow'] = 1;
    colorWheel['Blue'] = 1;
    return {
      name,
      deck,
      discard: [],
      drawnCards: [],
      draftedCards: [],
      colorWheel,
      materials: createEmptyMaterials(),
      completedGarments: [],
      ducats: 0,
    };
  });

  // Build draft deck
  const draftCards = [];

  // 4 copies of each of 15 dye cards
  for (const dye of DYE_CARDS) {
    for (let i = 0; i < 4; i++) {
      draftCards.push(dye);
    }
  }

  // 5 copies of each of 3 material types
  for (const material of MATERIAL_CARDS) {
    for (let i = 0; i < 5; i++) {
      draftCards.push(material);
    }
  }

  // 3 copies of each of 5 action cards
  for (const action of ACTION_CARDS) {
    for (let i = 0; i < 3; i++) {
      draftCards.push(action);
    }
  }

  const draftDeck = shuffle(createCardInstances(draftCards));

  // Build garment deck: all garments shuffled into a single deck
  const garmentDeck = shuffle(createCardInstances(GARMENT_CARDS));

  // Deal 6 garments to the display
  const garmentDisplay: CardInstance<GarmentCard>[] = [];
  for (let i = 0; i < 6 && garmentDeck.length > 0; i++) {
    garmentDisplay.push(garmentDeck.pop()!);
  }

  return {
    players,
    draftDeck,
    destroyedPile: [],
    garmentDeck,
    garmentDisplay,
    phase: { type: 'draw' },
    round: 1,
    aiPlayers: aiPlayers ?? playerNames.map(() => false),
  };
}
