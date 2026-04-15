import type { GameState } from '../data/types';
import type { Choice } from '../data/types';
import type { NetworkManager } from './networkManager';
import type { LobbyPlayer, GuestMessage } from './types';
import { createInitialGameState, executeDrawPhase, simultaneousPick, advanceDraft, applyChoice, getChoiceLogMessage } from '../engine/wasmEngine';
import { sanitizeGameState } from './sanitize';
import type { StructuredGameLog } from '../data/types';
import { GameLogAccumulator } from '../gameLog';
import { getActivePlayerIndex } from '../gameUtils';

const DISCONNECT_GRACE_MS = 60_000;
const MAX_NAME_LENGTH = 20;
const HOST_REJOIN_TOKEN = 'host';

function sanitizeName(raw: string): string {
  const trimmed = (raw ?? '').trim().replace(/\s+/g, ' ');
  return trimmed.slice(0, MAX_NAME_LENGTH);
}

function randomToken(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, b => b.toString(16).padStart(2, '0')).join('');
}

export class HostController {
  private network: NetworkManager;
  private gameState: GameState | null = null;
  private lobbyPlayers: LobbyPlayer[] = [];
  private peerToPlayerIndex: Map<string, number> = new Map();
  private playerIndexToPeer: Map<number, string> = new Map();
  private playerCount: number = 2;
  private gameLog: string[] = [];
  private disconnectTimers: Map<number, ReturnType<typeof setTimeout>> = new Map();
  private submittedDraftPicks: Set<number> = new Set();
  private structuredLog: GameLogAccumulator | null = null;
  private rejoinTokens: Map<number, string> = new Map();
  private gameStartTime: number = 0;

  onLobbyUpdated: ((players: LobbyPlayer[]) => void) | null = null;
  onGameStateChanged: ((state: GameState) => void) | null = null;
  onLogUpdated: ((log: string[]) => void) | null = null;
  onGameOver: ((state: GameState) => void) | null = null;
  onPlayerDisconnected: ((playerIndex: number, playerName: string) => void) | null = null;
  onPlayerReconnected: ((playerIndex: number, playerName: string) => void) | null = null;
  onPlayerReplacedByAI: ((playerIndex: number, playerName: string) => void) | null = null;

  constructor(network: NetworkManager, hostName: string) {
    this.network = network;

    this.lobbyPlayers.push({
      peerId: 'host',
      name: sanitizeName(hostName) || 'Host',
      playerIndex: 0,
      isHost: true,
      isConnected: true,
    });

    this.network.onGuestMessage = (msg, peerId) => this.handleGuestMessage(msg, peerId);
    this.network.onPeerLeave = (peerId) => this.handlePeerDisconnect(peerId);
  }

  setHostName(name: string) {
    const clean = sanitizeName(name) || 'Host';
    if (this.lobbyPlayers[0].name === clean) return;
    if (!this.gameState && this.isNameTaken(clean, 0)) return;
    this.lobbyPlayers[0].name = clean;
    this.broadcastLobbyUpdate();
  }

  setPlayerCount(count: number) {
    const connectedHumans = this.lobbyPlayers.filter(p => p.isConnected).length;
    if (count < connectedHumans) return;
    const maxIndex = this.lobbyPlayers.reduce((m, p) => Math.max(m, p.playerIndex), 0);
    if (count <= maxIndex) return;
    this.playerCount = count;
    this.broadcastLobbyUpdate();
  }

  getPlayerCount(): number {
    return this.playerCount;
  }

  getLobbyPlayers(): LobbyPlayer[] {
    return this.lobbyPlayers;
  }

  getGameState(): GameState | null {
    return this.gameState;
  }

  getGameLog(): string[] {
    return this.gameLog;
  }

  getGameStartTime(): number {
    return this.gameStartTime;
  }

  getStructuredLog(): StructuredGameLog | null {
    return this.structuredLog?.getLog() ?? null;
  }

  getHostRejoinToken(): string {
    return HOST_REJOIN_TOKEN;
  }

  hasSubmittedDraftPick(playerIndex: number): boolean {
    return this.submittedDraftPicks.has(playerIndex);
  }

  private isNameTaken(name: string, exceptIndex: number): boolean {
    return this.lobbyPlayers.some(p => p.playerIndex !== exceptIndex && p.name.toLowerCase() === name.toLowerCase());
  }

