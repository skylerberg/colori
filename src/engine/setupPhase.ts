import type { GameState, PlayerState, CardInstance, GarmentCard } from '../data/types';
import { BASIC_DYE_CARDS, FABRIC_CARDS, DYE_CARDS, GARMENT_CARDS } from '../data/cards';
import { createCardInstances, shuffle } from './deckUtils';
import { createEmptyWheel, createEmptyFabrics } from './colorWheel';

/**
 * Create the initial game state for a new game.
 *
 * Each player starts with a personal deck of 10 cards:
 *   2 Basic Red, 2 Basic Yellow, 2 Basic Blue, 1 Wool, 1 Silk, 1 Linen, 1 Cotton.
 *
 * Draft deck contains:
 *   - 4 copies of each of the 15 dye cards (60 total)
 *   - 4 copies of each of the 4 fabric types (16 total)
 *
 * Garment deck: 12 random garments from each of the 4 star tiers (48 total).
 * Garment display: 6 cards dealt from the garment deck.
 * Game starts at round 1, phase = 'draw'.
 */
export function createInitialGameState(playerNames: string[], aiPlayers?: boolean[]): GameState {
  const numPlayers = playerNames.length;

  // Find the specific basic dye and fabric cards by name/type
  const basicRed = BASIC_DYE_CARDS.find(c => c.name === 'Basic Red')!;
  const basicYellow = BASIC_DYE_CARDS.find(c => c.name === 'Basic Yellow')!;
  const basicBlue = BASIC_DYE_CARDS.find(c => c.name === 'Basic Blue')!;
  const woolCard = FABRIC_CARDS.find(c => c.fabricType === 'Wool')!;
  const silkCard = FABRIC_CARDS.find(c => c.fabricType === 'Silk')!;
  const linenCard = FABRIC_CARDS.find(c => c.fabricType === 'Linen')!;
  const cottonCard = FABRIC_CARDS.find(c => c.fabricType === 'Cotton')!;

  // Create players
  const players: PlayerState[] = playerNames.map(name => {
    const personalCards = [
      basicRed, basicRed,
      basicYellow, basicYellow,
      basicBlue, basicBlue,
      woolCard,
      silkCard,
      linenCard,
      cottonCard,
    ];
    const deck = shuffle(createCardInstances(personalCards));
    return {
      name,
      deck,
      discard: [],
      drawnCards: [],
      draftedCards: [],
      colorWheel: createEmptyWheel(),
      fabrics: createEmptyFabrics(),
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

  // 4 copies of each of 4 fabric types
  for (const fabric of FABRIC_CARDS) {
    for (let i = 0; i < 4; i++) {
      draftCards.push(fabric);
    }
  }

  const draftDeck = shuffle(createCardInstances(draftCards));

  // Build garment deck: 12 random garments from each star tier (48 total)
  const garmentsByStars = new Map<number, GarmentCard[]>();
  for (const garment of GARMENT_CARDS) {
    const list = garmentsByStars.get(garment.stars) ?? [];
    list.push(garment);
    garmentsByStars.set(garment.stars, list);
  }
  const selectedGarments: GarmentCard[] = [];
  for (const stars of [2, 3, 4, 5]) {
    const tier = shuffle([...(garmentsByStars.get(stars) ?? [])]);
    selectedGarments.push(...tier.slice(0, 12));
  }
  const garmentDeck = shuffle(createCardInstances(selectedGarments));

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
