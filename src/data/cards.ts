import type { AnyCard, Color, FabricType, DyeCard, BasicDyeCard, FabricCard, GarmentCard } from './types';
import { ALL_COLORS } from './colors';

export function getCardPips(card: AnyCard): Color[] {
  switch (card.kind) {
    case 'dye': return card.colors;
    case 'basicDye': return [card.color];
    case 'fabric': return [];
    case 'garment': return [];
  }
}

export const DYE_CARDS: DyeCard[] = [
  // Primary (3) — makeGarment
  {
    kind: 'dye',
    name: 'Kermes',
    colors: ['Red', 'Red', 'Red'],
    ability: { type: 'makeGarment' },
  },
  {
    kind: 'dye',
    name: 'Weld',
    colors: ['Yellow', 'Yellow', 'Yellow'],
    ability: { type: 'makeGarment' },
  },
  {
    kind: 'dye',
    name: 'Woad',
    colors: ['Blue', 'Blue', 'Blue'],
    ability: { type: 'makeGarment' },
  },
  // Secondary (6) — makeMaterials x3
  {
    kind: 'dye',
    name: 'Madder',
    colors: ['Orange', 'Red'],
    ability: { type: 'makeMaterials', count: 3 },
  },
  {
    kind: 'dye',
    name: 'Turmeric',
    colors: ['Orange', 'Yellow'],
    ability: { type: 'makeMaterials', count: 3 },
  },
  {
    kind: 'dye',
    name: "Dyer's Greenweed",
    colors: ['Green', 'Yellow'],
    ability: { type: 'makeMaterials', count: 3 },
  },
  {
    kind: 'dye',
    name: 'Verdigris',
    colors: ['Green', 'Blue'],
    ability: { type: 'makeMaterials', count: 3 },
  },
  {
    kind: 'dye',
    name: 'Orchil',
    colors: ['Purple', 'Red'],
    ability: { type: 'makeMaterials', count: 3 },
  },
  {
    kind: 'dye',
    name: 'Logwood',
    colors: ['Purple', 'Blue'],
    ability: { type: 'makeMaterials', count: 3 },
  },
  // Tertiary (6) — mixColors x2
  {
    kind: 'dye',
    name: 'Vermilion',
    colors: ['Vermilion'],
    ability: { type: 'mixColors', count: 2 },
  },
  {
    kind: 'dye',
    name: 'Saffron',
    colors: ['Amber'],
    ability: { type: 'mixColors', count: 2 },
  },
  {
    kind: 'dye',
    name: 'Persian Berries',
    colors: ['Chartreuse'],
    ability: { type: 'mixColors', count: 2 },
  },
  {
    kind: 'dye',
    name: 'Azurite',
    colors: ['Teal'],
    ability: { type: 'mixColors', count: 2 },
  },
  {
    kind: 'dye',
    name: 'Indigo',
    colors: ['Indigo'],
    ability: { type: 'mixColors', count: 2 },
  },
  {
    kind: 'dye',
    name: 'Cochineal',
    colors: ['Magenta'],
    ability: { type: 'mixColors', count: 2 },
  },
];

export const FABRIC_CARDS: FabricCard[] = [
  {
    kind: 'fabric',
    name: 'Wool',
    fabricType: 'Wool',
    ability: { type: 'destroyCards', count: 1 },
  },
  {
    kind: 'fabric',
    name: 'Silk',
    fabricType: 'Silk',
    ability: { type: 'destroyCards', count: 1 },
  },
  {
    kind: 'fabric',
    name: 'Linen',
    fabricType: 'Linen',
    ability: { type: 'destroyCards', count: 1 },
  },
  {
    kind: 'fabric',
    name: 'Cotton',
    fabricType: 'Cotton',
    ability: { type: 'destroyCards', count: 1 },
  },
];

export const BASIC_DYE_CARDS: BasicDyeCard[] = [
  {
    kind: 'basicDye',
    name: 'Basic Red',
    color: 'Red',
    ability: { type: 'makeGarment' },
  },
  {
    kind: 'basicDye',
    name: 'Basic Yellow',
    color: 'Yellow',
    ability: { type: 'makeGarment' },
  },
  {
    kind: 'basicDye',
    name: 'Basic Blue',
    color: 'Blue',
    ability: { type: 'makeGarment' },
  },
];

const COLOR_VALUES: Record<Color, number> = {
  Red: 1, Yellow: 1, Blue: 1,
  Orange: 2, Green: 2, Purple: 2,
  Vermilion: 3, Amber: 3, Chartreuse: 3, Teal: 3, Indigo: 3, Magenta: 3,
};

const TIER_MAP: Record<number, { stars: number; fabric: FabricType }> = {
  3: { stars: 2, fabric: 'Cotton' },
  4: { stars: 3, fabric: 'Linen' },
  5: { stars: 4, fabric: 'Wool' },
  6: { stars: 5, fabric: 'Silk' },
};

function generateAllGarments(): GarmentCard[] {
  const colors = ALL_COLORS;
  const garments: GarmentCard[] = [];

  // Enumerate all subsets of the 12 colors (each color at most once)
  for (let mask = 1; mask < (1 << colors.length); mask++) {
    let totalValue = 0;
    const subset: Color[] = [];
    for (let i = 0; i < colors.length; i++) {
      if (mask & (1 << i)) {
        subset.push(colors[i]);
        totalValue += COLOR_VALUES[colors[i]];
      }
    }
    if (totalValue < 3 || totalValue > 6) continue;
    const tier = TIER_MAP[totalValue];
    garments.push({
      kind: 'garment',
      stars: tier.stars,
      requiredFabric: tier.fabric,
      colorCost: subset,
    });
  }

  return garments;
}

export const GARMENT_CARDS: GarmentCard[] = generateAllGarments();
