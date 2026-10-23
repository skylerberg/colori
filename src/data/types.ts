export type Color = 'Red' | 'Vermilion' | 'Orange' | 'Amber' | 'Yellow' |
  'Chartreuse' | 'Green' | 'Teal' | 'Blue' | 'Ultramarine' | 'Purple' | 'Fuchsia';

export type MaterialType = 'Textiles' | 'Ceramics' | 'Paintings';


export type Ability =
  | { type: 'workshop'; count: number }
  | { type: 'mixColors'; count: number }
  | { type: 'liftCard' }
  | { type: 'sell' }
  | { type: 'gainGoldCoins'; count: number }
  | { type: 'gainSecondary' }
  | { type: 'gainBowls'; count: number }
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
  | 'Alum' | 'GumArabic' | 'LinseedOil' | 'Warehouse' | 'Gondola' | 'Chalk'
  | 'Primaries';

export type Buyer =
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
export interface PigmentCardData {
  kind: 'pigment';
  name: string;
  colors: Color[];
  // Resolved in order, then the card is trashed.
  topActions: Ability[];
}

export interface StarterPigmentCardData {
  kind: 'starterPigment';
  name: string;
  color: Color;
  // Resolved in order, then the card is trashed.
  topActions: Ability[];
}

export interface MaterialCardData {
  kind: 'material';
  name: string;
  materialTypes: MaterialType[];
  color?: Color;
  colorCount?: number;
  // Resolved in order when the card is trashed.
  topActions: Ability[];
}

export interface SpecialCardData {
  kind: 'special';
  name: string;
  // Resolved in order, then the card is trashed.
  topActions: Ability[];
  workshopAbilities: Ability[];
}

export interface BuyerData {
  kind: 'buyer';
  // The buyer's trade, e.g. "Potter". Several buyers share one.
  name: string;
  // Printed on the seal; scored once the sale is made.
  points: number;
  // One entry per unit, so a Tilemaker lists Ceramics twice.
  requiredMaterials: MaterialType[];
  // Boats the sale takes outright, before any standing in for a material.
  requiredBoats: number;
  colorCost: Color[];
}

export type AnyCardData = PigmentCardData | StarterPigmentCardData | MaterialCardData | SpecialCardData | BuyerData;

export interface CardInstance {
  instanceId: number;
  card: Card;
}

export interface BuyerInstance {
  instanceId: number;
  card: Buyer;
}

export interface PlayerState {
  // Ordered front-first: the deck is a queue of piles — the cards kept on top
  // of it, then independently shuffled piles, because a round's leftover
  // workshop goes to the bottom as one. See `colori_core::deck::Deck`.
  deck: CardInstance[][];
  // Buyers claimed for this player alone. Only these can be sold to; the
  // shared display is claimed from during the buyers phase.
  buyers: BuyerInstance[];
  usedCards: CardInstance[];
  workshopCards: CardInstance[];
  topCards: CardInstance[];
  colorWheel: Record<Color, number>;
  // Stored materials, on the materials wheel.
  materialsWheel: Record<MaterialType, number>;
  completedOrders: BuyerInstance[];
  goldCoins: number;
  // Bowls (wild primaries) and boats (wild materials); each is worth half a
  // point unspent.
  bowls: number;
  boats: number;
}

export interface DraftState {
  pickNumber: number;
  currentPlayerIndex: number;
  hands: CardInstance[][];
}

export interface PlayerTurn {
  abilityStack: Ability[];
  ended: boolean;
}

// Everyone takes their action turn at once, each with their own abilities
// waiting to resolve.
export interface ActionState {
  turns: PlayerTurn[];
}

export type GoldCoinPurchase = 'workshop' | 'mixColors' | 'sell';

export interface BuyersState {
  currentPlayerIndex: number;
}

export type GamePhase =
  | { type: 'buyers'; buyersState: BuyersState }
  | { type: 'draft'; draftState: DraftState }
  | { type: 'action'; actionState: ActionState }
  | { type: 'gameOver' };

export interface GameState {
  playerNames: string[];
  players: PlayerState[];
  draftDeck: CardInstance[];
  trash: CardInstance[];
  buyerDeck: BuyerInstance[];
  buyerDisplay: BuyerInstance[];
  phase: GamePhase;
  round: number;
  maxRounds: number;
  botPlayers: boolean[];
}

export type Choice =
  | { type: 'draftPick'; card: Card }
  | { type: 'trashTopCard'; card: Card }
  | { type: 'keepCard'; card: Card }
  // `keepRemaining` decides what becomes of the top cards the turn leaves
  // undecided: kept on top of the deck, or trashed without using their top
  // actions. Old logs omit it, which means keeping.
  | { type: 'endTurn'; keepRemaining: boolean }
  | { type: 'workshop'; cardTypes: Card[] }
  | { type: 'skipWorkshop' }
  | { type: 'liftCard'; card: Card | null }
  | { type: 'sellToBuyer'; buyer: Buyer }
  | { type: 'gainSecondary'; color: Color }
  | { type: 'mixAll'; mixes: [Color, Color][] }
  | { type: 'trashAndMix'; card: Card; mixes: [Color, Color][] }
  | { type: 'trashAndSell'; card: Card; buyer: Buyer }
  | { type: 'trashAndWorkshop'; card: Card; workshopCards: Card[] }
  | { type: 'trashAndLift'; card: Card; target: Card | null }
  | { type: 'spendGoldCoin'; purchase: GoldCoinPurchase }
  | { type: 'claimBuyer'; buyer: Buyer }
  | { type: 'drawBuyer' };

// ── Draw Event Types ──

export type DrawEvent =
  | { type: 'playerDeckDraw'; playerIndex: number; cards: CardInstance[] }
  | { type: 'draftDeal'; playerIndex: number; cards: CardInstance[] }
  | { type: 'buyerReveal'; buyer: BuyerInstance }
  | { type: 'buyerDraw'; playerIndex: number; buyer: BuyerInstance };

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
  botSettings?: BotSettings;
}

/** The advanced bot settings from the setup screen; `{}` is the default bot. Mirrors `BotSettings` in colori-wasm. */
export interface BotSettings {
  /** Which trained parameters to play with. */
  params?: string;
  /** Rounds a rollout plays before the heuristic scores it; one that reaches the end of the game gets its real score. Unset keeps the trained lookahead. */
  rolloutLookahead?: number | null;
}

export interface StructuredGameLog {
  version: 1;
  gameStartedAt: string;
  gameEndedAt: string | null;
  playerNames: string[];
  botPlayers: boolean[];
  initialState: GameState;
  finalScores: { name: string; score: number; completedOrders?: number; colorWheelTotal?: number }[] | null;
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
  completedOrders: BuyerInstance[];
  goldCoins: number;
  bowls?: number;
  boats?: number;
  colorWheel: Record<Color, number>;
  materialsWheel: Record<MaterialType, number>;
}
