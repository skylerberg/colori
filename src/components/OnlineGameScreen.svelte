<script lang="ts">
  import type { GameState, Choice, StructuredGameLog } from '../data/types';
  import type { SanitizedGameState } from '../network/types';
  import type { HostController } from '../network/hostController';
  import type { GuestController } from '../network/guestController';
  import { sanitizedToGameState } from '../network/stateAdapter';
  import { AIController } from '../ai/aiController';
  import { cloneGameState } from '../engine/wasmEngine';
  import { getActivePlayerIndex, isCurrentPlayerAI, orderByDraftOrder } from '../gameUtils';
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

  // Timer
  let elapsedSeconds = $state(0);

  $effect(() => {
    elapsedSeconds = Math.floor((Date.now() - gameStartTime) / 1000);
    const interval = setInterval(() => {
      elapsedSeconds = Math.floor((Date.now() - gameStartTime) / 1000);
    }, 1000);
    return () => clearInterval(interval);
  });

  // AI controller (host only)
  // svelte-ignore state_referenced_locally
  const aiController = role === 'host' ? new AIController() : null;

  // Derive the GameState for rendering (works for both host and guest)
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

  // Initialize selectedPlayerIndex to myPlayerIndex when it becomes available
  $effect(() => {
    if (myPlayerIndex >= 0) {
      selectedPlayerIndex = myPlayerIndex;
    }
  });

  function selectPlayer(index: number) {
    selectedPlayerIndex = index;
  }

  let myPlayer = $derived(
    gameState && myPlayerIndex >= 0 ? gameState.players[myPlayerIndex] : null
  );

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

    // Initialize with current state
    hostGameState = hostController.getGameState();
    hostGameLog = [...hostController.getGameLog()];
  }

  // Setup guest controller callbacks
  // svelte-ignore state_referenced_locally
  if (role === 'guest' && guestController) {
    guestController.onSanitizedStateChanged = (state) => {
      guestSanitizedState = state;
      guestGameLog = [...guestController!.getGameLog()];
    };
    guestController.onGameOver = (state) => {
      guestSanitizedState = state;
      guestGameLog = [...guestController!.getGameLog()];
      onGameOver(sanitizedToGameState(state));
    };
  }

  // Draft card order tracking
  let draftCardOrder: number[][] = $state([]);

  // Sync draftCardOrder when gameState changes (detect newly drafted cards)
  let lastDraftedCounts: number[] = $state([]);
  $effect(() => {
    if (!gameState) return;
    // Initialize if player count changed
    if (draftCardOrder.length !== gameState.players.length) {
      draftCardOrder = gameState.players.map(() => []);
      lastDraftedCounts = gameState.players.map(p => p.draftedCards.length);
      return;
    }
    // Detect newly added drafted cards for each player
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
        // New round — reset
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

  // Handle action from phase views
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

    // Simultaneous AI drafting: compute picks for all AI players at once
    if (gameState.phase.type === 'draft') {
      const ds = gameState.phase.draftState;
      const aiPlayerIndices = gameState.aiPlayers
        .map((isAI, idx) => isAI ? idx : -1)
        .filter(idx => idx >= 0)
        .filter(idx => !hostController!['submittedDraftPicks'].has(idx))
        .filter(idx => ds.hands[idx].length > 0);

      if (aiPlayerIndices.length === 0) return;

      aiThinking = true;
      Promise.all(
        aiPlayerIndices.map(playerIdx => {
          return aiController!.getAIChoice(gameState!, playerIdx, 100000).then(choice => ({
            playerIdx,
            choice,
          }));
        })
      ).then(results => {
        aiThinking = false;
        for (const { playerIdx, choice } of results) {
          hostController?.applyAction(choice, playerIdx);
        }
      }).catch((e) => {
        console.error('AI draft error:', e);
        aiThinking = false;
      });
      return;
    }

    // For non-draft phases, use sequential AI turns
    if (!isCurrentPlayerAI(gameState)) return;

    const playerIdx = getActivePlayerIndex(gameState);
    aiThinking = true;

    aiController!.getAIChoice(gameState, playerIdx, 100000).then((choice) => {
      aiThinking = false;
      hostController?.applyAction(choice, playerIdx);
    }).catch((e) => {
      console.error('AI action error:', e);
      aiThinking = false;
    });
  });

</script>

{#if gameState}
  <GameLayout {gameState} {activePlayerIndex} {aiThinking} {elapsedSeconds} {gameLog} onLeaveGame={onLeaveGame} {selectedPlayerIndex} onSelectPlayer={selectPlayer} {draftCardOrder}>
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

  /* ===== RESPONSIVE OVERRIDES (mobile-first) ===== */

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
