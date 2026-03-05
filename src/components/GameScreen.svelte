<script lang="ts">
  import type { GameState, CardInstance, Choice } from '../data/types';
  import { executeDrawPhase, applyChoice, getChoiceLogMessage, cloneGameState } from '../engine/wasmEngine';
  import { AIController, type PrecomputeRequest } from '../ai/aiController';
  import type { GameLogAccumulator } from '../gameLog';
  import { getActivePlayerIndex, isCurrentPlayerAI } from '../gameUtils';
  import GameLayout from './GameLayout.svelte';
  import DraftPhaseView from './DraftPhaseView.svelte';
  import ActionPhaseView from './ActionPhaseView.svelte';
  import CardList from './CardList.svelte';

  let { gameState, gameStartTime, onGameUpdated, initialGameLog, onLeaveGame, gameLogAccumulator, aiIterations }: {
    gameState: GameState;
    gameStartTime: number | null;
    onGameUpdated: (state: GameState, log: string[]) => void;
    initialGameLog: string[];
    onLeaveGame: () => void;
    gameLogAccumulator: GameLogAccumulator | null;
    aiIterations: number[];
  } = $props();

  let elapsedSeconds = $state(0);

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
  let aiDraftKnowledge: Map<number, CardInstance[][]> = $state(new Map());

  // When the phase is 'draw', automatically execute the draw phase.
  $effect(() => {
    if (gameState.phase.type === 'draw' && drawExecutedForRound !== gameState.round) {
      drawExecutedForRound = gameState.round;
      addLog(`Round ${gameState.round} began`);
      executeDrawPhase(gameState);
      // Reset aiDraftKnowledge for the new draft round
      aiDraftKnowledge = new Map();
      onGameUpdated(gameState, gameLog);
    }
  });

  let activePlayerIndex = $derived(getActivePlayerIndex(gameState));
  let selectedPlayerIndex = $state(0);
  let selectedPlayer = $derived(gameState.players[selectedPlayerIndex]);
  let isViewingActiveHuman = $derived(
    selectedPlayerIndex === activePlayerIndex && activePlayerIndex >= 0 && !gameState.aiPlayers[selectedPlayerIndex]
  );

  function selectPlayer(index: number) {
    selectedPlayerIndex = index;
  }

  // Unified action handler for both human UI and AI choices
  function handleAction(choice: Choice) {
    const playerIdx = getActivePlayerIndex(gameState);

    // Push undo snapshot for action phase choices (except endTurn)
    if (gameState.phase.type === 'action' && choice.type !== 'endTurn') {
      pushUndoSnapshot();
    }

    gameLogAccumulator?.recordChoice(gameState, choice, playerIdx);
    const logMsg = getChoiceLogMessage(gameState, choice, playerIdx);
    if (logMsg) addLog(logMsg);
    applyChoice(gameState, choice);

    onGameUpdated(gameState, gameLog);
  }

  // Precompute all AI players' draft picks while human is deciding
  $effect(() => {
    if (gameState.phase.type !== 'draft') {
      aiController.cancelPrecomputation();
      return;
    }
    const ds = gameState.phase.draftState;
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

        // Build aiDraftKnowledge snapshot for the AI player
        const aiSeenHands = aiDraftKnowledge.has(idx)
          ? [...aiDraftKnowledge.get(idx)!.map(h => [...h])]
          : [];
        const hand = ds.hands[idx];
        if (aiSeenHands.length <= ds.pickNumber) {
          aiSeenHands.push([...hand]);
        }

        requests.push({
          gameState: clone,
          playerIndex: idx,
          pickNumber: ds.pickNumber,
          iterations: aiIterations[idx],
          aiDraftKnowledge: aiSeenHands,
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

    if (!isCurrentPlayerAI(gameState)) return;

    const playerIdx = getActivePlayerIndex(gameState);

    // Record seen hand for draft knowledge tracking
    if (gameState.phase.type === 'draft') {
      const ds = gameState.phase.draftState;
      const hand = ds.hands[playerIdx];
      if (!aiDraftKnowledge.has(playerIdx)) {
        aiDraftKnowledge.set(playerIdx, []);
      }
      const playerSeenHands = aiDraftKnowledge.get(playerIdx)!;
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
    const playerSeenHands = aiDraftKnowledge.get(playerIdx);

    aiController.getAIChoice(gameState, playerIdx, aiIterations[playerIdx], playerSeenHands).then((choice) => {
      aiThinking = false;
      handleAction(choice);
    });
  });
</script>

<GameLayout {gameState} {activePlayerIndex} {aiThinking} {elapsedSeconds} {gameLog} onLeaveGame={onLeaveGame} sidebarPlayer={selectedPlayer ?? null} {selectedPlayerIndex} onSelectPlayer={selectPlayer} onAction={handleAction}>
  {#if gameState.phase.type === 'draft'}
    {#if isViewingActiveHuman}
      <DraftPhaseView {gameState} onAction={handleAction} />
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
</GameLayout>

<style>
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
