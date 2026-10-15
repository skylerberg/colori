export type Color = 'Red' | 'Vermilion' | 'Orange' | 'Amber' | 'Yellow' |
  'Chartreuse' | 'Green' | 'Teal' | 'Blue' | 'Ultramarine' | 'Purple' | 'Fuchsia';

export type MaterialType = 'Textiles' | 'Ceramics' | 'Paintings';

export const ALL_MATERIAL_TYPES: MaterialType[] = ['Textiles', 'Ceramics', 'Paintings'];

export type Ability =
  | { type: 'workshop'; count: number }
  | { type: 'drawCards'; count: number }
  | { type: 'mixColors'; count: number }
  | { type: 'moveToDraftPool' }
  | { type: 'sell' }
  | { type: 'gainDucats'; count: number }
  | { type: 'gainSecondary' }
  | { type: 'gainPrimary' }
  | { type: 'gainMaterial' }
  | { type: 'gainWildPrimaries'; count: number }
  | { type: 'gainBoats'; count: number };

// Card variant name strings from Rust
export type Card = 'BasicRed' | 'BasicYellow' | 'BasicBlue'
  | 'Realgar' | 'Kermes' | 'Litharge' | 'Malachite' | 'Woad' | 'Logwood'
  | 'DragonsBlood' | 'Orpiment' | 'GreenEarth' | 'Verdigris' | 'Brazilwood' | 'Indigo'
  | 'Cinnabar' | 'Saffron' | 'CopperResinate' | 'Azurite' | 'LapisLazuli' | 'Cochineal'
  | 'StarterCeramics' | 'StarterCanvas' | 'StarterFabric'
  | 'CeramicsRed' | 'CeramicsYellow' | 'CeramicsBlue'
  | 'CanvasRed' | 'CanvasYellow' | 'CanvasBlue'
  | 'FabricOrange' | 'FabricGreen' | 'FabricPurple'
  | 'CanvasCeramics' | 'CeramicsFabric' | 'CanvasFabric'
  | 'Alum' | 'GumArabic' | 'LinseedOil' | 'Warehouse' | 'Gondola' | 'Chalk';

export type SellCard =
  | 'ShipwrightPurpleGreen' | 'ShipwrightOrangeGreen' | 'ShipwrightPurpleOrange'
  | 'GlassmakerVermilionBlue' | 'GlassmakerAmberBlue' | 'GlassmakerChartreuseRed'
  | 'GlassmakerTealRed' | 'GlassmakerUltramarineYellow' | 'GlassmakerFuchsiaYellow'
  | 'DyerRedRed' | 'DyerRedYellow' | 'DyerRedBlue'
  | 'DyerYellowYellow' | 'DyerBlueYellow' | 'DyerBlueBlue'
  | 'DyerOrange' | 'DyerGreen' | 'DyerPurple'
  | 'IlluminatorVermilion' | 'IlluminatorAmber' | 'IlluminatorChartreuse'
  | 'IlluminatorTeal' | 'IlluminatorUltramarine' | 'IlluminatorFuchsia'
  | 'PotterVermilion' | 'PotterAmber' | 'PotterChartreuse'
  | 'PotterTeal' | 'PotterUltramarine' | 'PotterFuchsia'
  | 'TilemakerVermilion' | 'TilemakerAmber' | 'TilemakerChartreuse'
  | 'TilemakerTeal' | 'TilemakerUltramarine' | 'TilemakerFuchsia'
  | 'PainterVermilionOrange' | 'PainterVermilionGreen' | 'PainterVermilionPurple'
  | 'PainterAmberOrange' | 'PainterAmberGreen' | 'PainterAmberPurple'
  | 'PainterChartreuseOrange' | 'PainterChartreuseGreen' | 'PainterChartreusePurple'
  | 'PainterTealOrange' | 'PainterTealGreen' | 'PainterTealPurple'
  | 'PainterUltramarineOrange' | 'PainterUltramarineGreen' | 'PainterUltramarinePurple'
  | 'PainterFuchsiaOrange' | 'PainterFuchsiaGreen' | 'PainterFuchsiaPurple'
  | 'MasterPainterVermilionTeal' | 'MasterPainterVermilionFuchsia' | 'MasterPainterAmberUltramarine'
  | 'MasterPainterChartreuseAmber' | 'MasterPainterChartreuseFuchsia' | 'MasterPainterUltramarineTeal';

// Card data interfaces (for the lookup maps)
export interface DyeCardData {
  kind: 'dye';
  name: string;
  colors: Color[];
  // Resolved in order when the card is destroyed.
  destroyAbilities: Ability[];
}

export interface BasicDyeCardData {
  kind: 'basicDye';
  name: string;
  color: Color;
  // Resolved in order when the card is destroyed.
  destroyAbilities: Ability[];
}

export interface MaterialCardData {
  kind: 'material';
  name: string;
  materialTypes: MaterialType[];
  color?: Color;
  colorCount?: number;
  // Resolved in order when the card is destroyed.
  destroyAbilities: Ability[];
}

export interface ActionCardData {
  kind: 'action';
  name: string;
  // Resolved in order when the card is destroyed.
  destroyAbilities: Ability[];
  workshopAbilities: Ability[];
}

