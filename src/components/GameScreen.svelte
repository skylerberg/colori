<script lang="ts">
  import type { GameState, CardInstance, Choice } from '../data/types';
  import { executeDrawPhase, applyChoice, simultaneousPick, advanceDraft, getChoiceLogMessage, cloneGameState } from '../engine/wasmEngine';
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
  let aiError: string | null = $state(null);
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

  // Simultaneous draft state
  let submittedDraftPicks: Set<number> = $state(new Set());
  let humanPlayerIndex = $derived(gameState.aiPlayers.findIndex(ai => !ai));

  // When the phase is 'draw', automatically execute the draw phase.
  $effect(() => {
    if (gameState.phase.type === 'draw' && drawExecutedForRound !== gameState.round) {
      drawExecutedForRound = gameState.round;
      addLog(`Round ${gameState.round} began`);
      executeDrawPhase(gameState);
      // Reset draft state for the new round
      aiDraftKnowledge = new Map();
      submittedDraftPicks = new Set();
      onGameUpdated(gameState, gameLog);
    }
  });

  let activePlayerIndex = $derived(
    gameState.phase.type === 'draft' ? humanPlayerIndex : getActivePlayerIndex(gameState)
  );
  let selectedPlayerIndex = $state(0);
  let selectedPlayer = $derived(gameState.players[selectedPlayerIndex]);
  let isViewingActiveHuman = $derived.by(() => {
    if (gameState.phase.type === 'draft') {
      return selectedPlayerIndex === humanPlayerIndex && !submittedDraftPicks.has(humanPlayerIndex);
    }
    return selectedPlayerIndex === activePlayerIndex && activePlayerIndex >= 0 && !gameState.aiPlayers[selectedPlayerIndex];
  });

  function selectPlayer(index: number) {
    selectedPlayerIndex = index;
  }

  // Unified action handler for both human UI and AI choices
  function handleAction(choice: Choice) {
    // Draft picks use simultaneous picking
    if (gameState.phase.type === 'draft' && choice.type === 'draftPick') {
      handleDraftPick(choice);
      return;
    }

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

  function handleDraftPick(choice: Choice) {
    gameLogAccumulator?.recordChoice(gameState, choice, humanPlayerIndex);
    const logMsg = getChoiceLogMessage(gameState, choice, humanPlayerIndex);
    if (logMsg) addLog(logMsg);
    simultaneousPick(gameState, humanPlayerIndex, (choice as { type: 'draftPick'; card: any }).card);
    submittedDraftPicks.add(humanPlayerIndex);
    onGameUpdated(gameState, gameLog);
    resolveAIDraftPicks();
  }

  async function resolveAIDraftPicks() {
    try {
      aiThinking = true;
      const ds = gameState.phase.type === 'draft' ? gameState.phase.draftState : null;
      if (!ds) return;

      const aiIndices = gameState.aiPlayers
        .map((isAI, idx) => isAI ? idx : -1)
        .filter(idx => idx >= 0)
        .filter(idx => !submittedDraftPicks.has(idx));

      // Record seen hands for AI draft knowledge
      for (const playerIdx of aiIndices) {
        const hand = ds.hands[playerIdx];
        if (!aiDraftKnowledge.has(playerIdx)) {
          aiDraftKnowledge.set(playerIdx, []);
        }
        const playerSeenHands = aiDraftKnowledge.get(playerIdx)!;
        if (playerSeenHands.length <= ds.pickNumber) {
          playerSeenHands.push([...hand]);
        }
      }

      // Resolve all AI picks in parallel
      const results = await Promise.all(
        aiIndices.map(async (playerIdx) => {
          const precomputed = aiController.waitForPrecomputedChoice(playerIdx, ds.pickNumber);
          if (precomputed !== null) {
            return { playerIdx, choice: await precomputed };
          }
          const playerSeenHands = aiDraftKnowledge.get(playerIdx);
          const choice = await aiController.getAIChoice(gameState, playerIdx, aiIterations[playerIdx], playerSeenHands);
          return { playerIdx, choice };
        })
      );

      // Apply all AI picks
      for (const { playerIdx, choice } of results) {
        gameLogAccumulator?.recordChoice(gameState, choice, playerIdx);
        const logMsg = getChoiceLogMessage(gameState, choice, playerIdx);
        if (logMsg) addLog(logMsg);
        simultaneousPick(gameState, playerIdx, (choice as { type: 'draftPick'; card: any }).card);
      }

      // Advance draft (rotate hands)
      advanceDraft(gameState);
      submittedDraftPicks = new Set();
      onGameUpdated(gameState, gameLog);
    } catch (e) {
      aiError = String(e);
    } finally {
      aiThinking = false;
    }
  }

  // Precompute all AI players' draft picks while human is deciding
  $effect(() => {
    if (gameState.phase.type !== 'draft') {
      aiController.cancelPrecomputation();
      return;
    }
    const ds = gameState.phase.draftState;
    if (aiThinking) return;
    if (submittedDraftPicks.has(humanPlayerIndex)) return; // human already picked

    const requests: PrecomputeRequest[] = [];

    for (let idx = 0; idx < gameState.players.length; idx++) {
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
    }

    if (requests.length > 0) {
      aiController.precomputeDraftPicks(requests);
    }
  });

  // Trigger AI turn when the active player is AI (action phase only)
  $effect(() => {
    if (aiThinking) return;
    if (gameState.phase.type === 'gameOver') return;
    if (gameState.phase.type === 'draw') return;
    if (gameState.phase.type === 'draft') return; // draft AI handled by resolveAIDraftPicks

    if (!isCurrentPlayerAI(gameState)) return;

    const playerIdx = getActivePlayerIndex(gameState);

    aiThinking = true;

    aiController.getAIChoice(gameState, playerIdx, aiIterations[playerIdx]).then((choice) => {
      aiThinking = false;
      handleAction(choice);
    }).catch((e) => {
      aiThinking = false;
      aiError = String(e);
    });
  });
</script>

<GameLayout {gameState} {activePlayerIndex} {aiThinking} {elapsedSeconds} {gameLog} onLeaveGame={onLeaveGame} sidebarPlayer={selectedPlayer ?? null} {selectedPlayerIndex} onSelectPlayer={selectPlayer} {aiError} onRetryAI={() => { aiError = null; }}>
  {#if gameState.phase.type === 'draft'}
    {#if isViewingActiveHuman}
      <DraftPhaseView {gameState} onAction={handleAction} playerIndex={humanPlayerIndex} selectable={!submittedDraftPicks.has(humanPlayerIndex)} />
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
