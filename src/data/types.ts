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

export interface DyeCard {
  kind: 'dye';
  name: string;
  colors: Color[];
  ability: Ability;
}

export interface BasicDyeCard {
  kind: 'basicDye';
  name: string;
  color: Color;
  ability: Ability;  // always { type: 'sell' }
}

export interface MaterialCard {
  kind: 'material';
  name: string;
  materialTypes: MaterialType[];
  colorPip?: Color;
  ability: Ability;
}

export interface ActionCard {
  kind: 'action';
  name: string;
  ability: Ability;
  workshopAbilities: Ability[];
}

export interface BuyerCard {
  kind: 'buyer';
  stars: number;
  requiredMaterial: MaterialType;
  colorCost: Color[];
}

export type AnyCard = DyeCard | BasicDyeCard | MaterialCard | ActionCard | BuyerCard;

export interface CardInstance<T extends AnyCard = AnyCard> {
  instanceId: number;
  card: T;
}

export interface PlayerState {
  name: string;
  deck: CardInstance[];
  discard: CardInstance[];
  drawnCards: CardInstance[];
  draftedCards: CardInstance[];
  colorWheel: Record<Color, number>;
  materials: Record<MaterialType, number>;
  completedBuyers: CardInstance<BuyerCard>[];
  ducats: number;
}

export interface DraftState {
  pickNumber: number;            // 0-3
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
  | { type: 'chooseTertiaryToLose' }
  | { type: 'chooseTertiaryToGain'; lostColor: Color };

export interface ActionState {
  currentPlayerIndex: number;
  abilityStack: Ability[];
  pendingChoice: PendingChoice | null;
}

export type GamePhase =
  | { type: 'draw' }
  | { type: 'draft'; draftState: DraftState }
  | { type: 'action'; actionState: ActionState }
  | { type: 'gameOver' };

export interface GameState {
  players: PlayerState[];
  draftDeck: CardInstance[];
  destroyedPile: CardInstance[];
  buyerDeck: CardInstance<BuyerCard>[];
  buyerDisplay: CardInstance<BuyerCard>[];
  phase: GamePhase;
  round: number;
  aiPlayers: boolean[];
}
