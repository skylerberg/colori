import type { AnyCard, Color, DyeCard, BasicDyeCard, MaterialCard, ActionCard, BuyerCard } from './types';

export function getCardPips(card: AnyCard): Color[] {
  switch (card.kind) {
    case 'dye': return card.colors;
    case 'basicDye': return [card.color];
    case 'material': return card.colorPip ? [card.colorPip] : [];
    case 'action': return [];
    case 'buyer': return [];
  }
}

export const DYE_CARDS: DyeCard[] = [
  // Primary (3) — sell
  {
    kind: 'dye',
    name: 'Kermes',
    colors: ['Red', 'Red', 'Red'],
    ability: { type: 'sell' },
  },
  {
    kind: 'dye',
    name: 'Weld',
    colors: ['Yellow', 'Yellow', 'Yellow'],
    ability: { type: 'sell' },
  },
  {
    kind: 'dye',
    name: 'Woad',
    colors: ['Blue', 'Blue', 'Blue'],
    ability: { type: 'sell' },
  },
  // Secondary (6) — workshop x3
  {
    kind: 'dye',
    name: 'Madder',
    colors: ['Orange', 'Red'],
    ability: { type: 'workshop', count: 3 },
  },
  {
    kind: 'dye',
    name: 'Turmeric',
    colors: ['Orange', 'Yellow'],
    ability: { type: 'workshop', count: 3 },
  },
  {
    kind: 'dye',
    name: "Dyer's Greenweed",
    colors: ['Green', 'Yellow'],
    ability: { type: 'workshop', count: 3 },
  },
  {
    kind: 'dye',
    name: 'Verdigris',
    colors: ['Green', 'Blue'],
    ability: { type: 'workshop', count: 3 },
  },
  {
    kind: 'dye',
    name: 'Orchil',
    colors: ['Purple', 'Red'],
    ability: { type: 'workshop', count: 3 },
  },
  {
    kind: 'dye',
    name: 'Logwood',
    colors: ['Purple', 'Blue'],
    ability: { type: 'workshop', count: 3 },
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

export const MATERIAL_CARDS: MaterialCard[] = [
  {
    kind: 'material',
    name: 'Ceramics',
    materialTypes: ['Ceramics'],
    ability: { type: 'workshop', count: 2 },
  },
  {
    kind: 'material',
    name: 'Paintings',
    materialTypes: ['Paintings'],
    ability: { type: 'sell' },
  },
  {
    kind: 'material',
    name: 'Textiles',
    materialTypes: ['Textiles'],
    ability: { type: 'drawCards', count: 2 },
  },
];

export const DRAFT_MATERIAL_CARDS: MaterialCard[] = [
  // Double material cards
  {
    kind: 'material',
    name: 'Fine Ceramics',
    materialTypes: ['Ceramics', 'Ceramics'],
    ability: { type: 'workshop', count: 2 },
  },
  {
    kind: 'material',
    name: 'Fine Paintings',
    materialTypes: ['Paintings', 'Paintings'],
    ability: { type: 'sell' },
  },
  {
    kind: 'material',
    name: 'Fine Textiles',
    materialTypes: ['Textiles', 'Textiles'],
    ability: { type: 'drawCards', count: 2 },
  },
  // Material + color pip cards (Ceramics)
  {
    kind: 'material',
    name: 'Terra Cotta',
    materialTypes: ['Ceramics'],
    colorPip: 'Red',
    ability: { type: 'workshop', count: 2 },
  },
  {
    kind: 'material',
    name: 'Ochre Ware',
    materialTypes: ['Ceramics'],
    colorPip: 'Yellow',
    ability: { type: 'workshop', count: 2 },
  },
  {
    kind: 'material',
    name: 'Cobalt Ware',
    materialTypes: ['Ceramics'],
    colorPip: 'Blue',
    ability: { type: 'workshop', count: 2 },
  },
  // Material + color pip cards (Paintings)
  {
    kind: 'material',
    name: 'Cinnabar & Canvas',
    materialTypes: ['Paintings'],
    colorPip: 'Red',
    ability: { type: 'sell' },
  },
  {
    kind: 'material',
    name: 'Orpiment & Canvas',
    materialTypes: ['Paintings'],
    colorPip: 'Yellow',
    ability: { type: 'sell' },
  },
  {
    kind: 'material',
    name: 'Ultramarine & Canvas',
    materialTypes: ['Paintings'],
    colorPip: 'Blue',
    ability: { type: 'sell' },
  },
  // Material + color pip cards (Textiles)
  {
    kind: 'material',
    name: 'Alizarin & Fabric',
    materialTypes: ['Textiles'],
    colorPip: 'Red',
    ability: { type: 'drawCards', count: 2 },
  },
  {
    kind: 'material',
    name: 'Fustic & Fabric',
    materialTypes: ['Textiles'],
    colorPip: 'Yellow',
    ability: { type: 'drawCards', count: 2 },
  },
  {
    kind: 'material',
    name: 'Pastel & Fabric',
    materialTypes: ['Textiles'],
    colorPip: 'Blue',
    ability: { type: 'drawCards', count: 2 },
  },
  // Dual material cards
  {
    kind: 'material',
    name: 'Clay & Canvas',
    materialTypes: ['Ceramics', 'Paintings'],
    ability: { type: 'destroyCards', count: 1 },
  },
  {
    kind: 'material',
    name: 'Clay & Fabric',
    materialTypes: ['Ceramics', 'Textiles'],
    ability: { type: 'destroyCards', count: 1 },
  },
  {
    kind: 'material',
    name: 'Canvas & Fabric',
    materialTypes: ['Paintings', 'Textiles'],
    ability: { type: 'destroyCards', count: 1 },
  },
];

export const BASIC_DYE_CARDS: BasicDyeCard[] = [
  {
    kind: 'basicDye',
    name: 'Basic Red',
    color: 'Red',
    ability: { type: 'sell' },
  },
  {
    kind: 'basicDye',
    name: 'Basic Yellow',
    color: 'Yellow',
    ability: { type: 'sell' },
  },
  {
    kind: 'basicDye',
    name: 'Basic Blue',
    color: 'Blue',
    ability: { type: 'sell' },
  },
];

export const ACTION_CARDS: ActionCard[] = [
  {
    kind: 'action',
    name: 'Alum',
    ability: { type: 'destroyCards', count: 1 },
    workshopAbilities: [{ type: 'gainDucats', count: 1 }],
  },
  {
    kind: 'action',
    name: 'Cream of Tartar',
    ability: { type: 'destroyCards', count: 1 },
    workshopAbilities: [{ type: 'drawCards', count: 3 }],
  },
  {
    kind: 'action',
    name: 'Gum Arabic',
    ability: { type: 'destroyCards', count: 1 },
    workshopAbilities: [{ type: 'gainSecondary' }],
  },
  {
    kind: 'action',
    name: 'Potash',
    ability: { type: 'destroyCards', count: 1 },
    workshopAbilities: [{ type: 'workshop', count: 3 }],
  },
  {
    kind: 'action',
    name: 'Vinegar',
    ability: { type: 'destroyCards', count: 1 },
    workshopAbilities: [{ type: 'changeTertiary' }],
  },
];

export const PRIMARIES: Color[] = ['Red', 'Yellow', 'Blue'];
export const SECONDARIES: Color[] = ['Orange', 'Green', 'Purple'];
export const TERTIARIES: Color[] = ['Vermilion', 'Amber', 'Chartreuse', 'Teal', 'Indigo', 'Magenta'];

export const CHALK_CARD: ActionCard = {
  kind: 'action',
  name: 'Chalk',
  ability: { type: 'destroyCards', count: 1 },
  workshopAbilities: [{ type: 'gainPrimary' }],
};

function generateAllBuyers(): BuyerCard[] {
  const buyers: BuyerCard[] = [];
  // Textiles (2pt): one tertiary
  for (const t of TERTIARIES)
    buyers.push({ kind: 'buyer', stars: 2, requiredMaterial: 'Textiles', colorCost: [t] });
  // Textiles (2pt): one secondary + one primary
  for (const s of SECONDARIES)
    for (const p of PRIMARIES)
      buyers.push({ kind: 'buyer', stars: 2, requiredMaterial: 'Textiles', colorCost: [s, p] });
  // Ceramics (3pt): one tertiary + one primary
  for (const t of TERTIARIES)
    for (const p of PRIMARIES)
      buyers.push({ kind: 'buyer', stars: 3, requiredMaterial: 'Ceramics', colorCost: [t, p] });
  // Paintings (4pt): one tertiary + one secondary
  for (const t of TERTIARIES)
    for (const s of SECONDARIES)
      buyers.push({ kind: 'buyer', stars: 4, requiredMaterial: 'Paintings', colorCost: [t, s] });
  return buyers;
}

export const BUYER_CARDS: BuyerCard[] = generateAllBuyers();
