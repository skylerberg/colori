<script lang="ts">
  import type { GameState, CardInstance } from '../data/types';
  import type { ColoriChoice } from '../ai/coloriGame';
  import type { SanitizedGameState } from '../network/types';
  import type { HostController } from '../network/hostController';
  import type { GuestController } from '../network/guestController';
  import { sanitizedToGameState } from '../network/stateAdapter';
  import { AIController } from '../ai/aiController';
  import { cloneGameState } from '../ai/coloriGame';
  import PlayerStatus from './PlayerStatus.svelte';
  import DrawPhaseView from './DrawPhaseView.svelte';
  import DraftPhaseView from './DraftPhaseView.svelte';
  import ActionPhaseView from './ActionPhaseView.svelte';
  import GameLog from './GameLog.svelte';
  import ColorWheelDisplay from './ColorWheelDisplay.svelte';
  import GarmentDisplay from './GarmentDisplay.svelte';

  let { role, hostController, guestController, onGameOver, gameStartTime, onLeaveGame }: {
    role: 'host' | 'guest';
    hostController?: HostController;
    guestController?: GuestController;
    onGameOver: (gameState: GameState) => void;
    gameStartTime: number;
    onLeaveGame: () => void;
  } = $props();

  function handleLeaveGame() {
    if (confirm('Are you sure you want to leave this game? Your progress will be lost.')) {
      onLeaveGame();
    }
  }

  // Host state
  let hostGameState: GameState | null = $state(null);
  let hostGameLog: string[] = $state([]);

  // Guest state
  let guestSanitizedState: SanitizedGameState | null = $state(null);
  let guestGameLog: string[] = $state([]);

  // Shared state
  let aiThinking = $state(false);
  let showDrawPhase = $state(false);
  let drawExecutedForRound: number | null = $state(null);

  // Timer
  let elapsedSeconds = $state(0);

  function formatTime(seconds: number): string {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    if (h > 0) {
      return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
    }
    return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  }

  $effect(() => {
    elapsedSeconds = Math.floor((Date.now() - gameStartTime) / 1000);
    const interval = setInterval(() => {
      elapsedSeconds = Math.floor((Date.now() - gameStartTime) / 1000);
    }, 1000);
    return () => clearInterval(interval);
  });

  // AI controller (host only)
  const aiController = role === 'host' ? new AIController() : null;
  let seenHands: Map<number, CardInstance[][]> = $state(new Map());

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

  let currentPlayer = $derived(
    gameState && activePlayerIndex >= 0 ? gameState.players[activePlayerIndex] : null
  );

  let showSidebar = $derived(
    !aiThinking && !showDrawPhase && gameState !== null && isMyTurn &&
    (gameState.phase.type === 'draft' || gameState.phase.type === 'action')
  );

  let myPlayer = $derived(
    gameState && myPlayerIndex >= 0 ? gameState.players[myPlayerIndex] : null
  );

  function getActivePlayerIndex(gs: GameState): number {
    if (gs.phase.type === 'draft') {
      return gs.phase.draftState.currentPlayerIndex;
    }
    if (gs.phase.type === 'action') {
      return gs.phase.actionState.currentPlayerIndex;
    }
    return -1;
  }

  function isCurrentPlayerAI(gs: GameState): boolean {
    const idx = getActivePlayerIndex(gs);
    return idx >= 0 && gs.aiPlayers[idx];
  }

  // Setup host controller callbacks
  if (role === 'host' && hostController) {
    hostController.onGameStateUpdated = (state) => {
      hostGameState = state;
    };
    hostController.onLogUpdated = (log) => {
      hostGameLog = [...log];
    };
    hostController.onGameOver = (state) => {
      onGameOver(state);
    };

    // Initialize with current state
    hostGameState = hostController.getGameState();
    hostGameLog = [...hostController.getGameLog()];
  }

  // Setup guest controller callbacks
  if (role === 'guest' && guestController) {
    guestController.onStateUpdated = (state) => {
      guestSanitizedState = state;
      guestGameLog = [...guestController!.getGameLog()];
    };
    guestController.onGameOver = (state) => {
      guestSanitizedState = state;
      guestGameLog = [...guestController!.getGameLog()];
      onGameOver(sanitizedToGameState(state));
    };
  }

  // Handle draw phase for host (auto-show then continue)
  $effect(() => {
    if (role !== 'host' || !gameState) return;
    if (gameState.phase.type === 'draft' && drawExecutedForRound !== gameState.round) {
      drawExecutedForRound = gameState.round;
      // Show draw phase briefly if there are human players on host side
      if (!gameState.aiPlayers[0]) {
        showDrawPhase = true;
      }
    }
  });

  function handleDrawContinue() {
    showDrawPhase = false;
  }

  // Handle action from phase views
  function handleAction(choice: ColoriChoice) {
    if (role === 'host') {
      hostController?.applyHostAction(choice);
    } else {
      guestController?.sendAction(choice);
    }
  }

  // AI turn handling (host only)
  $effect(() => {
    if (role !== 'host' || !gameState || aiThinking) return;
    if (showDrawPhase) return;
    if (gameState.phase.type === 'gameOver') return;
    if (gameState.phase.type === 'draw') return;

    if (!isCurrentPlayerAI(gameState)) return;

    const playerIdx = getActivePlayerIndex(gameState);

    // Record seen hand for draft knowledge tracking
    if (gameState.phase.type === 'draft') {
      const ds = gameState.phase.draftState;
      const hand = ds.hands[playerIdx];
      if (!seenHands.has(playerIdx)) {
        seenHands.set(playerIdx, []);
      }
      const playerSeenHands = seenHands.get(playerIdx)!;
      if (playerSeenHands.length <= ds.pickNumber) {
        playerSeenHands.push([...hand]);
      }
    }

    aiThinking = true;
    const playerSeenHands = seenHands.get(playerIdx);

    aiController!.getAIChoice(gameState, playerIdx, 100000, playerSeenHands).then((choice) => {
      aiThinking = false;
      hostController?.applyAction(choice, playerIdx);
    });
  });

  // Reset seenHands when entering draft phase (host only)
  $effect(() => {
    if (role !== 'host' || !gameState) return;
    if (gameState.phase.type === 'draft' && gameState.phase.draftState.pickNumber === 0) {
      seenHands = new Map();
    }
  });
