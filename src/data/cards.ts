import type { AnyCard, Color, DyeCard, BasicDyeCard, MaterialCard, GarmentCard } from './types';

export function getCardPips(card: AnyCard): Color[] {
  switch (card.kind) {
    case 'dye': return card.colors;
    case 'basicDye': return [card.color];
    case 'material': return [];
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

export const MATERIAL_CARDS: MaterialCard[] = [
  {
    kind: 'material',
    name: 'Ceramics',
    materialType: 'Ceramics',
    ability: { type: 'makeMaterials', count: 2 },
  },
  {
    kind: 'material',
    name: 'Paintings',
    materialType: 'Paintings',
    ability: { type: 'makeGarment' },
  },
  {
    kind: 'material',
    name: 'Textiles',
    materialType: 'Textiles',
    ability: { type: 'drawCards', count: 2 },
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

const PRIMARIES: Color[] = ['Red', 'Yellow', 'Blue'];
const SECONDARIES: Color[] = ['Orange', 'Green', 'Purple'];
const TERTIARIES: Color[] = ['Vermilion', 'Amber', 'Chartreuse', 'Teal', 'Indigo', 'Magenta'];

function generateAllGarments(): GarmentCard[] {
  const garments: GarmentCard[] = [];
  // Textiles (2pt): one tertiary
  for (const t of TERTIARIES)
    garments.push({ kind: 'garment', stars: 2, requiredMaterial: 'Textiles', colorCost: [t] });
  // Textiles (2pt): one secondary + one primary
  for (const s of SECONDARIES)
    for (const p of PRIMARIES)
      garments.push({ kind: 'garment', stars: 2, requiredMaterial: 'Textiles', colorCost: [s, p] });
  // Ceramics (3pt): one tertiary + one primary
  for (const t of TERTIARIES)
    for (const p of PRIMARIES)
      garments.push({ kind: 'garment', stars: 3, requiredMaterial: 'Ceramics', colorCost: [t, p] });
  // Paintings (4pt): one tertiary + one secondary
  for (const t of TERTIARIES)
    for (const s of SECONDARIES)
      garments.push({ kind: 'garment', stars: 4, requiredMaterial: 'Paintings', colorCost: [t, s] });
  return garments;
}

export const GARMENT_CARDS: GarmentCard[] = generateAllGarments();