export interface SellCardData {
  kind: 'sellCard';
  // The buyer's trade, e.g. "Potter". Several sell cards share one.
  name: string;
  ducats: number;
  // One entry per unit, so a Tilemaker lists Ceramics twice.
  requiredMaterials: MaterialType[];
  // Boats the sale takes outright, before any standing in for a material.
  requiredBoats: number;
  colorCost: Color[];
}

export type AnyCardData = DyeCardData | BasicDyeCardData | MaterialCardData | ActionCardData | SellCardData;

export interface CardInstance {
  instanceId: number;
  card: Card;
}

export interface SellCardInstance {
  instanceId: number;
  card: SellCard;
}

export interface PlayerState {
  // Ordered front-first: the deck is a queue of independently shuffled piles,
  // because a round's leftover workshop goes to the bottom as one. See
  // `colori_core::deck::Deck`.
  deck: CardInstance[][];
  // Sell cards claimed for this player alone. Only these can be sold to; the
  // shared display is claimed from during the buyers phase.
  buyers: SellCardInstance[];
  workshoppedCards: CardInstance[];
  workshopCards: CardInstance[];
  draftedCards: CardInstance[];
  colorWheel: Record<Color, number>;
  materials: Record<MaterialType, number>;
  completedSellCards: SellCardInstance[];
  ducats: number;
  // Wild primaries and wild materials. Each is worth half a point unspent.
  wildPrimaries: number;
  boats: number;
}

export interface DraftState {
  pickNumber: number;
  currentPlayerIndex: number;
  hands: CardInstance[][];
}

export interface ActionState {
  currentPlayerIndex: number;
  abilityStack: Ability[];
}

export type DucatPurchase = 'workshop' | 'mixColors' | 'sell';

export interface BuyersState {
  currentPlayerIndex: number;
}

export type GamePhase =
  | { type: 'draw' }
  | { type: 'buyers'; buyersState: BuyersState }
  | { type: 'draft'; draftState: DraftState }
  | { type: 'action'; actionState: ActionState }
  | { type: 'gameOver' };

export interface GameState {
  playerNames: string[];
  players: PlayerState[];
  draftDeck: CardInstance[];
  destroyedPile: CardInstance[];
  sellCardDeck: SellCardInstance[];
  sellCardDisplay: SellCardInstance[];
  phase: GamePhase;
  round: number;
  maxRounds: number;
  aiPlayers: boolean[];
}

export type Choice =
  | { type: 'draftPick'; card: Card }
  | { type: 'destroyDraftedCard'; card: Card }
  | { type: 'endTurn' }
  | { type: 'workshop'; cardTypes: Card[] }
  | { type: 'skipWorkshop' }
  | { type: 'moveToDraftPool'; card: Card | null }
  | { type: 'selectSellCard'; sellCard: SellCard }
  | { type: 'gainSecondary'; color: Color }
  | { type: 'gainPrimary'; color: Color }
  | { type: 'gainMaterial'; material: MaterialType }
  | { type: 'mixAll'; mixes: [Color, Color][] }
  | { type: 'destroyAndMix'; card: Card; mixes: [Color, Color][] }
  | { type: 'destroyAndSell'; card: Card; sellCard: SellCard }
  | { type: 'destroyAndWorkshop'; card: Card; workshopCards: Card[] }
  | { type: 'destroyAndMoveToDraftPool'; card: Card; target: Card | null }
  | { type: 'spendDucat'; purchase: DucatPurchase }
  | { type: 'takeBuyer'; sellCard: SellCard }
  | { type: 'drawBuyer' };

// ── Draw Event Types ──

export type DrawEvent =
  | { type: 'playerDeckDraw'; playerIndex: number; cards: CardInstance[] }
  | { type: 'draftDeal'; playerIndex: number; cards: CardInstance[] }
  | { type: 'sellCardReveal'; sellCard: SellCardInstance }
  | { type: 'buyerDraw'; playerIndex: number; sellCard: SellCardInstance };

// ── Game Log Types ──

export interface StructuredLogEntry {
  seq: number;
  timestamp: number;
  round: number;
  phase: string;
  playerIndex: number;
  choice: Choice;
  draws?: DrawEvent[];
}

export interface PlayerVariant {
  name?: string;
  iterations: number;
  explorationConstant?: number;
  maxRolloutSteps?: number;
}

export interface StructuredGameLog {
  version: 1;
  gameStartedAt: string;
  gameEndedAt: string | null;
  playerNames: string[];
  aiPlayers: boolean[];
  initialState: GameState;
  finalScores: { name: string; score: number; completedSellCards?: number; colorWheelTotal?: number }[] | null;
  finalPlayerStats: FinalPlayerStats[] | null;
  entries: StructuredLogEntry[];
  initialDraws?: DrawEvent[];
  durationMs?: number;
  iterations?: number;
  playerVariants?: PlayerVariant[];
  note?: string;
}

export interface FinalPlayerStats {
  name: string;
  deckSize: number;
  completedSellCards: SellCardInstance[];
  ducats: number;
  wildPrimaries?: number;
  boats?: number;
  colorWheel: Record<Color, number>;
  materials: Record<MaterialType, number>;
}
