import type { CardInstance, SellCardInstance, Color, MaterialType, Ability } from '../data/types';
import type { Choice } from '../data/types';

export interface SanitizedPlayerState {
  deckCount: number;
  discardCount: number;
  // Private to owner: populated only when this entry belongs to the receiving player.
  // Opponents receive empty arrays; use *Count fields instead.
  deck: CardInstance[];
  discard: CardInstance[];
  // Public: visible to all players (face-up piles in the tableau).
  workshoppedCards: CardInstance[];
  workshopCards: CardInstance[];
  draftedCards: CardInstance[];
  colorWheel: Record<Color, number>;
  materials: Record<MaterialType, number>;
  completedSellCards: SellCardInstance[];
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
  phase: SanitizedGamePhase;
  round: number;
  aiPlayers: boolean[];
  myPlayerIndex: number;
  logEntries: string[];
  // Host's Date.now() at game start, so all clients share a consistent timer.
  gameStartTime?: number;
  // If true, the guest should replace its local log rather than append logEntries.
  resync?: boolean;
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
  | { type: 'gameStarted'; state: SanitizedGameState; rejoinToken: string }
  | { type: 'stateUpdate'; state: SanitizedGameState }
  | { type: 'gameOver'; state: SanitizedGameState }
  | { type: 'error'; message: string; context?: 'draftPick' | 'action' | 'join' }
  | { type: 'hostLeft'; reason: 'intentional' | 'crashed' }
  | { type: 'playerDisconnected'; playerIndex: number; playerName: string }
  | { type: 'playerReconnected'; playerIndex: number; playerName: string }
  | { type: 'playerReplacedByAI'; playerIndex: number; playerName: string }
  | { type: 'pong'; t: number };

export type GuestMessage =
  | { type: 'joinRequest'; name: string }
  | { type: 'rejoinRequest'; name: string; rejoinToken: string }
  | { type: 'action'; choice: Choice }
  | { type: 'ping'; t: number };
