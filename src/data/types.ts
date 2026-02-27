export type Color = 'Red' | 'Vermilion' | 'Orange' | 'Amber' | 'Yellow' |
  'Chartreuse' | 'Green' | 'Teal' | 'Blue' | 'Indigo' | 'Purple' | 'Magenta';

export type MaterialType = 'Textiles' | 'Ceramics' | 'Paintings';

export const ALL_MATERIAL_TYPES: MaterialType[] = ['Textiles', 'Ceramics', 'Paintings'];

export type Ability =
  | { type: 'workshop'; count: number }
  | { type: 'drawCards'; count: number }
  | { type: 'mixColors'; count: number }
  | { type: 'destroyCards'; count: number }
  | { type: 'sell' }
  | { type: 'gainDucats'; count: number }
  | { type: 'gainSecondary' }
  | { type: 'gainPrimary' }
  | { type: 'changeTertiary' };

// Card variant name strings from Rust
export type Card = 'BasicRed' | 'BasicYellow' | 'BasicBlue'
  | 'Kermes' | 'Weld' | 'Woad'
  | 'Madder' | 'Turmeric' | 'DyersGreenweed' | 'Verdigris' | 'Orchil' | 'Logwood'
  | 'VermilionDye' | 'Saffron' | 'PersianBerries' | 'Azurite' | 'IndigoDye' | 'Cochineal'
  | 'StarterCeramics' | 'StarterPaintings' | 'StarterTextiles'
  | 'FineCeramics' | 'FinePaintings' | 'FineTextiles'
  | 'TerraCotta' | 'OchreWare' | 'CobaltWare'
  | 'CinnabarCanvas' | 'OrpimentCanvas' | 'UltramarineCanvas'
  | 'AlizarinFabric' | 'FusticFabric' | 'PastelFabric'
  | 'ClayCanvas' | 'ClayFabric' | 'CanvasFabric'
  | 'Alum' | 'CreamOfTartar' | 'GumArabic' | 'Potash' | 'Vinegar' | 'Chalk';

export type BuyerCard = 'Textiles2Vermilion' | 'Textiles2Amber' | 'Textiles2Chartreuse'
  | 'Textiles2Teal' | 'Textiles2Indigo' | 'Textiles2Magenta'
  | 'Textiles2OrangeRed' | 'Textiles2OrangeYellow' | 'Textiles2OrangeBlue'
  | 'Textiles2GreenRed' | 'Textiles2GreenYellow' | 'Textiles2GreenBlue'
  | 'Textiles2PurpleRed' | 'Textiles2PurpleYellow' | 'Textiles2PurpleBlue'
  | 'Textiles2RedRedRed' | 'Textiles2YellowYellowYellow' | 'Textiles2BlueBlueBlue'
  | 'Ceramics3VermilionRed' | 'Ceramics3VermilionYellow' | 'Ceramics3VermilionBlue'
  | 'Ceramics3AmberRed' | 'Ceramics3AmberYellow' | 'Ceramics3AmberBlue'
  | 'Ceramics3ChartreuseRed' | 'Ceramics3ChartreuseYellow' | 'Ceramics3ChartreuseBlue'
  | 'Ceramics3TealRed' | 'Ceramics3TealYellow' | 'Ceramics3TealBlue'
  | 'Ceramics3IndigoRed' | 'Ceramics3IndigoYellow' | 'Ceramics3IndigoBlue'
  | 'Ceramics3MagentaRed' | 'Ceramics3MagentaYellow' | 'Ceramics3MagentaBlue'
  | 'Paintings4VermilionOrange' | 'Paintings4VermilionGreen' | 'Paintings4VermilionPurple'
  | 'Paintings4AmberOrange' | 'Paintings4AmberGreen' | 'Paintings4AmberPurple'
  | 'Paintings4ChartreuseOrange' | 'Paintings4ChartreuseGreen' | 'Paintings4ChartreusePurple'
  | 'Paintings4TealOrange' | 'Paintings4TealGreen' | 'Paintings4TealPurple'
  | 'Paintings4IndigoOrange' | 'Paintings4IndigoGreen' | 'Paintings4IndigoPurple'
  | 'Paintings4MagentaOrange' | 'Paintings4MagentaGreen' | 'Paintings4MagentaPurple';

