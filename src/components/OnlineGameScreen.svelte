<script lang="ts">
  import type { GameState, CardInstance, Choice, StructuredGameLog } from '../data/types';
  import type { SanitizedGameState } from '../network/types';
  import type { HostController } from '../network/hostController';
  import type { GuestController } from '../network/guestController';
  import { sanitizedToGameState } from '../network/stateAdapter';
  import { AIController } from '../ai/aiController';
  import { cloneGameState } from '../engine/wasmEngine';
  import { getActivePlayerIndex, isCurrentPlayerAI } from '../gameUtils';
  import GameLayout from './GameLayout.svelte';
  import DraftPhaseView from './DraftPhaseView.svelte';
  import ActionPhaseView from './ActionPhaseView.svelte';
  import CleanupPhaseView from './CleanupPhaseView.svelte';
  import CardList from './CardList.svelte';
  import OpponentBoardPanel from './OpponentBoardPanel.svelte';

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
  const aiController = role === 'host' ? new AIController() : null;
  let aiDraftKnowledge: Map<number, CardInstance[][]> = $state(new Map());

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

  let myPlayer = $derived(
    gameState && myPlayerIndex >= 0 ? gameState.players[myPlayerIndex] : null
  );

  // Setup host controller callbacks
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
      const aiPlayerIndices = gameState.aiPlayers
        .map((isAI, idx) => isAI ? idx : -1)
        .filter(idx => idx >= 0)
        .filter(idx => !hostController!['submittedDraftPicks'].has(idx));

      if (aiPlayerIndices.length === 0) return;

      // Record seen hands for all AI players
      const ds = gameState.phase.draftState;
      for (const playerIdx of aiPlayerIndices) {
        const hand = ds.hands[playerIdx];
        if (!aiDraftKnowledge.has(playerIdx)) {
          aiDraftKnowledge.set(playerIdx, []);
        }
        const playerSeenHands = aiDraftKnowledge.get(playerIdx)!;
        if (playerSeenHands.length <= ds.pickNumber) {
          playerSeenHands.push([...hand]);
        }
      }

      aiThinking = true;
      Promise.all(
        aiPlayerIndices.map(playerIdx => {
          const playerSeenHands = aiDraftKnowledge.get(playerIdx);
          return aiController!.getAIChoice(gameState!, playerIdx, 100000, playerSeenHands).then(choice => ({
            playerIdx,
            choice,
          }));
        })
      ).then(results => {
        aiThinking = false;
        for (const { playerIdx, choice } of results) {
          hostController?.applyAction(choice, playerIdx);
        }
      });
      return;
    }

    // For non-draft phases, use sequential AI turns
    if (!isCurrentPlayerAI(gameState)) return;

    const playerIdx = getActivePlayerIndex(gameState);
    aiThinking = true;
    const playerSeenHands = aiDraftKnowledge.get(playerIdx);

    aiController!.getAIChoice(gameState, playerIdx, 100000, playerSeenHands).then((choice) => {
      aiThinking = false;
      hostController?.applyAction(choice, playerIdx);
    });
  });

  // Reset aiDraftKnowledge when entering draft phase (host only)
  $effect(() => {
    if (role !== 'host' || !gameState) return;
    if (gameState.phase.type === 'draft' && gameState.phase.draftState.pickNumber === 0) {
      aiDraftKnowledge = new Map();
    }
  });
</script>

{#if gameState}
  <GameLayout {gameState} {activePlayerIndex} {aiThinking} {elapsedSeconds} {gameLog} onLeaveGame={onLeaveGame} sidebarPlayer={myPlayer}>
    {#if !isMyTurn && !aiThinking && gameState.phase.type === 'action'}
      <div class="waiting-banner">
        <div class="spinner"></div>
        <p>Waiting for {gameState.playerNames[activePlayerIndex] ?? 'other player'}...</p>
      </div>
      {#if myPlayer}
        <div class="readonly-cards">
          <div class="section">
            <h3>Your Workshop</h3>
            <CardList cards={myPlayer.workshopCards} />
          </div>
          <div class="section">
            <h3>Your Drafted Cards</h3>
            <CardList cards={myPlayer.draftedCards} />
          </div>
        </div>
        <div class="opponents-section">
          <h3>Other Players</h3>
          <div class="opponents-list">
            {#each gameState.players as player, i}
              {#if i !== myPlayerIndex}
                <OpponentBoardPanel {player} playerName={gameState.playerNames[i]} />
              {/if}
            {/each}
          </div>
        </div>
      {/if}
    {/if}

    {#if gameState.phase.type === 'draft'}
      <DraftPhaseView {gameState} onAction={handleAction} playerIndex={myPlayerIndex} selectable={!hasPicked} />
      {#if hasPicked}
        <div class="waiting-banner">
          <p>Waiting for other players to pick...</p>
        </div>
      {/if}
    {:else if gameState.phase.type === 'action' && isMyTurn}
      <ActionPhaseView {gameState} onAction={handleAction} onUndo={() => {}} undoAvailable={false} />
    {:else if gameState.phase.type === 'cleanup' && isMyTurn}
      <CleanupPhaseView {gameState} onAction={handleAction} />
    {:else if gameState.phase.type === 'cleanup' && !isMyTurn}
      <div class="waiting-banner">
        <div class="spinner"></div>
        <p>Waiting for {gameState.playerNames[activePlayerIndex] ?? 'other player'} to finish cleanup...</p>
      </div>
      {#if myPlayer}
        <div class="readonly-cards">
          <div class="section">
            <h3>Your Workshop</h3>
            <CardList cards={myPlayer.workshopCards} />
          </div>
        </div>
      {/if}
    {/if}
  </GameLayout>
{/if}

<style>
  .waiting-banner {
    display: flex;
    align-items: center;
    gap: 12px;
    justify-content: center;
    padding: 12px;
    color: #2a6bcf;
    font-weight: 600;
  }

  .waiting-banner p {
    font-size: 1rem;
    margin: 0;
  }

  .readonly-cards {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .section {
    border: 1px solid #ddd;
    border-radius: 8px;
    padding: 10px 12px;
    background: #fff;
    text-align: left;
  }

  .section h3 {
    font-size: 0.85rem;
    color: #4a3728;
    margin-bottom: 6px;
  }

  .opponents-section {
    border-top: 2px solid #e0e0e0;
    padding-top: 1rem;
  }

  .opponents-section h3 {
    font-size: 0.85rem;
    color: #888;
    margin-bottom: 8px;
  }

  .opponents-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 4px solid #e0d5c5;
    border-top-color: #e67e22;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
