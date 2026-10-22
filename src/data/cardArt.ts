import type { Card, Buyer } from './types';
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

// Each seat plays one character: its starter deck and its player board.
export function seatCharacter(seat: number): number {
  return (seat % CHARACTER_COUNT) + 1;
}

// Setup hands out starter cards first, a deck per seat in seat order, so a
// starter card's instance id says whose deck it was printed for. The card
// keeps that portrait wherever it ends up, as the physical card does.
function starterCharacter(instanceId: number | undefined): number {
  if (instanceId === undefined) return 1;
  return seatCharacter(Math.floor(instanceId / STARTER_DECK_SIZE));
}

export function getCardArtFilename(card: Card, instanceId?: number): string {
  const starter = STARTER_CARD_FILENAMES[card];
  if (starter) return `characters/${starterCharacter(instanceId)}/${starter}.webp`;
  return pascalToKebab(card) + '.webp';
}

export function getBuyerArtFilename(buyer: Buyer): string {
  return `sell/${pascalToKebab(buyer)}.webp`;
}

export function getCardArtUrl(card: Card, instanceId?: number): string {
  return CARD_ART_BASE_PATH + getCardArtFilename(card, instanceId);
}

export function getBuyerArtUrl(buyer: Buyer): string {
  return CARD_ART_BASE_PATH + getBuyerArtFilename(buyer);
}

export const CARD_BACK_URL = CARD_ART_BASE_PATH + 'back.webp';
export const SELL_CARD_BACK_URL = CARD_ART_BASE_PATH + 'sell/back.webp';

export function playerBoardArtUrl(seat: number): string {
  return `/boards/${seatCharacter(seat)}.webp`;
}

export const BUYERS_BOARD_ART_URL = '/boards/buyers.webp';
