<script lang="ts">
  import type { GameState, CardInstance } from '../data/types';
  import { executeDrawPhase, confirmPass, applyChoice, getChoiceLogMessage, cloneGameState } from '../engine/wasmEngine';
  import { AIController, type PrecomputeRequest } from '../ai/aiController';
  import type { ColoriChoice } from '../data/types';
  import type { GameLogAccumulator } from '../gameLog';
  import PlayerStatus from './PlayerStatus.svelte';
  import DraftPhaseView from './DraftPhaseView.svelte';
  import ActionPhaseView from './ActionPhaseView.svelte';
  import GameLog from './GameLog.svelte';
  import ColorWheelDisplay from './ColorWheelDisplay.svelte';
  import BuyerDisplay from './BuyerDisplay.svelte';
  import CardList from './CardList.svelte';

  let { gameState, gameStartTime, onGameUpdated, initialGameLog, onLeaveGame, gameLogAccumulator }: {
    gameState: GameState;
    gameStartTime: number | null;
    onGameUpdated: (state: GameState, log: string[]) => void;
    initialGameLog: string[];
    onLeaveGame: () => void;
    gameLogAccumulator: GameLogAccumulator | null;
  } = $props();

  function handleLeaveGame() {
    if (confirm('Are you sure you want to leave this game? Your progress will be lost.')) {
      onLeaveGame();
    }
  }

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
    if (gameStartTime === null) return;
    elapsedSeconds = Math.floor((Date.now() - gameStartTime) / 1000);
    const interval = setInterval(() => {
      elapsedSeconds = Math.floor((Date.now() - gameStartTime) / 1000);
    }, 1000);
    return () => clearInterval(interval);
  });

  let drawExecutedForRound: number | null = $state(null);
  let aiThinking = $state(false);
  let gameLog: string[] = $state(initialGameLog);

  let undoStack: { gameState: GameState; logLength: number }[] = $state([]);
  let undoPlayerIndex: number | null = $state(null);

  function pushUndoSnapshot() {
    if (gameState.phase.type !== 'action') return;
    const currentIdx = gameState.phase.actionState.currentPlayerIndex;
    if (currentIdx !== undoPlayerIndex) {
      undoStack = [];
      undoPlayerIndex = currentIdx;
    }
    undoStack.push({
      gameState: cloneGameState(gameState),
      logLength: gameLog.length,
    });
  }

  function performUndo() {
    if (undoStack.length === 0) return;
    const snapshot = undoStack.pop()!;
    gameState = snapshot.gameState;
    gameLog = gameLog.slice(0, snapshot.logLength);
    onGameUpdated(gameState, gameLog);
  }

  $effect(() => {
    if (gameState.phase.type !== 'action') {
      undoStack = [];
      undoPlayerIndex = null;
    }
  });

  function addLog(entry: string) {
    gameLog.push(entry);
  }

  const aiController = new AIController();

  // Per-AI-player seen hands for draft knowledge tracking
  let seenHands: Map<number, CardInstance[][]> = $state(new Map());

  // When the phase is 'draw', automatically execute the draw phase.
  $effect(() => {
    if (gameState.phase.type === 'draw' && drawExecutedForRound !== gameState.round) {
      drawExecutedForRound = gameState.round;
      addLog(`Round ${gameState.round} began`);
      executeDrawPhase(gameState);
      // Reset seenHands for the new draft round
      seenHands = new Map();
      onGameUpdated(gameState, gameLog);
    }
  });

  let activePlayerIndex = $derived(getActivePlayerIndex(gameState));
  let selectedPlayerIndex = $state(0);
  let selectedPlayer = $derived(gameState.players[selectedPlayerIndex]);
  let isViewingActiveHuman = $derived(
    selectedPlayerIndex === activePlayerIndex && activePlayerIndex >= 0 && !gameState.aiPlayers[selectedPlayerIndex]
  );
  let showSidebar = $derived(
    selectedPlayer !== undefined &&
    (gameState.phase.type === 'draft' || gameState.phase.type === 'action')
  );

  function selectPlayer(index: number) {
    selectedPlayerIndex = index;
  }

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

  // Unified action handler for both human UI and AI choices
  function handleAction(choice: ColoriChoice) {
    const playerIdx = getActivePlayerIndex(gameState);
    const name = playerIdx >= 0 ? gameState.playerNames[playerIdx] : 'Unknown';

    // Push undo snapshot for action phase choices (except endTurn)
    if (gameState.phase.type === 'action' && choice.type !== 'endTurn') {
      pushUndoSnapshot();
    }

    gameLogAccumulator?.recordChoice(gameState, choice, playerIdx);
    const logMsg = getChoiceLogMessage(gameState, choice, playerIdx);
    if (logMsg) addLog(logMsg);
    applyChoice(gameState, choice);

    if (choice.type === 'draftPick' && gameState.phase.type === 'draft' && gameState.phase.draftState.waitingForPass) {
      confirmPass(gameState);
    }
    onGameUpdated(gameState, gameLog);
  }

  // Precompute all AI players' draft picks while human is deciding
  $effect(() => {
    if (gameState.phase.type !== 'draft') {
      aiController.cancelPrecomputation();
      return;
    }
    const ds = gameState.phase.draftState;
    if (ds.waitingForPass) return;
    if (aiThinking) return;

    const currentIdx = ds.currentPlayerIndex;
    if (gameState.aiPlayers[currentIdx]) return; // current player is AI, not human

    const numPlayers = gameState.players.length;
    const startingPlayer = (gameState.round - 1) % numPlayers;
    const requests: PrecomputeRequest[] = [];

    // Loop through players in turn order after the human, up to startingPlayer (end-of-round boundary)
    let idx = (currentIdx + 1) % numPlayers;
    while (idx !== startingPlayer) {
      if (gameState.aiPlayers[idx]) {
        const clone = cloneGameState(gameState);
        const cloneDs = (clone.phase as { type: 'draft'; draftState: typeof ds }).draftState;
        cloneDs.currentPlayerIndex = idx;
        cloneDs.waitingForPass = false;

        // Build seenHands snapshot for the AI player
        const aiSeenHands = seenHands.has(idx)
          ? [...seenHands.get(idx)!.map(h => [...h])]
          : [];
        const hand = ds.hands[idx];
        if (aiSeenHands.length <= ds.pickNumber) {
          aiSeenHands.push([...hand]);
        }

        requests.push({
          gameState: clone,
          playerIndex: idx,
          pickNumber: ds.pickNumber,
          iterations: 100000,
          seenHands: aiSeenHands,
        });
      }
      idx = (idx + 1) % numPlayers;
    }

    if (requests.length > 0) {
      aiController.precomputeDraftPicks(requests);
    }
  });

  // Trigger AI turn when the active player is AI
  $effect(() => {
    if (aiThinking) return;
    if (gameState.phase.type === 'gameOver') return;
    if (gameState.phase.type === 'draw') return;

    // Handle draft waitingForPass for AI
    if (gameState.phase.type === 'draft' && gameState.phase.draftState.waitingForPass) {
      const nextPlayer = gameState.phase.draftState.currentPlayerIndex;
      if (gameState.aiPlayers[nextPlayer]) {
        confirmPass(gameState);
        onGameUpdated(gameState, gameLog);
      }
      return;
    }

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

      // Try to use precomputed result for draft picks
      const precomputed = aiController.waitForPrecomputedChoice(playerIdx, ds.pickNumber);
      if (precomputed !== null) {
        aiThinking = true;
        precomputed.then((choice) => {
          aiThinking = false;
          handleAction(choice);
        });
        return;
      }
    }

    aiThinking = true;
    const playerSeenHands = seenHands.get(playerIdx);

    aiController.getAIChoice(gameState, playerIdx, 100000, playerSeenHands).then((choice) => {
      aiThinking = false;
      handleAction(choice);
    });
  });
