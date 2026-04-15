import type { NetworkManager } from './networkManager';
import type { SanitizedGameState, LobbyPlayer, HostMessage } from './types';
import type { Choice } from '../data/types';

const PING_INTERVAL_MS = 3_000;
const PING_STALE_MS = 8_000;

export class GuestController {
  private network: NetworkManager;
  private gameLog: string[] = [];
  private pendingJoinName: string | null = null;
  private pendingRejoin: { name: string; token: string } | null = null;
  private rejoinToken: string | null = null;
  private myName: string | null = null;
  private pingTimer: ReturnType<typeof setInterval> | null = null;
  private lastPongAt: number = 0;
  private latencyMs: number = 0;

  onLobbyUpdated: ((players: LobbyPlayer[], playerCount: number) => void) | null = null;
  onGameStarted: ((state: SanitizedGameState) => void) | null = null;
  onSanitizedStateChanged: ((state: SanitizedGameState) => void) | null = null;
  onGameOver: ((state: SanitizedGameState) => void) | null = null;
  onError: ((message: string, context?: string) => void) | null = null;
  onHostDisconnected: ((reason: 'intentional' | 'crashed') => void) | null = null;
  onPlayerDisconnected: ((playerIndex: number, playerName: string) => void) | null = null;
  onPlayerReconnected: ((playerIndex: number, playerName: string) => void) | null = null;
  onPlayerReplacedByAI: ((playerIndex: number, playerName: string) => void) | null = null;
  onLatencyChange: ((latencyMs: number, stalled: boolean) => void) | null = null;

  constructor(network: NetworkManager) {
    this.network = network;

    this.network.onHostMessage = (msg) => this.handleHostMessage(msg);
    this.network.onPeerJoin = () => {
      if (this.pendingRejoin) {
        this.network.sendToHost({ type: 'rejoinRequest', name: this.pendingRejoin.name, rejoinToken: this.pendingRejoin.token });
        this.myName = this.pendingRejoin.name;
        this.rejoinToken = this.pendingRejoin.token;
        this.pendingRejoin = null;
      } else if (this.pendingJoinName) {
        this.network.sendToHost({ type: 'joinRequest', name: this.pendingJoinName });
        this.myName = this.pendingJoinName;
        this.pendingJoinName = null;
      }
      this.startHeartbeat();
    };
    this.network.onPeerLeave = () => {
      if (this.network.peers.size === 0) {
        this.stopHeartbeat();
        this.onHostDisconnected?.('crashed');
      }
    };
  }

  join(name: string) {
    this.myName = name;
    if (this.network.peers.size > 0) {
      this.network.sendToHost({ type: 'joinRequest', name });
      this.startHeartbeat();
    } else {
      this.pendingJoinName = name;
    }
  }

  rejoin(name: string, token: string) {
    this.myName = name;
    this.rejoinToken = token;
    if (this.network.peers.size > 0) {
      this.network.sendToHost({ type: 'rejoinRequest', name, rejoinToken: token });
      this.startHeartbeat();
    } else {
      this.pendingRejoin = { name, token };
    }
  }

  sendAction(choice: Choice) {
    this.network.sendToHost({ type: 'action', choice });
  }

  getGameLog(): string[] {
    return this.gameLog;
  }

  getRejoinToken(): string | null {
    return this.rejoinToken;
  }

  getMyName(): string | null {
    return this.myName;
  }

  getLatencyMs(): number {
    return this.latencyMs;
  }

  leave() {
    this.stopHeartbeat();
  }

  private startHeartbeat() {
    if (this.pingTimer) return;
    this.lastPongAt = Date.now();
    this.pingTimer = setInterval(() => {
      const now = Date.now();
      if (this.network.peers.size === 0) return;
      this.network.sendToHost({ type: 'ping', t: now });
      if (this.lastPongAt > 0 && now - this.lastPongAt > PING_STALE_MS) {
        this.onLatencyChange?.(this.latencyMs, true);
      }
    }, PING_INTERVAL_MS);
  }

  private stopHeartbeat() {
    if (this.pingTimer) {
      clearInterval(this.pingTimer);
      this.pingTimer = null;
    }
  }

  private handleHostMessage(msg: HostMessage) {
    switch (msg.type) {
      case 'lobbyUpdate':
        this.onLobbyUpdated?.(msg.players, msg.playerCount);
        break;
      case 'gameStarted':
        this.rejoinToken = msg.rejoinToken;
        this.gameLog = [...msg.state.logEntries];
        this.onGameStarted?.(msg.state);
        break;
      case 'stateUpdate':
        if (msg.state.resync) {
          this.gameLog = [...msg.state.logEntries];
        } else {
          this.gameLog.push(...msg.state.logEntries);
        }
        this.onSanitizedStateChanged?.(msg.state);
        break;
      case 'gameOver':
        if (msg.state.resync) {
          this.gameLog = [...msg.state.logEntries];
        } else {
          this.gameLog.push(...msg.state.logEntries);
        }
        this.onGameOver?.(msg.state);
        break;
      case 'error':
        this.onError?.(msg.message, msg.context);
        break;
      case 'hostLeft':
        this.stopHeartbeat();
        this.onHostDisconnected?.(msg.reason);
        break;
      case 'playerDisconnected':
        this.onPlayerDisconnected?.(msg.playerIndex, msg.playerName);
        break;
      case 'playerReconnected':
        this.onPlayerReconnected?.(msg.playerIndex, msg.playerName);
        break;
      case 'playerReplacedByAI':
        this.onPlayerReplacedByAI?.(msg.playerIndex, msg.playerName);
        break;
      case 'pong': {
        const now = Date.now();
        this.lastPongAt = now;
        this.latencyMs = Math.max(0, now - msg.t);
        this.onLatencyChange?.(this.latencyMs, false);
        break;
      }
    }
  }
}
