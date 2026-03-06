import type { Card, BuyerCard } from './types';

const CARD_ART_BASE_PATH = '/cards/';

// Special-case overrides where PascalCase->kebab-case doesn't match the art filename
const CARD_FILENAME_OVERRIDES: Partial<Record<Card, string>> = {
  VermilionDye: 'vermilion',
  IndigoDye: 'indigo',
  StarterCeramics: 'ceramics',
  StarterPaintings: 'paintings',
  StarterTextiles: 'textiles',
  FineCeramics: 'fine-ceramics',
  FinePaintings: 'fine-paintings',
  FineTextiles: 'fine-textiles',
  TerraCotta: 'terra-cotta',
  OchreWare: 'ochre-ware',
  CobaltWare: 'cobalt-ware',
  CinnabarCanvas: 'cinnabar-canvas',
  OrpimentCanvas: 'orpiment-canvas',
  UltramarineCanvas: 'ultramarine-canvas',
  AlizarinFabric: 'alizarin-fabric',
  FusticFabric: 'fustic-fabric',
  PastelFabric: 'pastel-fabric',
  ClayCanvas: 'clay-canvas',
  ClayFabric: 'clay-fabric',
  CanvasFabric: 'canvas-fabric',
  CreamOfTartar: 'cream-of-tartar',
  GumArabic: 'gum-arabic',
  DyersGreenweed: 'dyers-greenweed',
  PersianBerries: 'persian-berries',
  BasicRed: 'basic-red',
  BasicBlue: 'basic-blue',
  BasicYellow: 'basic-yellow',
};

function pascalToKebab(s: string): string {
  return s.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase();
}

export function getCardArtFilename(card: Card): string {
  const override = CARD_FILENAME_OVERRIDES[card];
  if (override) return override + '.png';
  return pascalToKebab(card) + '.png';
}

// Buyer card mapping: "Textiles2Vermilion" -> "vermilion-textile.png"
// "Ceramics3AmberRed" -> "amber-red-ceramic.png"
// "Paintings4TealOrange" -> "teal-orange-painting.png"
// "Textiles2RedRedRed" -> "red-red-red-textile.png"
const MATERIAL_SINGULAR: Record<string, string> = {
  Textiles: 'textile',
  Ceramics: 'ceramic',
  Paintings: 'painting',
};

export function getBuyerArtFilename(buyer: BuyerCard): string {
  const match = buyer.match(/^(Textiles|Ceramics|Paintings)(\d)(.+)$/);
  if (!match) return 'basic-red.png'; // fallback
  const [, material, , colorPart] = match;
  // Split PascalCase color names: "AmberRed" -> ["Amber", "Red"], "RedRedRed" -> ["Red", "Red", "Red"]
  const colors = colorPart.match(/[A-Z][a-z]*/g) ?? [];
  const colorKebab = colors.map(c => c.toLowerCase()).join('-');
  return colorKebab + '-' + MATERIAL_SINGULAR[material] + '.png';
}

export function getCardArtUrl(card: Card): string {
  return CARD_ART_BASE_PATH + getCardArtFilename(card);
}

export function getBuyerArtUrl(buyer: BuyerCard): string {
  return CARD_ART_BASE_PATH + getBuyerArtFilename(buyer);
}
