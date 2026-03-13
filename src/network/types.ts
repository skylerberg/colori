import type { CardInstance, SellCardInstance, GlassInstance, Color, MaterialType, Ability, Expansions } from '../data/types';
import type { Choice } from '../data/types';

export interface SanitizedPlayerState {
  deckCount: number;
  discardCount: number;
  workshoppedCardsCount: number;
  workshopCards: CardInstance[];
  draftedCards: CardInstance[];
  colorWheel: Record<Color, number>;
  materials: Record<MaterialType, number>;
  completedSellCards: SellCardInstance[];
  completedGlass: GlassInstance[];
  ducats: number;
}

export interface SanitizedDraftState {
  pickNumber: number;
  currentPlayerIndex: number;
  hands: CardInstance[][];
}

export interface SanitizedActionState {
  currentPlayerIndex: number;
  abilityStack: Ability[];
  usedGlass: number;
}

export type SanitizedGamePhase =
  | { type: 'draw' }
  | { type: 'draft'; draftState: SanitizedDraftState }
  | { type: 'action'; actionState: SanitizedActionState }
  | { type: 'gameOver' };

export interface SanitizedGameState {
  playerNames: string[];
  players: SanitizedPlayerState[];
  draftDeckCount: number;
  destroyedPile: CardInstance[];
  sellCardDeckCount: number;
  sellCardDisplay: SellCardInstance[];
  glassDeckCount: number;
  glassDisplay: GlassInstance[];
  expansions: Expansions;
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
  isConnected: boolean;
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
  | { type: 'action'; choice: Choice };
