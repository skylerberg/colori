import type { NetworkManager } from './networkManager';
import type { SanitizedGameState, LobbyPlayer, HostMessage } from './types';
import type { ColoriChoice } from '../data/types';

export class GuestController {
  private network: NetworkManager;
  private gameLog: string[] = [];
  private pendingJoinName: string | null = null;
  private pendingRejoinName: string | null = null;

  onLobbyUpdated: ((players: LobbyPlayer[], playerCount: number) => void) | null = null;
  onGameStarted: ((state: SanitizedGameState) => void) | null = null;
  onStateUpdated: ((state: SanitizedGameState) => void) | null = null;
  onGameOver: ((state: SanitizedGameState) => void) | null = null;
  onError: ((message: string) => void) | null = null;
  onHostDisconnected: (() => void) | null = null;
  onPlayerDisconnected: ((playerIndex: number, playerName: string) => void) | null = null;
  onPlayerReconnected: ((playerIndex: number, playerName: string) => void) | null = null;

  constructor(network: NetworkManager) {
    this.network = network;

    this.network.onHostMessage = (msg) => this.handleHostMessage(msg);
    this.network.onPeerJoin = () => {
      if (this.pendingJoinName) {
        this.network.sendToHost({ type: 'joinRequest', name: this.pendingJoinName });
        this.pendingJoinName = null;
      }
      if (this.pendingRejoinName) {
        this.network.sendToHost({ type: 'rejoinRequest', name: this.pendingRejoinName });
        this.pendingRejoinName = null;
      }
    };
    this.network.onPeerLeave = () => {
      if (this.network.peers.size === 0) {
        this.onHostDisconnected?.();
      }
    };
  }

  join(name: string) {
    if (this.network.peers.size > 0) {
      this.network.sendToHost({ type: 'joinRequest', name });
    } else {
      this.pendingJoinName = name;
    }
  }

  rejoin(name: string) {
    if (this.network.peers.size > 0) {
      this.network.sendToHost({ type: 'rejoinRequest', name });
    } else {
      this.pendingRejoinName = name;
    }
  }

  sendAction(choice: ColoriChoice) {
    this.network.sendToHost({ type: 'action', choice });
  }

  getGameLog(): string[] {
    return this.gameLog;
  }

  private handleHostMessage(msg: HostMessage) {
    switch (msg.type) {
      case 'lobbyUpdate':
        this.onLobbyUpdated?.(msg.players, msg.playerCount);
        break;
      case 'gameStarted':
        this.gameLog = [...msg.state.logEntries];
        this.onGameStarted?.(msg.state);
        break;
      case 'stateUpdate':
        this.gameLog.push(...msg.state.logEntries);
        this.onStateUpdated?.(msg.state);
        break;
      case 'gameOver':
        this.gameLog.push(...msg.state.logEntries);
        this.onGameOver?.(msg.state);
        break;
      case 'error':
        this.onError?.(msg.message);
        break;
      case 'playerDisconnected':
        this.onPlayerDisconnected?.(msg.playerIndex, msg.playerName);
        break;
      case 'playerReconnected':
        this.onPlayerReconnected?.(msg.playerIndex, msg.playerName);
        break;
    }
  }
}