</script>

{#if gameState}
  <div class="game-screen">
    <div class="top-bar">
      <div class="top-bar-row">
        <div class="round-indicator">Round {gameState.round} of 8 &mdash; {formatTime(elapsedSeconds)}</div>
        <button class="leave-btn" onclick={handleLeaveGame}>Leave Game</button>
      </div>
      <div class="player-bar">
        {#each gameState.players as player, i}
          <PlayerStatus {player} active={i === activePlayerIndex} isAI={gameState.aiPlayers[i]} />
        {/each}
      </div>
    </div>

    <div class="game-body">
      {#if showSidebar && myPlayer}
        <aside class="left-sidebar">
          <div class="sidebar-section">
            <h3>Color Wheel</h3>
            <ColorWheelDisplay wheel={myPlayer.colorWheel} size={160} />
          </div>

          <div class="sidebar-section">
            <h3>Stored Materials</h3>
            <div class="material-counts">
              {#each Object.entries(myPlayer.materials) as [material, count]}
                <span class="material-count">{material}: {count}</span>
              {/each}
            </div>
          </div>
        </aside>
      {/if}

      <div class="main-column">
        <div class="phase-content">
          {#if aiThinking}
            <div class="ai-thinking">
              <div class="spinner"></div>
              <p>AI is thinking...</p>
            </div>
          {/if}

          {#if !isMyTurn && !aiThinking && !showDrawPhase && gameState.phase.type !== 'gameOver' && gameState.phase.type !== 'draw'}
            <div class="waiting-overlay">
              <div class="spinner"></div>
              <p>Waiting for {gameState.players[activePlayerIndex]?.name ?? 'other player'}...</p>
            </div>
          {/if}

          {#if showDrawPhase && gameState.phase.type === 'draft'}
            <DrawPhaseView {gameState} onContinue={handleDrawContinue} />
          {:else if gameState.phase.type === 'draft' && isMyTurn && !aiThinking}
            <DraftPhaseView {gameState} onAction={handleAction} />
          {:else if gameState.phase.type === 'action' && isMyTurn && !aiThinking}
            <ActionPhaseView {gameState} onAction={handleAction} onUndo={() => {}} undoAvailable={false} />
          {/if}
        </div>

        {#if gameLog.length > 0}
          <GameLog entries={gameLog} />
        {/if}
      </div>

      {#if showSidebar && myPlayer}
        <aside class="sidebar">
          <div class="sidebar-section">
            <GarmentDisplay garments={gameState.garmentDisplay} />
          </div>
        </aside>
      {/if}
    </div>
  </div>
{/if}

<style>
  .game-screen {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .top-bar {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-bottom: 8px;
    border-bottom: 2px solid #e0d5c5;
  }

  .top-bar-row {
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
  }

  .round-indicator {
    font-size: 0.85rem;
    font-weight: 600;
    color: #4a3728;
    text-align: center;
  }

  .leave-btn {
    position: absolute;
    right: 0;
    padding: 4px 12px;
    font-size: 0.75rem;
    background: #e74c3c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .leave-btn:hover {
    background: #c0392b;
  }

  .player-bar {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    justify-content: center;
    flex-wrap: wrap;
  }

  .game-body {
    display: flex;
    gap: 1rem;
  }

  .main-column {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .phase-content {
    min-height: 300px;
  }

  .left-sidebar {
    width: 250px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .sidebar {
    width: 250px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .sidebar-section {
    border: 1px solid #ddd;
    border-radius: 8px;
    padding: 10px 12px;
    background: #fff;
  }

  .sidebar-section h3 {
    font-size: 0.85rem;
    color: #4a3728;
    margin-bottom: 6px;
  }

  .material-counts {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.8rem;
    color: #8b6914;
  }

  .material-count {
    font-weight: 600;
  }

  .ai-thinking, .waiting-overlay {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    padding: 3rem 1rem;
    color: #666;
  }

  .ai-thinking p {
    font-size: 1.1rem;
    font-weight: 600;
    color: #e67e22;
  }

  .waiting-overlay p {
    font-size: 1.1rem;
    font-weight: 600;
    color: #2a6bcf;
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
