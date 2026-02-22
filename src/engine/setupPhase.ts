import type { GameState, PlayerState, CardInstance, GarmentCard } from '../data/types';
import { BASIC_DYE_CARDS, MATERIAL_CARDS, DYE_CARDS, GARMENT_CARDS } from '../data/cards';
import { createCardInstances, shuffle } from './deckUtils';
import { createEmptyWheel, createEmptyMaterials } from './colorWheel';

/**
 * Create the initial game state for a new game.
 *
 * Each player starts with a personal deck of 10 cards:
 *   2 Basic Red, 2 Basic Yellow, 2 Basic Blue, 1 Ceramics, 1 Paintings, 1 Textiles, 1 Glass.
 *
 * Draft deck contains:
 *   - 4 copies of each of the 15 dye cards (60 total)
 *   - 4 copies of each of the 4 material types (16 total)
 *
 * Garment deck: all 60 garments shuffled into a single deck.
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
  const glassCard = MATERIAL_CARDS.find(c => c.materialType === 'Glass')!;

  // Create players
  const players: PlayerState[] = playerNames.map(name => {
    const personalCards = [
      basicRed, basicRed,
      basicYellow, basicYellow,
      basicBlue, basicBlue,
      ceramicsCard,
      paintingsCard,
      textilesCard,
      glassCard,
    ];
    const deck = shuffle(createCardInstances(personalCards));
    return {
      name,
      deck,
      discard: [],
      drawnCards: [],
      draftedCards: [],
      colorWheel: createEmptyWheel(),
      materials: createEmptyMaterials(),
      completedGarments: [],
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

  // 4 copies of each of 4 material types
  for (const material of MATERIAL_CARDS) {
    for (let i = 0; i < 4; i++) {
      draftCards.push(material);
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
