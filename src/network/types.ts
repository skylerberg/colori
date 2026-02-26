import type { CardInstance, BuyerInstance, Color, MaterialType, PendingChoice, Ability } from '../data/types';
import type { ColoriChoice } from '../data/types';

export interface SanitizedPlayerState {
  deckCount: number;
  discardCount: number;
  usedCardsCount: number;
  workshopCards: CardInstance[];
  draftedCards: CardInstance[];
  colorWheel: Record<Color, number>;
  materials: Record<MaterialType, number>;
  completedBuyers: BuyerInstance[];
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

export interface SanitizedCleanupState {
  currentPlayerIndex: number;
}

export type SanitizedGamePhase =
  | { type: 'draw' }
  | { type: 'draft'; draftState: SanitizedDraftState }
  | { type: 'action'; actionState: SanitizedActionState }
  | { type: 'cleanup'; cleanupState: SanitizedCleanupState }
  | { type: 'gameOver' };

export interface SanitizedGameState {
  playerNames: string[];
  players: SanitizedPlayerState[];
  draftDeckCount: number;
  destroyedPile: CardInstance[];
  buyerDeckCount: number;
  buyerDisplay: BuyerInstance[];
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
