export type Color = 'Red' | 'Vermilion' | 'Orange' | 'Amber' | 'Yellow' |
  'Chartreuse' | 'Green' | 'Teal' | 'Blue' | 'Indigo' | 'Purple' | 'Magenta';

export type MaterialType = 'Textiles' | 'Ceramics' | 'Paintings';

export const ALL_MATERIAL_TYPES: MaterialType[] = ['Textiles', 'Ceramics', 'Paintings'];

export type Ability =
  | { type: 'workshop'; count: number }
  | { type: 'drawCards'; count: number }
  | { type: 'mixColors'; count: number }
  | { type: 'destroyCards'; count: number }
  | { type: 'makeGarment' }
  | { type: 'gainDucats'; count: number };

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
  ability: Ability;  // always { type: 'makeGarment' }
}

export interface MaterialCard {
  kind: 'material';
  name: string;
  materialType: MaterialType;
  ability: Ability;
}

export interface ActionCard {
  kind: 'action';
  name: string;
  ability: Ability;
  workshopAbilities: Ability[];
}

export interface GarmentCard {
  kind: 'garment';
  stars: number;
  requiredMaterial: MaterialType;
  colorCost: Color[];
}

export type AnyCard = DyeCard | BasicDyeCard | MaterialCard | ActionCard | GarmentCard;

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
  completedGarments: CardInstance<GarmentCard>[];
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
  | { type: 'chooseGarment' };

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
  garmentDeck: CardInstance<GarmentCard>[];
  garmentDisplay: CardInstance<GarmentCard>[];
  phase: GamePhase;
  round: number;
  aiPlayers: boolean[];
}
