export type Color = 'Red' | 'Vermilion' | 'Orange' | 'Amber' | 'Yellow' |
  'Chartreuse' | 'Green' | 'Teal' | 'Blue' | 'Indigo' | 'Purple' | 'Magenta';

export type FabricType = 'Wool' | 'Silk' | 'Linen' | 'Cotton';

export type Ability =
  | { type: 'makeMaterials'; count: number }
  | { type: 'drawCards'; count: number }
  | { type: 'mixColors'; count: number }
  | { type: 'destroyCards'; count: number }
  | { type: 'makeGarment' };

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
  ability: Ability;  // always { type: 'makeMaterials', count: 2 }
}

export interface FabricCard {
  kind: 'fabric';
  name: string;
  fabricType: FabricType;
  ability: Ability;
}

export interface GarmentCard {
  kind: 'garment';
  name: string;
  stars: number;
  requiredFabric: FabricType;
  colorCost: Color[];
}

export type AnyCard = DyeCard | BasicDyeCard | FabricCard | GarmentCard;

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
  fabrics: Record<FabricType, number>;
  completedGarments: CardInstance<GarmentCard>[];
}

export interface DraftState {
  pickNumber: number;            // 0-3
  currentPlayerIndex: number;
  hands: CardInstance[][];
  direction: 1 | -1;
  waitingForPass: boolean;
}

export type PendingChoice =
  | { type: 'chooseCardsForMaterials'; count: number }
  | { type: 'chooseCardsToDestroy'; count: number }
  | { type: 'chooseMix'; remaining: number }
  | { type: 'chooseGarment' };

export interface ActionState {
  currentPlayerIndex: number;
  abilityQueue: Ability[];
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
  garmentDeck: CardInstance<GarmentCard>[];
  garmentDisplay: CardInstance<GarmentCard>[];
  phase: GamePhase;
  round: number;  // 1-8
  aiPlayers: boolean[];
}