  private handleGuestMessage(msg: GuestMessage, peerId: string) {
    switch (msg.type) {
      case 'joinRequest':
        this.handleJoinRequest(peerId, msg.name);
        break;
      case 'rejoinRequest':
        this.handleRejoinRequest(peerId, msg.name, msg.rejoinToken);
        break;
      case 'action':
        this.handlePlayerAction(peerId, msg.choice);
        break;
      case 'ping':
        this.network.sendToGuest({ type: 'pong', t: msg.t }, peerId);
        break;
    }
  }

  private handleJoinRequest(peerId: string, rawName: string) {
    const name = sanitizeName(rawName);
    if (!name) {
      this.network.sendToGuest({ type: 'error', message: 'Name required', context: 'join' }, peerId);
      return;
    }

    if (this.gameState) {
      this.network.sendToGuest({ type: 'error', message: 'Game already in progress. Use rejoin.', context: 'join' }, peerId);
      return;
    }

    if (this.peerToPlayerIndex.has(peerId)) return;

    if (this.isNameTaken(name, -1)) {
      this.network.sendToGuest({ type: 'error', message: 'Name already taken', context: 'join' }, peerId);
      return;
    }

    const humanCount = this.lobbyPlayers.length;
    if (humanCount >= this.playerCount) {
      this.network.sendToGuest({ type: 'error', message: 'Game is full', context: 'join' }, peerId);
      return;
    }

    const usedIndices = new Set(this.lobbyPlayers.map(p => p.playerIndex));
    let playerIndex = 1;
    while (usedIndices.has(playerIndex)) playerIndex++;

    if (playerIndex >= this.playerCount) {
      this.network.sendToGuest({ type: 'error', message: 'Game is full', context: 'join' }, peerId);
      return;
    }

    this.lobbyPlayers.push({
      peerId,
      name,
      playerIndex,
      isHost: false,
      isConnected: true,
    });
    this.peerToPlayerIndex.set(peerId, playerIndex);
    this.playerIndexToPeer.set(playerIndex, peerId);
    this.broadcastLobbyUpdate();
  }

  private handleRejoinRequest(peerId: string, rawName: string, token: string) {
    const name = sanitizeName(rawName);
    if (!this.gameState) {
      this.network.sendToGuest({ type: 'error', message: 'No game in progress', context: 'join' }, peerId);
      return;
    }

    let slotIndex: number | undefined;
    for (const [idx, stored] of this.rejoinTokens.entries()) {
      if (stored === token && token && token !== HOST_REJOIN_TOKEN) {
        slotIndex = idx;
        break;
      }
    }
    if (slotIndex === undefined) {
      this.network.sendToGuest({ type: 'error', message: 'Invalid rejoin token', context: 'join' }, peerId);
      return;
    }

    const player = this.lobbyPlayers.find(p => p.playerIndex === slotIndex);
    if (!player) {
      this.network.sendToGuest({ type: 'error', message: 'No matching player slot', context: 'join' }, peerId);
      return;
    }
    if (player.name !== name) {
      this.network.sendToGuest({ type: 'error', message: 'Name does not match rejoin token', context: 'join' }, peerId);
      return;
    }

    const timer = this.disconnectTimers.get(player.playerIndex);
    if (timer) {
      clearTimeout(timer);
      this.disconnectTimers.delete(player.playerIndex);
    }

    const oldPeerId = player.peerId;
    this.peerToPlayerIndex.delete(oldPeerId);

    player.peerId = peerId;
    player.isConnected = true;
    this.peerToPlayerIndex.set(peerId, player.playerIndex);
    this.playerIndexToPeer.set(player.playerIndex, peerId);
    this.gameState.aiPlayers[player.playerIndex] = false;

    const sanitized = sanitizeGameState(this.gameState, player.playerIndex, [...this.gameLog]);
    sanitized.resync = true;
    sanitized.gameStartTime = this.gameStartTime;
    this.network.sendToGuest({ type: 'stateUpdate', state: sanitized }, peerId);

    this.network.sendToAllGuests({
      type: 'playerReconnected',
      playerIndex: player.playerIndex,
      playerName: player.name,
    });

    this.addLog(`${player.name} reconnected`);
    this.broadcastLobbyUpdate();
    this.onPlayerReconnected?.(player.playerIndex, player.name);
    this.onGameStateChanged?.(this.gameState);
  }

