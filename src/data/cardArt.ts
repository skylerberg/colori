import type { Card, SellCard } from './types';
import { STARTER_DECK_SIZE } from './cards';

const CARD_ART_BASE_PATH = '/cards/';

// The starter decks are printed once per character, identical but for the
// portrait. Each seat plays one of them.
export const CHARACTER_COUNT = 6;

const STARTER_CARD_FILENAMES: Partial<Record<Card, string>> = {
  BasicRed: 'red',
  BasicYellow: 'yellow',
  BasicBlue: 'blue',
  StarterCeramics: 'ceramics',
  StarterCanvas: 'canvas',
  StarterFabric: 'fabric',
  Chalk: 'chalk',
};

function pascalToKebab(s: string): string {
  return s.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase();
}

// Setup hands out starter cards first, a deck per seat in seat order, so a
// starter card's instance id says whose deck it was printed for. The card
// keeps that portrait wherever it ends up, as the physical card does.
function starterCharacter(instanceId: number | undefined): number {
  if (instanceId === undefined) return 1;
  return (Math.floor(instanceId / STARTER_DECK_SIZE) % CHARACTER_COUNT) + 1;
}

export function getCardArtFilename(card: Card, instanceId?: number): string {
  const starter = STARTER_CARD_FILENAMES[card];
  if (starter) return `characters/${starterCharacter(instanceId)}/${starter}.webp`;
  return pascalToKebab(card) + '.webp';
}

export function getSellCardArtFilename(sellCard: SellCard): string {
  return `sell/${pascalToKebab(sellCard)}.webp`;
}

export function getCardArtUrl(card: Card, instanceId?: number): string {
  return CARD_ART_BASE_PATH + getCardArtFilename(card, instanceId);
}

export function getSellCardArtUrl(sellCard: SellCard): string {
  return CARD_ART_BASE_PATH + getSellCardArtFilename(sellCard);
}
