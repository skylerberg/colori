import type { Card, BuyerCard, Color, AnyCardData, DyeCardData, BasicDyeCardData, MaterialCardData, ActionCardData, BuyerCardData } from './types';

export const DYE_CARDS: DyeCardData[] = [
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

export const MATERIAL_CARDS: MaterialCardData[] = [
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
    ability: { type: 'workshop', count: 2 },
  },
];

export const DRAFT_MATERIAL_CARDS: MaterialCardData[] = [
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

export const BASIC_DYE_CARDS: BasicDyeCardData[] = [
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

export const ACTION_CARDS: ActionCardData[] = [
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

export const CHALK_CARD: ActionCardData = {
  kind: 'action',
  name: 'Chalk',
  ability: { type: 'sell' },
  workshopAbilities: [{ type: 'gainPrimary' }],
};

function generateAllBuyers(): BuyerCardData[] {
  const buyers: BuyerCardData[] = [];
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

export const BUYER_CARDS: BuyerCardData[] = generateAllBuyers();

// Build lookup map from Card variant name -> card data
const CARD_LOOKUP: Record<string, AnyCardData> = {};

// Basic dyes
CARD_LOOKUP['BasicRed'] = { kind: 'basicDye', name: 'Basic Red', color: 'Red', ability: { type: 'sell' } };
CARD_LOOKUP['BasicYellow'] = { kind: 'basicDye', name: 'Basic Yellow', color: 'Yellow', ability: { type: 'sell' } };
CARD_LOOKUP['BasicBlue'] = { kind: 'basicDye', name: 'Basic Blue', color: 'Blue', ability: { type: 'sell' } };

// Register all dye cards by matching name to variant
for (const dye of DYE_CARDS) {
  let variantName: string;
  switch (dye.name) {
    case 'Kermes': variantName = 'Kermes'; break;
    case 'Weld': variantName = 'Weld'; break;
    case 'Woad': variantName = 'Woad'; break;
    case 'Madder': variantName = 'Madder'; break;
    case 'Turmeric': variantName = 'Turmeric'; break;
    case "Dyer's Greenweed": variantName = 'DyersGreenweed'; break;
    case 'Verdigris': variantName = 'Verdigris'; break;
    case 'Orchil': variantName = 'Orchil'; break;
    case 'Logwood': variantName = 'Logwood'; break;
    case 'Vermilion': variantName = 'VermilionDye'; break;
    case 'Saffron': variantName = 'Saffron'; break;
    case 'Persian Berries': variantName = 'PersianBerries'; break;
    case 'Azurite': variantName = 'Azurite'; break;
    case 'Indigo': variantName = 'IndigoDye'; break;
    case 'Cochineal': variantName = 'Cochineal'; break;
    default: variantName = dye.name; break;
  }
  CARD_LOOKUP[variantName] = dye;
}

// Starter materials
CARD_LOOKUP['StarterCeramics'] = MATERIAL_CARDS[0];  // Ceramics
CARD_LOOKUP['StarterPaintings'] = MATERIAL_CARDS[1];  // Paintings
CARD_LOOKUP['StarterTextiles'] = MATERIAL_CARDS[2];  // Textiles

// Draft materials - map by name to variant
const DRAFT_MATERIAL_NAME_MAP: Record<string, string> = {
  'Fine Ceramics': 'FineCeramics',
  'Fine Paintings': 'FinePaintings',
  'Fine Textiles': 'FineTextiles',
  'Terra Cotta': 'TerraCotta',
  'Ochre Ware': 'OchreWare',
  'Cobalt Ware': 'CobaltWare',
  'Cinnabar & Canvas': 'CinnabarCanvas',
  'Orpiment & Canvas': 'OrpimentCanvas',
  'Ultramarine & Canvas': 'UltramarineCanvas',
  'Alizarin & Fabric': 'AlizarinFabric',
  'Fustic & Fabric': 'FusticFabric',
  'Pastel & Fabric': 'PastelFabric',
  'Clay & Canvas': 'ClayCanvas',
  'Clay & Fabric': 'ClayFabric',
  'Canvas & Fabric': 'CanvasFabric',
};
for (const mat of DRAFT_MATERIAL_CARDS) {
  CARD_LOOKUP[DRAFT_MATERIAL_NAME_MAP[mat.name]] = mat;
}

// Action cards
const ACTION_NAME_MAP: Record<string, string> = {
  'Alum': 'Alum',
  'Cream of Tartar': 'CreamOfTartar',
  'Gum Arabic': 'GumArabic',
  'Potash': 'Potash',
  'Vinegar': 'Vinegar',
};
for (const act of ACTION_CARDS) {
  CARD_LOOKUP[ACTION_NAME_MAP[act.name]] = act;
}
CARD_LOOKUP['Chalk'] = CHALK_CARD;

// Build buyer lookup
const BUYER_LOOKUP: Record<string, BuyerCardData> = {};
const buyerVariantNames = [
  'Textiles2Vermilion', 'Textiles2Amber', 'Textiles2Chartreuse',
  'Textiles2Teal', 'Textiles2Indigo', 'Textiles2Magenta',
  'Textiles2OrangeRed', 'Textiles2OrangeYellow', 'Textiles2OrangeBlue',
  'Textiles2GreenRed', 'Textiles2GreenYellow', 'Textiles2GreenBlue',
  'Textiles2PurpleRed', 'Textiles2PurpleYellow', 'Textiles2PurpleBlue',
  'Ceramics3VermilionRed', 'Ceramics3VermilionYellow', 'Ceramics3VermilionBlue',
  'Ceramics3AmberRed', 'Ceramics3AmberYellow', 'Ceramics3AmberBlue',
  'Ceramics3ChartreuseRed', 'Ceramics3ChartreuseYellow', 'Ceramics3ChartreuseBlue',
  'Ceramics3TealRed', 'Ceramics3TealYellow', 'Ceramics3TealBlue',
  'Ceramics3IndigoRed', 'Ceramics3IndigoYellow', 'Ceramics3IndigoBlue',
  'Ceramics3MagentaRed', 'Ceramics3MagentaYellow', 'Ceramics3MagentaBlue',
  'Paintings4VermilionOrange', 'Paintings4VermilionGreen', 'Paintings4VermilionPurple',
  'Paintings4AmberOrange', 'Paintings4AmberGreen', 'Paintings4AmberPurple',
  'Paintings4ChartreuseOrange', 'Paintings4ChartreuseGreen', 'Paintings4ChartreusePurple',
  'Paintings4TealOrange', 'Paintings4TealGreen', 'Paintings4TealPurple',
  'Paintings4IndigoOrange', 'Paintings4IndigoGreen', 'Paintings4IndigoPurple',
  'Paintings4MagentaOrange', 'Paintings4MagentaGreen', 'Paintings4MagentaPurple',
];
for (let i = 0; i < BUYER_CARDS.length; i++) {
  BUYER_LOOKUP[buyerVariantNames[i]] = BUYER_CARDS[i];
}

export function getCardData(card: Card): AnyCardData {
  return CARD_LOOKUP[card];
}

export function getBuyerData(buyer: BuyerCard): BuyerCardData {
  return BUYER_LOOKUP[buyer];
}

// Unified lookup for any card string (Card or BuyerCard)
export function getAnyCardData(card: string): AnyCardData {
  return CARD_LOOKUP[card] ?? BUYER_LOOKUP[card];
}

export function getCardPips(card: string): Color[] {
  const data = getAnyCardData(card);
  if (!data) return [];
  switch (data.kind) {
    case 'dye': return data.colors;
    case 'basicDye': return [data.color];
    case 'material': return data.colorPip ? [data.colorPip] : [];
    case 'action': return [];
    case 'buyer': return [];
  }
}

export const DRAFT_COPY_COUNTS: Record<string, number> = {};
for (const card of DYE_CARDS) {
  DRAFT_COPY_COUNTS[card.name] = 4;
}
for (const card of DRAFT_MATERIAL_CARDS) {
  DRAFT_COPY_COUNTS[card.name] = 1;
}
for (const card of ACTION_CARDS) {
  DRAFT_COPY_COUNTS[card.name] = 3;
}

export function getDraftCopies(name: string): number {
  return DRAFT_COPY_COUNTS[name] ?? 1;
}

export interface CardCategory {
  label: string;
  cardNames: string[];
  totalCopies: number;
}

export const DRAFT_CARD_CATEGORIES: CardCategory[] = [
  (() => {
    const cardNames = DYE_CARDS.filter(c => c.ability.type === 'sell').map(c => c.name);
    return { label: 'Primary Dyes', cardNames, totalCopies: cardNames.length * 4 };
  })(),
  (() => {
    const cardNames = DYE_CARDS.filter(c => c.ability.type === 'workshop').map(c => c.name);
    return { label: 'Secondary Dyes', cardNames, totalCopies: cardNames.length * 4 };
  })(),
  (() => {
    const cardNames = DYE_CARDS.filter(c => c.ability.type === 'mixColors').map(c => c.name);
    return { label: 'Tertiary Dyes', cardNames, totalCopies: cardNames.length * 4 };
  })(),
  (() => {
    const cardNames = ACTION_CARDS.map(c => c.name);
    return { label: 'Action Cards', cardNames, totalCopies: cardNames.length * 3 };
  })(),
  (() => {
    const cardNames = DRAFT_MATERIAL_CARDS
      .filter(c => c.materialTypes.length === 2 && c.materialTypes[0] === c.materialTypes[1] && !c.colorPip)
      .map(c => c.name);
    return { label: 'Double Materials', cardNames, totalCopies: cardNames.length * 1 };
  })(),
  (() => {
    const cardNames = DRAFT_MATERIAL_CARDS
      .filter(c => c.colorPip !== undefined)
      .map(c => c.name);
    return { label: 'Material + Color', cardNames, totalCopies: cardNames.length * 1 };
  })(),
  (() => {
    const cardNames = DRAFT_MATERIAL_CARDS
      .filter(c => c.materialTypes.length === 2 && c.materialTypes[0] !== c.materialTypes[1] && !c.colorPip)
      .map(c => c.name);
    return { label: 'Dual Materials', cardNames, totalCopies: cardNames.length * 1 };
  })(),
];

export function getStarterCardCategories(numPlayers: number): CardCategory[] {
  const starterDyeNames = BASIC_DYE_CARDS.map(c => c.name);
  const starterMaterialNames = MATERIAL_CARDS.map(c => c.name);
  return [
    { label: 'Starter Dyes', cardNames: starterDyeNames, totalCopies: starterDyeNames.length * numPlayers },
    { label: 'Starter Materials', cardNames: starterMaterialNames, totalCopies: starterMaterialNames.length * numPlayers },
    { label: 'Chalk', cardNames: ['Chalk'], totalCopies: 1 * numPlayers },
  ];
}
