<script lang="ts">
  import type { GameState, CardInstance } from '../data/types';
  import { executeDrawPhase } from '../engine/drawPhase';
  import { playerPick, confirmPass } from '../engine/draftPhase';
  import {
    destroyDraftedCard, endPlayerTurn, resolveMakeMaterials,
    resolveMixColors, skipMix, resolveDestroyCards,
    resolveSelectGarment,
  } from '../engine/actionPhase';
  import { AIController } from '../ai/aiController';
  import type { ColoriChoice } from '../ai/coloriGame';
  import { cloneGameState } from '../ai/coloriGame';
  import PlayerStatus from './PlayerStatus.svelte';
  import DrawPhaseView from './DrawPhaseView.svelte';
  import DraftPhaseView from './DraftPhaseView.svelte';
  import ActionPhaseView from './ActionPhaseView.svelte';
  import GameLog from './GameLog.svelte';
  import ColorWheelDisplay from './ColorWheelDisplay.svelte';
  import GarmentDisplay from './GarmentDisplay.svelte';
  import { mixResult } from '../data/colors';

  let { gameState, gameStartTime, onGameUpdated }: {
    gameState: GameState;
    gameStartTime: number | null;
    onGameUpdated: (state: GameState) => void;
  } = $props();

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
  let showDrawPhase = $state(false);
  let aiThinking = $state(false);
  let gameLog: string[] = $state([]);

  let undoStack: { gameState: GameState; logLength: number }[] = $state([]);
  let undoPlayerIndex: number | null = $state(null);

  function pushUndoSnapshot() {
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
    onGameUpdated(gameState);
  }

  $effect(() => {
    if (gameState.phase.type !== 'action') {
      undoStack = [];
      undoPlayerIndex = null;
      return;
    }
    const currentIdx = gameState.phase.actionState.currentPlayerIndex;
    if (currentIdx !== undoPlayerIndex) {
      undoStack = [];
      undoPlayerIndex = currentIdx;
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
      // Check if any human players exist to show draw phase
      const hasHuman = gameState.aiPlayers.some(ai => !ai);
      if (hasHuman) {
        showDrawPhase = true;
      }
      onGameUpdated(gameState);
    }
  });

  function handleDrawContinue() {
    showDrawPhase = false;
    onGameUpdated(gameState);
  }

  function handleGameUpdated() {
    onGameUpdated(gameState);
  }

  let activePlayerIndex = $derived(getActivePlayerIndex(gameState));
  let currentPlayer = $derived(activePlayerIndex >= 0 ? gameState.players[activePlayerIndex] : null);
  let showSidebar = $derived(
    !aiThinking && !showDrawPhase && currentPlayer !== null &&
    (gameState.phase.type === 'draft' || gameState.phase.type === 'action') &&
    !isCurrentPlayerAI(gameState)
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

  // Apply an AI choice to the game state
  function applyAIChoice(choice: ColoriChoice) {
    const playerIdx = getActivePlayerIndex(gameState);
    const name = playerIdx >= 0 ? gameState.players[playerIdx].name : 'Unknown';
    switch (choice.type) {
      case 'draftPick':
        playerPick(gameState, choice.cardInstanceId);
        if (gameState.phase.type === 'draft' && gameState.phase.draftState.waitingForPass) {
          confirmPass(gameState);
        }
        break;
      case 'destroyDraftedCard': {
        const card = gameState.players[playerIdx].draftedCards.find(c => c.instanceId === choice.cardInstanceId);
        addLog(`${name} destroyed ${card && 'name' in card.card ? card.card.name : 'a card'} from drafted cards`);
        destroyDraftedCard(gameState, choice.cardInstanceId);
        break;
      }
      case 'endTurn':
        addLog(`${name} ended their turn`);
        endPlayerTurn(gameState);
        if (gameState.phase.type === 'draw') {
          // Will be handled by the draw phase effect
        }
        break;
      case 'makeMaterials': {
        const cardNames = choice.cardInstanceIds.map(id => {
          const c = gameState.players[playerIdx].drawnCards.find(c => c.instanceId === id);
          return c && 'name' in c.card ? c.card.name : 'a card';
        });
        addLog(`${name} stored materials from ${cardNames.join(', ')}`);
        resolveMakeMaterials(gameState, choice.cardInstanceIds);
        break;
      }
      case 'destroyDrawnCards': {
        const cardNames = choice.cardInstanceIds.map(id => {
          const c = gameState.players[playerIdx].drawnCards.find(c => c.instanceId === id);
          return c && 'name' in c.card ? c.card.name : 'a card';
        });
        addLog(`${name} destroyed ${cardNames.join(', ')} from drawn cards`);
        resolveDestroyCards(gameState, choice.cardInstanceIds);
        break;
      }
      case 'mix': {
        const result = mixResult(choice.colorA, choice.colorB);
        addLog(`${name} mixed ${choice.colorA} + ${choice.colorB} to make ${result}`);
        resolveMixColors(gameState, choice.colorA, choice.colorB);
        break;
      }
      case 'skipMix':
        addLog(`${name} skipped remaining mixes`);
        skipMix(gameState);
        break;
      case 'selectGarment': {
        const garment = gameState.garmentDisplay.find(g => g.instanceId === choice.garmentInstanceId);
        addLog(`${name} completed a ${garment?.card.stars ?? '?'}-star garment`);
        resolveSelectGarment(gameState, choice.garmentInstanceId);
        break;
      }
    }
    onGameUpdated(gameState);
  }

  // Precompute next AI player's draft pick while human is deciding
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
    const nextIdx = (currentIdx + 1) % numPlayers;
    const startingPlayer = (gameState.round - 1) % numPlayers;
    if (nextIdx === startingPlayer) return; // human is last picker this round, hands will rotate
    if (!gameState.aiPlayers[nextIdx]) return; // next player is not AI

    const clone = cloneGameState(gameState);
    const cloneDs = (clone.phase as { type: 'draft'; draftState: typeof ds }).draftState;
    cloneDs.currentPlayerIndex = nextIdx;
    cloneDs.waitingForPass = false;

    // Build seenHands snapshot for the AI player
    const aiSeenHands = seenHands.has(nextIdx)
      ? [...seenHands.get(nextIdx)!.map(h => [...h])]
      : [];
    const nextHand = ds.hands[nextIdx];
    if (aiSeenHands.length <= ds.pickNumber) {
      aiSeenHands.push([...nextHand]);
    }

    aiController.precomputeDraftPick(clone, nextIdx, ds.pickNumber, 100000, aiSeenHands);
  });

  // Trigger AI turn when the active player is AI
  $effect(() => {
    if (aiThinking) return;
    if (showDrawPhase) return;
    if (gameState.phase.type === 'gameOver') return;
    if (gameState.phase.type === 'draw') return;

    // Handle draft waitingForPass for AI
    if (gameState.phase.type === 'draft' && gameState.phase.draftState.waitingForPass) {
      const nextPlayer = gameState.phase.draftState.currentPlayerIndex;
      if (gameState.aiPlayers[nextPlayer]) {
        confirmPass(gameState);
        onGameUpdated(gameState);
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
          applyAIChoice(choice);
        });
        return;
      }
    }

    aiThinking = true;
    const playerSeenHands = seenHands.get(playerIdx);

    aiController.getAIChoice(gameState, playerIdx, 100000, playerSeenHands).then((choice) => {
      aiThinking = false;
      applyAIChoice(choice);
    });
  });