</script>

<div class="game-screen">
  <div class="top-bar">
    <div class="top-bar-row">
      <div class="round-indicator">Round {gameState.round} &mdash; {formatTime(elapsedSeconds)}</div>
      <button class="leave-btn" onclick={handleLeaveGame}>Leave Game</button>
    </div>
    <div class="player-bar">
      {#each gameState.players as player, i}
        <PlayerStatus {player} playerName={gameState.playerNames[i]} active={i === activePlayerIndex} selected={i === selectedPlayerIndex} isAI={gameState.aiPlayers[i]} thinking={aiThinking && i === activePlayerIndex} onclick={() => selectPlayer(i)} />
      {/each}
    </div>
  </div>

  <div class="game-body">
    {#if showSidebar && selectedPlayer}
      <aside class="left-sidebar">
        <div class="sidebar-section">
          <h3>Color Wheel</h3>
          <ColorWheelDisplay wheel={selectedPlayer.colorWheel} size={160} />
        </div>

        <div class="sidebar-section">
          <h3>Stored Materials</h3>
          <div class="material-counts">
            {#each Object.entries(selectedPlayer.materials) as [material, count]}
              <span class="material-count">{material}: {count}</span>
            {/each}
          </div>
          {#if selectedPlayer.ducats > 0}
            <div class="ducats-count">Ducats: {selectedPlayer.ducats}</div>
          {/if}
        </div>
      </aside>
    {/if}

    <div class="main-column">
      <div class="phase-content">
        {#if gameState.phase.type === 'draft'}
          {#if isViewingActiveHuman}
            <DraftPhaseView {gameState} onAction={handleAction} onGameUpdated={() => onGameUpdated(gameState, gameLog)} />
          {:else}
            <div class="readonly-cards">
              <div class="section-box">
                <h3>Drafted Cards</h3>
                <CardList cards={selectedPlayer.draftedCards} />
              </div>
              <div class="section-box">
                <h3>Workshop</h3>
                <CardList cards={selectedPlayer.workshopCards} />
              </div>
            </div>
          {/if}
        {:else if gameState.phase.type === 'action'}
          {#if isViewingActiveHuman}
            <ActionPhaseView {gameState} onAction={handleAction} onUndo={performUndo} undoAvailable={undoStack.length > 0} />
          {:else}
            <div class="readonly-cards">
              <div class="section-box">
                <h3>Drafted Cards</h3>
                <CardList cards={selectedPlayer.draftedCards} />
              </div>
              <div class="section-box">
                <h3>Workshop</h3>
                <CardList cards={selectedPlayer.workshopCards} />
              </div>
              {#if selectedPlayer.completedBuyers.length > 0}
                <div class="section-box">
                  <h3>Completed Buyers</h3>
                  <CardList cards={selectedPlayer.completedBuyers} />
                </div>
              {/if}
            </div>
          {/if}
        {/if}
      </div>

      {#if gameLog.length > 0}
        <GameLog entries={gameLog} />
      {/if}
    </div>

    {#if showSidebar && selectedPlayer}
      <aside class="sidebar">
        <div class="sidebar-section">
          <BuyerDisplay buyers={gameState.buyerDisplay} />
        </div>
      </aside>
    {/if}
  </div>
</div>

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

  .ducats-count {
    font-size: 0.8rem;
    color: #d4a017;
    font-weight: 600;
    margin-top: 4px;
  }

  .readonly-cards {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-top: 1rem;
  }

  .section-box {
    border: 1px solid #ddd;
    border-radius: 8px;
    padding: 10px 12px;
    background: #fff;
    text-align: left;
  }

  .section-box h3 {
    font-size: 0.85rem;
    color: #4a3728;
    margin-bottom: 6px;
  }

</style>
