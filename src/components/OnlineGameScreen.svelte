<script lang="ts">
  import type { GameState, Choice, StructuredGameLog } from '../data/types';
  import type { SanitizedGameState } from '../network/types';
  import type { HostController } from '../network/hostController';
  import type { GuestController } from '../network/guestController';
  import { sanitizedToGameState } from '../network/stateAdapter';
  import { AIController } from '../ai/aiController';
  import { getActivePlayerIndex, isCurrentPlayerAI } from '../gameUtils';
  import GameLayout from './GameLayout.svelte';
  import DraftPhaseView from './DraftPhaseView.svelte';
  import ActionPhaseView from './ActionPhaseView.svelte';

  let { role, hostController, guestController, onGameOver, gameStartTime, onLeaveGame }: {
    role: 'host' | 'guest';
    hostController?: HostController;
    guestController?: GuestController;
    onGameOver: (gameState: GameState, structuredLog?: StructuredGameLog) => void;
    gameStartTime: number;
    onLeaveGame: () => void;
  } = $props();

  // Host state
  let hostGameState: GameState | null = $state(null);
  let hostGameLog: string[] = $state([]);

  // Guest state
  let guestSanitizedState: SanitizedGameState | null = $state(null);
  let guestGameLog: string[] = $state([]);

  // Shared state
  let aiThinking = $state(false);
  // svelte-ignore state_referenced_locally
  let effectiveStartTime = $state(gameStartTime);
  let connectionStatus = $state<'ok' | 'stalled' | 'lost'>('ok');
  let latencyMs = $state(0);
  let connectionBanner: { text: string; kind: 'info' | 'warn' } | null = $state(null);
  let connectionBannerTimer: ReturnType<typeof setTimeout> | null = null;

  // Timer
  let elapsedSeconds = $state(0);

  $effect(() => {
    elapsedSeconds = Math.floor((Date.now() - effectiveStartTime) / 1000);
    const interval = setInterval(() => {
      elapsedSeconds = Math.floor((Date.now() - effectiveStartTime) / 1000);
    }, 1000);
    return () => clearInterval(interval);
  });

  // AI controller (host only)
  // svelte-ignore state_referenced_locally
  const aiController = role === 'host' ? new AIController() : null;
  let aiGenerationSeq = 0;

  let gameState = $derived.by(() => {
    if (role === 'host') {
      return hostGameState;
    } else if (guestSanitizedState) {
      return sanitizedToGameState(guestSanitizedState);
    }
    return null;
  });

  let gameLog = $derived(role === 'host' ? hostGameLog : guestGameLog);

  let myPlayerIndex = $derived(
    role === 'host' ? 0 : (guestSanitizedState?.myPlayerIndex ?? -1)
  );

  let activePlayerIndex = $derived(
    gameState ? getActivePlayerIndex(gameState) : -1
  );

  let isMyTurn = $derived(activePlayerIndex === myPlayerIndex);

  let selectedPlayerIndex = $state(0);

  $effect(() => {
    if (myPlayerIndex >= 0) {
      selectedPlayerIndex = myPlayerIndex;
    }
  });

  function selectPlayer(index: number) {
    selectedPlayerIndex = index;
  }

  function showBanner(text: string, kind: 'info' | 'warn' = 'info', durationMs = 4000) {
    if (connectionBannerTimer) clearTimeout(connectionBannerTimer);
    connectionBanner = { text, kind };
    connectionBannerTimer = setTimeout(() => {
      connectionBanner = null;
    }, durationMs);
  }

  // Setup host controller callbacks
  // svelte-ignore state_referenced_locally
  if (role === 'host' && hostController) {
    hostController.onGameStateChanged = (state) => {
      hostGameState = state;
    };
    hostController.onLogUpdated = (log) => {
      hostGameLog = [...log];
    };
    hostController.onGameOver = (state) => {
      const structuredLog = hostController!.getStructuredLog() ?? undefined;
      onGameOver(state, structuredLog);
    };
    hostController.onPlayerDisconnected = (_idx, name) => {
      showBanner(`${name} disconnected — waiting up to 60s before AI takes over`, 'warn', 6000);
    };
    hostController.onPlayerReconnected = (_idx, name) => {
      showBanner(`${name} reconnected`, 'info');
    };
    hostController.onPlayerReplacedByAI = (_idx, name) => {
      showBanner(`${name} replaced by AI`, 'warn', 6000);
    };

    hostGameState = hostController.getGameState();
    hostGameLog = [...hostController.getGameLog()];
    effectiveStartTime = hostController.getGameStartTime() || gameStartTime;
  }

  // Setup guest controller callbacks
  // svelte-ignore state_referenced_locally
  if (role === 'guest' && guestController) {
    guestController.onSanitizedStateChanged = (state) => {
      guestSanitizedState = state;
      guestGameLog = [...guestController!.getGameLog()];
      if (state.gameStartTime) effectiveStartTime = state.gameStartTime;
      // If host tells us a draft pick was rejected, unlock the UI.
    };
    guestController.onGameOver = (state) => {
      guestSanitizedState = state;
      guestGameLog = [...guestController!.getGameLog()];
      onGameOver(sanitizedToGameState(state));
    };
    guestController.onError = (message, context) => {
      if (context === 'draftPick') {
        hasPicked = false;
      }
      showBanner(message, 'warn', 5000);
    };
    guestController.onPlayerDisconnected = (_idx, name) => {
      showBanner(`${name} disconnected`, 'warn', 5000);
    };
    guestController.onPlayerReconnected = (_idx, name) => {
      showBanner(`${name} reconnected`, 'info');
    };
    guestController.onPlayerReplacedByAI = (_idx, name) => {
      showBanner(`${name} replaced by AI`, 'warn', 5000);
    };
    guestController.onLatencyChange = (ms, stalled) => {
      latencyMs = ms;
      connectionStatus = stalled ? 'stalled' : 'ok';
    };
  }

  // Draft card order tracking
  let draftCardOrder: number[][] = $state([]);

  let lastDraftedCounts: number[] = $state([]);
  $effect(() => {
    if (!gameState) return;
    if (draftCardOrder.length !== gameState.players.length) {
      draftCardOrder = gameState.players.map(() => []);
      lastDraftedCounts = gameState.players.map(p => p.draftedCards.length);
      return;
    }
    for (let i = 0; i < gameState.players.length; i++) {
      const currentCount = gameState.players[i].draftedCards.length;
      if (currentCount > (lastDraftedCounts[i] ?? 0)) {
        const knownIds = new Set(draftCardOrder[i]);
        for (const c of gameState.players[i].draftedCards) {
          if (!knownIds.has(c.instanceId)) {
            draftCardOrder[i] = [...draftCardOrder[i], c.instanceId];
          }
        }
      } else if (currentCount === 0 && draftCardOrder[i].length > 0) {
        draftCardOrder[i] = [];
      }
    }
    lastDraftedCounts = gameState.players.map(p => p.draftedCards.length);
  });

  // Simultaneous draft state
  let hasPicked = $state(false);
  let lastPickNumber: number | null = $state(null);

  $effect(() => {
    if (gameState?.phase.type === 'draft') {
      const currentPickNumber = gameState.phase.draftState.pickNumber;
      if (lastPickNumber !== null && currentPickNumber !== lastPickNumber) {
        hasPicked = false;
      }
      lastPickNumber = currentPickNumber;
    }
  });

  function handleAction(choice: Choice) {
    if (choice.type === 'draftPick') {
      hasPicked = true;
    }
    if (role === 'host') {
      hostController?.applyHostAction(choice);
    } else {
      guestController?.sendAction(choice);
    }
  }

  // AI turn handling (host only)
  $effect(() => {
    if (role !== 'host' || !gameState || aiThinking) return;
    if (gameState.phase.type === 'gameOver') return;
    if (gameState.phase.type === 'draw') return;

    if (gameState.phase.type === 'draft') {
      const ds = gameState.phase.draftState;
      const aiPlayerIndices = gameState.aiPlayers
        .map((isAI, idx) => isAI ? idx : -1)
        .filter(idx => idx >= 0)
        .filter(idx => !hostController!.hasSubmittedDraftPick(idx))
        .filter(idx => ds.hands[idx].length > 0);

      if (aiPlayerIndices.length === 0) return;

      aiThinking = true;
      const mySeq = ++aiGenerationSeq;
      const snapshotAI = [...gameState.aiPlayers];
      Promise.all(
        aiPlayerIndices.map(playerIdx => {
          return aiController!.getAIChoice(gameState!, playerIdx, 100000).then(choice => ({
            playerIdx,
            choice,
          }));
        })
      ).then(results => {
        aiThinking = false;
        if (mySeq !== aiGenerationSeq) return;
        for (const { playerIdx, choice } of results) {
          // Skip if the player has since reconnected (no longer AI) or state moved on.
          if (!hostController) break;
          const curState = hostController.getGameState();
          if (!curState || curState.phase.type !== 'draft') break;
          if (!curState.aiPlayers[playerIdx]) continue;
          if (!snapshotAI[playerIdx]) continue;
          if (hostController.hasSubmittedDraftPick(playerIdx)) continue;
          hostController.applyAction(choice, playerIdx);
        }
      }).catch((e) => {
        console.error('AI draft error:', e);
        aiThinking = false;
      });
      return;
    }

    if (!isCurrentPlayerAI(gameState)) return;

    const playerIdx = getActivePlayerIndex(gameState);
    aiThinking = true;
    const mySeq = ++aiGenerationSeq;

    aiController!.getAIChoice(gameState, playerIdx, 100000).then((choice) => {
      aiThinking = false;
      if (mySeq !== aiGenerationSeq) return;
      if (!hostController) return;
      const curState = hostController.getGameState();
      if (!curState || curState.phase.type === 'gameOver') return;
      // Abandon if this player is no longer AI (reconnected) or no longer the active player.
      if (!curState.aiPlayers[playerIdx]) return;
      if (getActivePlayerIndex(curState) !== playerIdx) return;
      hostController.applyAction(choice, playerIdx);
    }).catch((e) => {
      console.error('AI action error:', e);
      aiThinking = false;
    });
  });