  private handlePeerDisconnect(peerId: string) {
    const playerIndex = this.peerToPlayerIndex.get(peerId);
    if (playerIndex === undefined) return;

    const player = this.lobbyPlayers.find(p => p.peerId === peerId);
    if (!player) return;

    player.isConnected = false;

    if (!this.gameState) {
      this.lobbyPlayers = this.lobbyPlayers.filter(p => p.peerId !== peerId);
      this.peerToPlayerIndex.delete(peerId);
      this.playerIndexToPeer.delete(playerIndex);
      this.broadcastLobbyUpdate();
      return;
    }

    this.network.sendToAllGuests({
      type: 'playerDisconnected',
      playerIndex: player.playerIndex,
      playerName: player.name,
    });

    this.addLog(`${player.name} disconnected`);
    this.broadcastLobbyUpdate();
    this.onPlayerDisconnected?.(player.playerIndex, player.name);

    const timer = setTimeout(() => {
      this.replaceWithAI(player.playerIndex);
    }, DISCONNECT_GRACE_MS);
    this.disconnectTimers.set(player.playerIndex, timer);
  }

  private replaceWithAI(playerIndex: number) {
    if (!this.gameState) return;
    this.gameState.aiPlayers[playerIndex] = true;
    this.disconnectTimers.delete(playerIndex);
    const name = this.gameState.playerNames[playerIndex];
    this.addLog(`${name} replaced by AI`);
    this.network.sendToAllGuests({
      type: 'playerReplacedByAI',
      playerIndex,
      playerName: name,
    });
    this.onPlayerReplacedByAI?.(playerIndex, name);
    this.onGameStateChanged?.(this.gameState);
  }

  startGame() {
    this.submittedDraftPicks.clear();
    this.rejoinTokens.clear();
    const playerNames: string[] = new Array(this.playerCount);
    const aiPlayers: boolean[] = new Array(this.playerCount).fill(true);

    for (const lp of this.lobbyPlayers) {
      if (lp.playerIndex < this.playerCount) {
        playerNames[lp.playerIndex] = lp.name;
        aiPlayers[lp.playerIndex] = false;
      }
    }

    for (let i = 0; i < this.playerCount; i++) {
      if (!playerNames[i]) {
        playerNames[i] = `AI Player ${i + 1}`;
      }
    }

    try {
      this.gameState = createInitialGameState(playerNames, aiPlayers);
    } catch (e) {
      console.error('Failed to create game state', e);
      return;
    }
    this.structuredLog = new GameLogAccumulator(this.gameState);
    this.gameLog = [];
    this.gameStartTime = Date.now();
    this.addLog('Game started');

    for (const lp of this.lobbyPlayers) {
      this.rejoinTokens.set(lp.playerIndex, lp.isHost ? HOST_REJOIN_TOKEN : randomToken());
    }

    this.executeDrawIfNeeded();

    for (const lp of this.lobbyPlayers) {
      if (!lp.isHost && lp.isConnected) {
        const sanitized = sanitizeGameState(this.gameState, lp.playerIndex, [...this.gameLog]);
        sanitized.gameStartTime = this.gameStartTime;
        sanitized.resync = true;
        const token = this.rejoinTokens.get(lp.playerIndex) ?? '';
        this.network.sendToGuest({ type: 'gameStarted', state: sanitized, rejoinToken: token }, lp.peerId);
      }
    }

    this.onGameStateChanged?.(this.gameState);
    this.onLogUpdated?.(this.gameLog);
  }

  applyHostAction(choice: Choice) {
    if (!this.gameState) return;
    this.applyAction(choice, 0);
  }

  private handlePlayerAction(peerId: string, choice: Choice) {
    const playerIndex = this.peerToPlayerIndex.get(peerId);
    if (playerIndex === undefined) return;
    this.applyAction(choice, playerIndex);
  }

