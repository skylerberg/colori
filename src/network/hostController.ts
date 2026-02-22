import type { GameState } from '../data/types';
import type { ColoriChoice } from '../ai/coloriGame';
import type { NetworkManager } from './networkManager';
import type { LobbyPlayer, GuestMessage } from './types';
import { createInitialGameState } from '../engine/setupPhase';
import { executeDrawPhase } from '../engine/drawPhase';
import { playerPick, confirmPass } from '../engine/draftPhase';
import {
  destroyDraftedCard, endPlayerTurn, resolveMakeMaterials,
  resolveMixColors, skipMix, resolveDestroyCards,
  resolveSelectGarment,
} from '../engine/actionPhase';
import { mixResult } from '../data/colors';
import { sanitizeGameState } from './sanitize';

export class HostController {
  private network: NetworkManager;
  private gameState: GameState | null = null;
  private lobbyPlayers: LobbyPlayer[] = [];
  private peerToPlayerIndex: Map<string, number> = new Map();
  private playerIndexToPeer: Map<number, string> = new Map();
  private playerCount: number = 2;
  private gameLog: string[] = [];
  private disconnectTimers: Map<number, ReturnType<typeof setTimeout>> = new Map();

  onLobbyUpdated: ((players: LobbyPlayer[]) => void) | null = null;
  onGameStateUpdated: ((state: GameState) => void) | null = null;
  onLogUpdated: ((log: string[]) => void) | null = null;
  onGameOver: ((state: GameState) => void) | null = null;

  constructor(network: NetworkManager, hostName: string) {
    this.network = network;

    this.lobbyPlayers.push({
      peerId: 'host',
      name: hostName,
      playerIndex: 0,
      isHost: true,
      connected: true,
    });

    this.network.onGuestMessage = (msg, peerId) => this.handleGuestMessage(msg, peerId);
    this.network.onPeerLeave = (peerId) => this.handlePeerDisconnect(peerId);
  }