</script>

<div class="game-screen">
  <div class="top-bar">
    <div class="round-indicator">Round {gameState.round} of 8 &mdash; {formatTime(elapsedSeconds)}</div>
    <div class="player-bar">
      {#each gameState.players as player, i}
        <PlayerStatus {player} active={i === activePlayerIndex} isAI={gameState.aiPlayers[i]} />
      {/each}
    </div>
  </div>

  <div class="game-body">
    <div class="main-column">
      <div class="phase-content">
        {#if aiThinking}
          <div class="ai-thinking">
            <div class="spinner"></div>
            <p>AI is thinking...</p>
          </div>
        {/if}

        {#if showDrawPhase && gameState.phase.type === 'draft'}
          <DrawPhaseView {gameState} onContinue={handleDrawContinue} />
        {:else if gameState.phase.type === 'draft' && !isCurrentPlayerAI(gameState)}
          <DraftPhaseView {gameState} onGameUpdated={handleGameUpdated} />
        {:else if gameState.phase.type === 'action' && !isCurrentPlayerAI(gameState)}
          <ActionPhaseView {gameState} onGameUpdated={handleGameUpdated} onLog={addLog} onBeforeAction={pushUndoSnapshot} onUndo={performUndo} undoAvailable={undoStack.length > 0} />
        {/if}
      </div>

      {#if gameLog.length > 0}
        <GameLog entries={gameLog} />
      {/if}
    </div>

    {#if showSidebar && currentPlayer}
      <aside class="sidebar">
        <div class="sidebar-section">
          <h3>Color Wheel</h3>
          <ColorWheelDisplay wheel={currentPlayer.colorWheel} size={160} />
        </div>

        <div class="sidebar-section">
          <h3>Stored Fabrics</h3>
          <div class="fabric-counts">
            {#each Object.entries(currentPlayer.fabrics) as [fabric, count]}
              <span class="fabric-count">{fabric}: {count}</span>
            {/each}
          </div>
        </div>

        <div class="sidebar-section">
          <GarmentDisplay garments={gameState.garmentDisplay} />
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

  .round-indicator {
    font-size: 0.85rem;
    font-weight: 600;
    color: #4a3728;
    text-align: center;
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

  .fabric-counts {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.8rem;
    color: #8b6914;
  }

  .fabric-count {
    font-weight: 600;
  }

  .ai-thinking {
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