// Card data interfaces (for the lookup maps)
export interface DyeCardData {
  kind: 'dye';
  name: string;
  colors: Color[];
  ability: Ability;
}

export interface BasicDyeCardData {
  kind: 'basicDye';
  name: string;
  color: Color;
  ability: Ability;
}

export interface MaterialCardData {
  kind: 'material';
  name: string;
  materialTypes: MaterialType[];
  colorPip?: Color;
  ability: Ability;
}

export interface ActionCardData {
  kind: 'action';
  name: string;
  ability: Ability;
  workshopAbilities: Ability[];
}

export interface BuyerCardData {
  kind: 'buyer';
  stars: number;
  requiredMaterial: MaterialType;
  colorCost: Color[];
}

export type AnyCardData = DyeCardData | BasicDyeCardData | MaterialCardData | ActionCardData | BuyerCardData;

export interface CardInstance {
  instanceId: number;
  card: Card;
}

export interface BuyerInstance {
  instanceId: number;
  card: BuyerCard;
}

export interface PlayerState {
  deck: CardInstance[];
  discard: CardInstance[];
  usedCards: CardInstance[];
  workshopCards: CardInstance[];
  draftedCards: CardInstance[];
  colorWheel: Record<Color, number>;
  materials: Record<MaterialType, number>;
  completedBuyers: BuyerInstance[];
  ducats: number;
}

export interface DraftState {
  pickNumber: number;
  currentPlayerIndex: number;
  hands: CardInstance[][];
  direction: 1 | -1;
  waitingForPass: boolean;
}

export type PendingChoice =
  | { type: 'chooseCardsForWorkshop'; count: number }
  | { type: 'chooseCardsToDestroy'; count: number }
  | { type: 'chooseMix'; remaining: number }
  | { type: 'chooseBuyer' }
  | { type: 'chooseSecondaryColor' }
  | { type: 'choosePrimaryColor' }
  | { type: 'chooseTertiaryToLose' };

export interface ActionState {
  currentPlayerIndex: number;
  abilityStack: Ability[];
  pendingChoice: PendingChoice | null;
}

export interface CleanupState {
  currentPlayerIndex: number;
}

export type GamePhase =
  | { type: 'draw' }
  | { type: 'draft'; draftState: DraftState }
  | { type: 'action'; actionState: ActionState }
  | { type: 'cleanup'; cleanupState: CleanupState }
  | { type: 'gameOver' };

export interface GameState {
  playerNames: string[];
  players: PlayerState[];
  draftDeck: CardInstance[];
  destroyedPile: CardInstance[];
  buyerDeck: BuyerInstance[];
  buyerDisplay: BuyerInstance[];
  phase: GamePhase;
  round: number;
  aiPlayers: boolean[];
}

export type ColoriChoice =
  | { type: 'draftPick'; cardInstanceId: number }
  | { type: 'destroyDraftedCard'; cardInstanceId: number }
  | { type: 'endTurn' }
  | { type: 'workshop'; cardInstanceIds: number[] }
  | { type: 'skipWorkshop' }
  | { type: 'destroyDrawnCards'; cardInstanceIds: number[] }
  | { type: 'selectBuyer'; buyerInstanceId: number }
  | { type: 'gainSecondary'; color: Color }
  | { type: 'gainPrimary'; color: Color }
  | { type: 'mixAll'; mixes: [Color, Color][] }
  | { type: 'swapTertiary'; loseColor: Color; gainColor: Color }
  | { type: 'destroyAndMixAll'; cardInstanceId: number; mixes: [Color, Color][] }
  | { type: 'destroyAndSell'; cardInstanceId: number; buyerInstanceId: number }
  | { type: 'keepWorkshopCards'; cardInstanceIds: number[] };