  setPlayerCount(count: number) {
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

  private handleGuestMessage(msg: GuestMessage, peerId: string) {
    switch (msg.type) {
      case 'joinRequest':
        this.handleJoinRequest(peerId, msg.name);
        break;
      case 'rejoinRequest':
        this.handleRejoinRequest(peerId, msg.name);
        break;
      case 'action':
        this.handlePlayerAction(peerId, msg.choice);
        break;
    }
  }

  private handleJoinRequest(peerId: string, name: string) {
    if (this.gameState) {
      this.network.sendToGuest({ type: 'error', message: 'Game already in progress' }, peerId);
      return;
    }

    if (this.peerToPlayerIndex.has(peerId)) return;

    const humanCount = this.lobbyPlayers.length;
    if (humanCount >= this.playerCount) {
      this.network.sendToGuest({ type: 'error', message: 'Game is full' }, peerId);
      return;
    }

    const usedIndices = new Set(this.lobbyPlayers.map(p => p.playerIndex));
    let playerIndex = 1;
    while (usedIndices.has(playerIndex)) playerIndex++;

    this.lobbyPlayers.push({
      peerId,
      name,
      playerIndex,
      isHost: false,
      connected: true,
    });
    this.peerToPlayerIndex.set(peerId, playerIndex);
    this.playerIndexToPeer.set(playerIndex, peerId);
    this.broadcastLobbyUpdate();
  }

  private handleRejoinRequest(peerId: string, name: string) {
    if (!this.gameState) {
      this.network.sendToGuest({ type: 'error', message: 'No game in progress' }, peerId);
      return;
    }

    const player = this.lobbyPlayers.find(p => !p.connected && p.name === name);
    if (!player) {
      this.network.sendToGuest({ type: 'error', message: 'Cannot rejoin: no matching disconnected player' }, peerId);
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
    player.connected = true;
    this.peerToPlayerIndex.set(peerId, player.playerIndex);
    this.playerIndexToPeer.set(player.playerIndex, peerId);
    this.gameState.aiPlayers[player.playerIndex] = false;

    const sanitized = sanitizeGameState(this.gameState, player.playerIndex, [...this.gameLog]);
    this.network.sendToGuest({ type: 'stateUpdate', state: sanitized }, peerId);

    this.network.sendToAllGuests({
      type: 'playerReconnected',
      playerIndex: player.playerIndex,
      playerName: player.name,
    });

    this.addLog(`${player.name} reconnected`);
    this.broadcastLobbyUpdate();
    this.onGameStateUpdated?.(this.gameState);
  }

  private handlePeerDisconnect(peerId: string) {
    const playerIndex = this.peerToPlayerIndex.get(peerId);
    if (playerIndex === undefined) return;

    const player = this.lobbyPlayers.find(p => p.peerId === peerId);
    if (!player) return;

    player.connected = false;

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

    const timer = setTimeout(() => {
      this.replaceWithAI(player.playerIndex);
    }, 60000);
    this.disconnectTimers.set(player.playerIndex, timer);
  }

  private replaceWithAI(playerIndex: number) {
    if (!this.gameState) return;
    this.gameState.aiPlayers[playerIndex] = true;
    this.disconnectTimers.delete(playerIndex);
    this.addLog(`${this.gameState.players[playerIndex].name} replaced by AI`);
    this.onGameStateUpdated?.(this.gameState);
  }

  startGame() {
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

    this.gameState = createInitialGameState(playerNames, aiPlayers);
    this.gameLog = [];
    this.addLog('Game started');

    this.executeDrawIfNeeded();

    for (const lp of this.lobbyPlayers) {
      if (!lp.isHost && lp.connected) {
        const sanitized = sanitizeGameState(this.gameState, lp.playerIndex, [...this.gameLog]);
        this.network.sendToGuest({ type: 'gameStarted', state: sanitized }, lp.peerId);
      }
    }

    this.onGameStateUpdated?.(this.gameState);
    this.onLogUpdated?.(this.gameLog);
  }

  applyHostAction(choice: ColoriChoice) {
    if (!this.gameState) return;
    this.applyAction(choice, 0);
  }

  private handlePlayerAction(peerId: string, choice: ColoriChoice) {
    const playerIndex = this.peerToPlayerIndex.get(peerId);
    if (playerIndex === undefined) return;
    this.applyAction(choice, playerIndex);
  }

  applyAction(choice: ColoriChoice, playerIndex: number) {
    if (!this.gameState) return;

    const activeIndex = this.getActivePlayerIndex();
    if (activeIndex !== playerIndex) {
      const peerId = this.playerIndexToPeer.get(playerIndex);
      if (peerId) {
        this.network.sendToGuest({ type: 'error', message: 'Not your turn' }, peerId);
      }
      return;
    }

    const newLogEntries: string[] = [];
    const name = this.gameState.players[playerIndex].name;

    switch (choice.type) {
      case 'draftPick':
        playerPick(this.gameState, choice.cardInstanceId);
        if (this.gameState.phase.type === 'draft' && this.gameState.phase.draftState.waitingForPass) {
          confirmPass(this.gameState);
        }
        break;
      case 'destroyDraftedCard': {
        const card = this.gameState.players[playerIndex].draftedCards.find(
          c => c.instanceId === choice.cardInstanceId
        );
        newLogEntries.push(`${name} destroyed ${card && 'name' in card.card ? card.card.name : 'a card'} from drafted cards`);
        destroyDraftedCard(this.gameState, choice.cardInstanceId);
        break;
      }
      case 'endTurn':
        newLogEntries.push(`${name} ended their turn`);
        endPlayerTurn(this.gameState);
        break;
      case 'makeMaterials': {
        const cardNames = choice.cardInstanceIds.map(id => {
          const c = this.gameState!.players[playerIndex].drawnCards.find(c => c.instanceId === id);
          return c && 'name' in c.card ? c.card.name : 'a card';
        });
        newLogEntries.push(`${name} stored materials from ${cardNames.join(', ')}`);
        resolveMakeMaterials(this.gameState, choice.cardInstanceIds);
        break;
      }
      case 'destroyDrawnCards': {
        const cardNames = choice.cardInstanceIds.map(id => {
          const c = this.gameState!.players[playerIndex].drawnCards.find(c => c.instanceId === id);
          return c && 'name' in c.card ? c.card.name : 'a card';
        });
        newLogEntries.push(`${name} destroyed ${cardNames.join(', ')} from drawn cards`);
        resolveDestroyCards(this.gameState, choice.cardInstanceIds);
        break;
      }
      case 'mix': {
        const result = mixResult(choice.colorA, choice.colorB);
        newLogEntries.push(`${name} mixed ${choice.colorA} + ${choice.colorB} to make ${result}`);
        resolveMixColors(this.gameState, choice.colorA, choice.colorB);
        break;
      }
      case 'skipMix':
        newLogEntries.push(`${name} skipped remaining mixes`);
        skipMix(this.gameState);
        break;
      case 'selectGarment': {
        const garment = this.gameState.garmentDisplay.find(g => g.instanceId === choice.garmentInstanceId);
        newLogEntries.push(`${name} completed a ${garment?.card.stars ?? '?'}-star garment`);
        resolveSelectGarment(this.gameState, choice.garmentInstanceId);
        break;
      }
    }

    this.gameLog.push(...newLogEntries);
    this.executeDrawIfNeeded();
    this.broadcastGameState(newLogEntries);
    this.onGameStateUpdated?.(this.gameState);
    this.onLogUpdated?.(this.gameLog);
  }

  getActivePlayerIndex(): number {
    if (!this.gameState) return -1;
    if (this.gameState.phase.type === 'draft') {
      return this.gameState.phase.draftState.currentPlayerIndex;
    }
    if (this.gameState.phase.type === 'action') {
      return this.gameState.phase.actionState.currentPlayerIndex;
    }
    return -1;
  }

  private executeDrawIfNeeded() {
    if (!this.gameState) return;
    if (this.gameState.phase.type === 'draw') {
      this.gameLog.push(`Round ${this.gameState.round} began`);
      executeDrawPhase(this.gameState);
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
      if (!lp.isHost && lp.connected) {
        const sanitized = sanitizeGameState(this.gameState, lp.playerIndex, newLogEntries);
        if (isGameOver) {
          this.network.sendToGuest({ type: 'gameOver', state: sanitized }, lp.peerId);
        } else {
          this.network.sendToGuest({ type: 'stateUpdate', state: sanitized }, lp.peerId);
        }
      }
    }

    if (isGameOver) {
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

  cleanup() {
    for (const timer of this.disconnectTimers.values()) {
      clearTimeout(timer);
    }
    this.disconnectTimers.clear();
  }
}