  applyAction(choice: Choice, playerIndex: number) {
    if (!this.gameState) return;
    const peerId = this.playerIndexToPeer.get(playerIndex);

    if (this.gameState.phase.type === 'draft' && choice.type === 'draftPick') {
      if (this.submittedDraftPicks.has(playerIndex)) {
        if (peerId) {
          this.network.sendToGuest({ type: 'error', message: 'Already picked this round', context: 'draftPick' }, peerId);
        }
        return;
      }

      const ds = this.gameState.phase.draftState;
      if (!ds.hands[playerIndex] || ds.hands[playerIndex].length === 0) {
        if (peerId) {
          this.network.sendToGuest({ type: 'error', message: 'No cards to pick this round', context: 'draftPick' }, peerId);
        }
        return;
      }

      try {
        this.structuredLog?.recordChoice(this.gameState, choice, playerIndex);
        simultaneousPick(this.gameState, playerIndex, choice.card);
      } catch (e) {
        console.error('simultaneousPick failed', e);
        if (peerId) {
          this.network.sendToGuest({ type: 'error', message: 'Invalid draft pick', context: 'draftPick' }, peerId);
        }
        return;
      }
      this.submittedDraftPicks.add(playerIndex);
      this.broadcastGameState([]);
      this.onGameStateChanged?.(this.gameState);

      // The pick-round is complete when every player has either submitted this round
      // OR started the round with no cards to pick. Counting `hands[idx].length > 0`
      // *after* simultaneousPick is wrong: it excludes the player who just emptied
      // their hand, so the threshold drops below the still-pending picks and triggers
      // an early advanceDraft — which then drops the next pick that arrives over the
      // network or is applied by the AI loop. With submittedDraftPicks.has(idx) we
      // count "I've already done my pick this round" correctly regardless of order.
      const ds2 = (this.gameState.phase as { type: 'draft'; draftState: { hands: unknown[][] } }).draftState;
      const allDone = this.gameState.players.every(
        (_, idx) => this.submittedDraftPicks.has(idx) || ds2.hands[idx].length === 0,
      );
      if (allDone) {
        try {
          advanceDraft(this.gameState);
        } catch (e) {
          console.error('advanceDraft failed', e);
          return;
        }
        this.submittedDraftPicks.clear();
        this.executeDrawIfNeeded();
        this.broadcastGameState([]);
        this.onGameStateChanged?.(this.gameState);
        this.onLogUpdated?.(this.gameLog);
      }
      return;
    }

    const activeIndex = getActivePlayerIndex(this.gameState);
    if (activeIndex !== playerIndex) {
      if (peerId) {
        this.network.sendToGuest({ type: 'error', message: 'Not your turn', context: 'action' }, peerId);
      }
      return;
    }

    let newLogEntries: string[] = [];
    try {
      this.structuredLog?.recordChoice(this.gameState, choice, playerIndex);
      const logMsg = getChoiceLogMessage(this.gameState, choice, playerIndex);
      newLogEntries = logMsg ? logMsg.split('\n') : [];

      const draws = applyChoice(this.gameState, choice);
      this.structuredLog?.attachDrawsToLastEntry(draws);
    } catch (e) {
      console.error('applyChoice failed', e);
      if (peerId) {
        this.network.sendToGuest({ type: 'error', message: 'Invalid action', context: 'action' }, peerId);
      }
      return;
    }

    this.gameLog.push(...newLogEntries);
    this.executeDrawIfNeeded();
    this.broadcastGameState(newLogEntries);
    this.onGameStateChanged?.(this.gameState);
    this.onLogUpdated?.(this.gameLog);
  }

  private executeDrawIfNeeded() {
    if (!this.gameState) return;
    if (this.gameState.phase.type === 'draw') {
      this.gameLog.push(`Round ${this.gameState.round} began`);
      try {
        const draws = executeDrawPhase(this.gameState);
        this.structuredLog?.recordDrawPhaseDraws(draws);
      } catch (e) {
        console.error('executeDrawPhase failed', e);
      }
    }
  }

  private addLog(entry: string) {
    this.gameLog.push(entry);
    this.onLogUpdated?.(this.gameLog);
  }

  private broadcastGameState(newLogEntries: string[]) {
    if (!this.gameState) return;

    const isGameOver = this.gameState.phase.type === 'gameOver';

    for (const lp of this.lobbyPlayers) {
      if (!lp.isHost && lp.isConnected) {
        const sanitized = sanitizeGameState(this.gameState, lp.playerIndex, newLogEntries);
        if (isGameOver) {
          this.network.sendToGuest({ type: 'gameOver', state: sanitized }, lp.peerId);
        } else {
          this.network.sendToGuest({ type: 'stateUpdate', state: sanitized }, lp.peerId);
        }
      }
    }

    if (isGameOver) {
      this.structuredLog?.finalize(this.gameState);
      this.onGameOver?.(this.gameState);
    }
  }

  private broadcastLobbyUpdate() {
    this.network.sendToAllGuests({
      type: 'lobbyUpdate',
      players: this.lobbyPlayers,
      playerCount: this.playerCount,
    });
    this.onLobbyUpdated?.(this.lobbyPlayers);
  }

  announceHostLeaving() {
    this.network.sendToAllGuests({ type: 'hostLeft', reason: 'intentional' });
  }

  cleanup() {
    for (const timer of this.disconnectTimers.values()) {
      clearTimeout(timer);
    }
    this.disconnectTimers.clear();
  }
}
