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

// Special-case overrides where PascalCase->kebab-case doesn't match the art filename
const CARD_FILENAME_OVERRIDES: Partial<Record<Card, string>> = {
  IndigoDye: 'indigo',
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
  return (CARD_FILENAME_OVERRIDES[card] ?? pascalToKebab(card)) + '.webp';
}

// Sell card mapping: "Textiles2Vermilion" -> "vermilion-textile.webp"
// "Ceramics3AmberRed" -> "amber-red-ceramic.webp"
// "Paintings4TealOrange" -> "teal-orange-painting.webp"
// "Textiles2RedRedRed" -> "red-red-red-textile.webp"
const MATERIAL_SINGULAR: Record<string, string> = {
  Textiles: 'textile',
  Ceramics: 'ceramic',
  Paintings: 'painting',
};

export function getSellCardArtFilename(sellCard: SellCard): string {
  const match = sellCard.match(/^(Textiles|Ceramics|Paintings)(\d)(.+)$/);
  if (!match) return 'empty-pigment.webp'; // fallback
  const [, material, , colorPart] = match;
  // Split PascalCase color names: "AmberRed" -> ["Amber", "Red"], "RedRedRed" -> ["Red", "Red", "Red"]
  const colors = colorPart.match(/[A-Z][a-z]*/g) ?? [];
  const colorKebab = colors.map(c => c.toLowerCase()).join('-');
  return colorKebab + '-' + MATERIAL_SINGULAR[material] + '.webp';
}

export function getCardArtUrl(card: Card, instanceId?: number): string {
  return CARD_ART_BASE_PATH + getCardArtFilename(card, instanceId);
}

export function getSellCardArtUrl(sellCard: SellCard): string {
  return CARD_ART_BASE_PATH + getSellCardArtFilename(sellCard);
}
