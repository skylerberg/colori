import type { CardInstance, BuyerCard, Color, MaterialType, PendingChoice, Ability } from '../data/types';
import type { ColoriChoice } from '../data/types';

export interface SanitizedPlayerState {
  name: string;
  deckCount: number;
  discardCount: number;
  workshopCards: CardInstance[];
  draftedCards: CardInstance[];
  colorWheel: Record<Color, number>;
  materials: Record<MaterialType, number>;
  completedBuyers: CardInstance<BuyerCard>[];
  ducats: number;
}

export interface SanitizedDraftState {
  pickNumber: number;
  currentPlayerIndex: number;
  hands: CardInstance[][];
  direction: 1 | -1;
  waitingForPass: boolean;
}

export interface SanitizedActionState {
  currentPlayerIndex: number;
  abilityStack: Ability[];
  pendingChoice: PendingChoice | null;
}

export type SanitizedGamePhase =
  | { type: 'draw' }
  | { type: 'draft'; draftState: SanitizedDraftState }
  | { type: 'action'; actionState: SanitizedActionState }
  | { type: 'gameOver' };

export interface SanitizedGameState {
  players: SanitizedPlayerState[];
  draftDeckCount: number;
  destroyedPile: CardInstance[];
  buyerDeckCount: number;
  buyerDisplay: CardInstance<BuyerCard>[];
  phase: SanitizedGamePhase;
  round: number;
  aiPlayers: boolean[];
  myPlayerIndex: number;
  logEntries: string[];
}

export interface LobbyPlayer {
  peerId: string;
  name: string;
  playerIndex: number;
  isHost: boolean;
  connected: boolean;
}

export type HostMessage =
  | { type: 'lobbyUpdate'; players: LobbyPlayer[]; playerCount: number }
  | { type: 'gameStarted'; state: SanitizedGameState }
  | { type: 'stateUpdate'; state: SanitizedGameState }
  | { type: 'gameOver'; state: SanitizedGameState }
  | { type: 'error'; message: string }
  | { type: 'playerDisconnected'; playerIndex: number; playerName: string }
  | { type: 'playerReconnected'; playerIndex: number; playerName: string };

export type GuestMessage =
  | { type: 'joinRequest'; name: string }
  | { type: 'rejoinRequest'; name: string }
  | { type: 'action'; choice: ColoriChoice };