</script>

{#if gameState}
  <GameLayout {gameState} {activePlayerIndex} {aiThinking} {elapsedSeconds} {gameLog} onLeaveGame={onLeaveGame} {selectedPlayerIndex} onSelectPlayer={selectPlayer} {draftCardOrder}>
    {#if connectionBanner}
      <div class="connection-banner" class:warn={connectionBanner.kind === 'warn'}>{connectionBanner.text}</div>
    {/if}

    {#if role === 'guest' && connectionStatus === 'stalled'}
      <div class="connection-banner warn">Connection to host appears stalled{latencyMs > 0 ? ` (last ping ${latencyMs}ms)` : ''}…</div>
    {/if}

    {#if !isMyTurn && !aiThinking && gameState.phase.type === 'action'}
      <div class="waiting-banner">
        <div class="spinner"></div>
        <p>Waiting for {gameState.playerNames[activePlayerIndex] ?? 'other player'}...</p>
      </div>
    {/if}

    {#if gameState.phase.type === 'draft'}
      <DraftPhaseView {gameState} onAction={handleAction} playerIndex={myPlayerIndex} selectable={!hasPicked} />
      {#if hasPicked}
        <div class="waiting-banner">
          <p>Waiting for other players to pick...</p>
        </div>
      {/if}
    {:else if gameState.phase.type === 'action' && isMyTurn}
      <ActionPhaseView {gameState} onAction={handleAction} onUndo={() => {}} undoAvailable={false} {draftCardOrder} />
    {/if}
  </GameLayout>
{/if}

<style>
  .waiting-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    justify-content: center;
    padding: 8px;
    color: #c9a84c;
    font-weight: 600;
    flex-wrap: wrap;
  }

  .waiting-banner p {
    font-size: 0.85rem;
    margin: 0;
    text-align: center;
  }

  .connection-banner {
    padding: 8px 12px;
    border-radius: 6px;
    background: rgba(201, 168, 76, 0.15);
    color: #2c1e12;
    font-size: 0.85rem;
    text-align: center;
    margin: 4px;
  }

  .connection-banner.warn {
    background: rgba(139, 32, 32, 0.12);
    color: #8b2020;
    font-weight: 600;
  }

  .spinner {
    width: 28px;
    height: 28px;
    border: 3px solid rgba(201, 168, 76, 0.3);
    border-top-color: #c9a84c;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @media (min-width: 768px) {
    .waiting-banner {
      padding: 12px;
      gap: 12px;
      flex-wrap: nowrap;
    }

    .waiting-banner p {
      font-size: 1rem;
      text-align: left;
    }

    .spinner {
      width: 40px;
      height: 40px;
      border-width: 4px;
    }
  }
</style>
