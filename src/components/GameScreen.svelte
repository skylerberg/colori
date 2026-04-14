<script lang="ts">
  import type { GameState, Choice, Card, CardInstance } from '../data/types';
  import { executeDrawPhase, applyChoice, simultaneousPick, advanceDraft, getChoiceLogMessage, cloneGameState } from '../engine/wasmEngine';
  import { getCardData } from '../data/cards';
  import { AIController, type PrecomputeRequest } from '../ai/aiController';
  import type { GameLogAccumulator } from '../gameLog';
  import { getActivePlayerIndex, isCurrentPlayerAI, orderByDraftOrder } from '../gameUtils';
  import GameLayout from './GameLayout.svelte';
  import DraftPhaseView from './DraftPhaseView.svelte';
  import ActionPhaseView from './ActionPhaseView.svelte';
  import CardList from './CardList.svelte';
  import { startTutorial, cancelTutorial } from '../tutorial/tutorial';

  let { gameState, gameStartTime, onGameUpdated, initialGameLog, onLeaveGame, gameLogAccumulator, aiIterations, aiStyle }: {
    gameState: GameState;
    gameStartTime: number | null;
    onGameUpdated: (state: GameState, log: string[]) => void;
    initialGameLog: string[];
    onLeaveGame: () => void;
    gameLogAccumulator: GameLogAccumulator | null;
    aiIterations: number[];
    aiStyle: string;
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

  // svelte-ignore state_referenced_locally
  let draftCardOrder: number[][] = $state(gameState.players.map(() => []));
  let drawExecutedForRound: number | null = $state(null);
  let aiThinking = $state(false);
  let aiError: string | null = $state(null);
  // svelte-ignore state_referenced_locally
  let gameLog: string[] = $state(initialGameLog);

  // Cards the human has staged as "moved to draft pool" but the engine still
  // holds in the workshop. The UI renders these in the drafted row and filters
  // them out of every workshop-facing prompt, so the player's mental model
  // (moved, not yet destroyed) matches what they see — while the engine sees
  // a single atomic DestroyCards choice pair (skip + later deferred destroy)
  // that produces the same end state as the AI's compound choice.
  let deferredMoves: CardInstance[] = $state([]);

  let undoStack: {
    gameState: GameState;
    logLength: number;
    accumulatorLength: number;
    draftCardOrder: number[][];
    deferredMoves: CardInstance[];
  }[] = $state([]);
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
      accumulatorLength: gameLogAccumulator?.getLog().entries.length ?? 0,
      draftCardOrder: draftCardOrder.map(order => [...order]),
      deferredMoves: [...deferredMoves],
    });
  }

  function performUndo() {
    if (undoStack.length === 0) return;
    const snapshot = undoStack.pop()!;
    pendingDestroyCard = null;
    gameState = snapshot.gameState;
    gameLog = gameLog.slice(0, snapshot.logLength);
    gameLogAccumulator?.truncateEntries(snapshot.accumulatorLength);
    draftCardOrder = snapshot.draftCardOrder;
    deferredMoves = snapshot.deferredMoves;
    onGameUpdated(gameState, gameLog);
  }

  $effect(() => {
    if (gameState.phase.type !== 'action') {
      undoStack = [];
      undoPlayerIndex = null;
      deferredMoves = [];
    }
  });

  // Clear deferred moves on turn transition — end-of-turn cleanup in the
  // engine sweeps both workshop and draft pool into discard, so there's
  // nothing for the next player to see.
  let currentActionPlayerIdx = $derived(
    gameState.phase.type === 'action' ? gameState.phase.actionState.currentPlayerIndex : -1
  );
  $effect(() => {
    currentActionPlayerIdx;
    deferredMoves = [];
  });

  function stageDeferredMove(ci: CardInstance) {
    handleAction({ type: 'deferredMoveToDraft', card: ci.card });
    deferredMoves = [...deferredMoves, ci];
  }

  function commitDeferredDestroy(ci: CardInstance) {
    handleAction({ type: 'destroyWorkshopCardDeferred', card: ci.card });
    deferredMoves = deferredMoves.filter(c => c.instanceId !== ci.instanceId);
  }

  // Start tutorial on first mount (checks localStorage internally)
  let tutorialStarted = false;
  $effect(() => {
    if (!tutorialStarted && gameState.phase.type === 'draft') {
      tutorialStarted = true;
      startTutorial();
    }
  });

  // Clean up tutorial on unmount
  $effect(() => {
    return () => cancelTutorial();
  });

  function addLog(entry: string) {
    for (const line of entry.split('\n')) {
      gameLog.push(line);
    }
  }

  const aiController = new AIController();
  // svelte-ignore state_referenced_locally
  aiController.aiStyle = aiStyle;

  // Simultaneous draft state
  let submittedDraftPicks: Set<number> = $state(new Set());
  let humanPlayerIndex = $derived(gameState.aiPlayers.findIndex(ai => !ai));

  // When the phase is 'draw', automatically execute the draw phase.
  $effect(() => {
    if (gameState.phase.type === 'draw' && drawExecutedForRound !== gameState.round) {
      drawExecutedForRound = gameState.round;
      addLog(`Round ${gameState.round} began`);
      const drawPhaseDraws = executeDrawPhase(gameState);
      gameLogAccumulator?.recordDrawPhaseDraws(drawPhaseDraws);
      // Reset draft state for the new round
      submittedDraftPicks = new Set();
      draftCardOrder = gameState.players.map(() => []);
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

  // ── Compound choice tracking ──
  // When a human destroys a drafted card with a compound-able ability,
  // we remember it so the follow-up choice can be combined into a single
  // compound choice in the game log. The UI flow is unchanged.
  let pendingDestroyCard: Card | null = $state(null);

  const COMPOUND_ABILITY_TYPES = new Set(['mixColors', 'sell', 'workshop', 'destroyCards']);

  function isCompoundableDestroy(choice: Choice): choice is { type: 'destroyDraftedCard'; card: Card } {
    if (choice.type !== 'destroyDraftedCard') return false;
    const data = getCardData(choice.card as Card);
    return data != null && data.kind !== 'sellCard' && COMPOUND_ABILITY_TYPES.has(data.ability.type);
  }

  function buildCompoundChoice(destroyCard: Card, followUp: Choice): Choice | null {
    switch (followUp.type) {
      case 'mixAll':
        return { type: 'destroyAndMix', card: destroyCard, mixes: followUp.mixes };
      case 'selectSellCard':
        return { type: 'destroyAndSell', card: destroyCard, sellCard: followUp.sellCard };
      case 'workshop':
        return { type: 'destroyAndWorkshop', card: destroyCard, workshopCards: followUp.cardTypes };
      case 'skipWorkshop':
        return { type: 'destroyAndWorkshop', card: destroyCard, workshopCards: [] };
      case 'destroyDrawnCards':
        return { type: 'destroyAndDestroyCards', card: destroyCard, target: followUp.card };
      default:
        return null;
    }
  }

  // Unified action handler for both human UI and AI choices
  function handleAction(choice: Choice) {
    // Draft picks use simultaneous picking
    if (gameState.phase.type === 'draft' && choice.type === 'draftPick') {
      handleDraftPick(choice);
      return;
    }

    const playerIdx = getActivePlayerIndex(gameState);

    // Check if this follow-up can be combined with a pending destroy
    if (pendingDestroyCard !== null) {
      const compound = buildCompoundChoice(pendingDestroyCard, choice);
      if (compound !== null) {
        pendingDestroyCard = null;
        // Replace the destroy log entry + this follow-up with the compound choice
        gameLogAccumulator?.replaceLastEntry(gameState, compound, playerIdx);
        const draws = applyChoice(gameState, choice);
        gameLogAccumulator?.attachDrawsToLastEntry(draws);
        // Replace log message: remove the destroy message and add compound message
        const compoundLogMsg = getChoiceLogMessage(gameState, compound, playerIdx);
        // Pop the destroy log message we added earlier
        gameLog.pop();
        if (compoundLogMsg) addLog(compoundLogMsg);
        onGameUpdated(gameState, gameLog);
        return;
      }
      // Follow-up wasn't compound-able (e.g. gainPrimary from a workshopped action card)
      pendingDestroyCard = null;
    }

    // Push undo snapshot for action phase choices (except endTurn)
    if (gameState.phase.type === 'action' && choice.type !== 'endTurn') {
      pushUndoSnapshot();
    }

    // Check if this is a destroy that starts a compound sequence
    if (isCompoundableDestroy(choice)) {
      pendingDestroyCard = choice.card as Card;
    }

    const logMsg = getChoiceLogMessage(gameState, choice, playerIdx);
    gameLogAccumulator?.recordChoice(gameState, choice, playerIdx);
    const draws = applyChoice(gameState, choice);
    gameLogAccumulator?.attachDrawsToLastEntry(draws);
    if (logMsg) addLog(logMsg);

    onGameUpdated(gameState, gameLog);
  }

  function handleDraftPick(choice: Choice) {
    gameLogAccumulator?.recordChoice(gameState, choice, humanPlayerIndex);
    const logMsg = getChoiceLogMessage(gameState, choice, humanPlayerIndex);
    if (logMsg) addLog(logMsg);
    simultaneousPick(gameState, humanPlayerIndex, (choice as { type: 'draftPick'; card: any }).card);
    // Track the newly drafted card's order
    const knownIds = new Set(draftCardOrder[humanPlayerIndex]);
    const newCard = gameState.players[humanPlayerIndex].draftedCards.find(
      c => !knownIds.has(c.instanceId)
    );
    if (newCard) {
      draftCardOrder[humanPlayerIndex] = [...draftCardOrder[humanPlayerIndex], newCard.instanceId];
    }
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
        .filter(idx => !submittedDraftPicks.has(idx))
        .filter(idx => ds.hands[idx].length > 0);

      // Resolve all AI picks in parallel
      const results = await Promise.all(
        aiIndices.map(async (playerIdx) => {
          const precomputed = aiController.waitForPrecomputedChoice(playerIdx, ds.pickNumber);
          if (precomputed !== null) {
            return { playerIdx, choice: await precomputed };
          }
          const choice = await aiController.getAIChoice(gameState, playerIdx, aiIterations[playerIdx]);
          return { playerIdx, choice };
        })
      );

      // Apply all AI picks
      for (const { playerIdx, choice } of results) {
        gameLogAccumulator?.recordChoice(gameState, choice, playerIdx);
        const logMsg = getChoiceLogMessage(gameState, choice, playerIdx);
        if (logMsg) addLog(logMsg);
        simultaneousPick(gameState, playerIdx, (choice as { type: 'draftPick'; card: any }).card);
        // Track the newly drafted card's order
        const knownIds = new Set(draftCardOrder[playerIdx]);
        const newCard = gameState.players[playerIdx].draftedCards.find(
          c => !knownIds.has(c.instanceId)
        );
        if (newCard) {
          draftCardOrder[playerIdx] = [...draftCardOrder[playerIdx], newCard.instanceId];
        }
      }

      // Advance draft (rotate hands)
      advanceDraft(gameState);
      submittedDraftPicks = new Set();
      onGameUpdated(gameState, gameLog);
    } catch (e) {
      console.error('AI draft error:', e);
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
      if (gameState.aiPlayers[idx] && ds.hands[idx].length > 0) {
        const clone = cloneGameState(gameState);
        const cloneDs = (clone.phase as { type: 'draft'; draftState: typeof ds }).draftState;
        cloneDs.currentPlayerIndex = idx;

        requests.push({
          gameState: clone,
          playerIndex: idx,
          pickNumber: ds.pickNumber,
          iterations: aiIterations[idx],
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
      console.error('AI action error:', e);
      aiThinking = false;
      aiError = String(e);
    });
  });
</script>

<GameLayout {gameState} {activePlayerIndex} {aiThinking} {elapsedSeconds} {gameLog} onLeaveGame={onLeaveGame} {selectedPlayerIndex} onSelectPlayer={selectPlayer} {aiError} onRetryAI={() => { aiError = null; }} hidePlayerCards={isViewingActiveHuman && gameState.phase.type === 'action'} {draftCardOrder}>
  {#if gameState.phase.type === 'draft'}
    {#if isViewingActiveHuman}
      <DraftPhaseView {gameState} onAction={handleAction} playerIndex={humanPlayerIndex} selectable={!submittedDraftPicks.has(humanPlayerIndex)} />
    {:else}
      <div class="waiting-indicator">
        <span class="waiting-spinner"></span>
        <span class="waiting-text">{aiThinking && selectedPlayerIndex === activePlayerIndex ? 'Thinking...' : 'Waiting...'}</span>
      </div>
    {/if}
  {:else if gameState.phase.type === 'action'}
    <div style:display={isViewingActiveHuman ? 'contents' : 'none'}>
      <ActionPhaseView {gameState} onAction={handleAction} onUndo={performUndo} undoAvailable={undoStack.length > 0} {draftCardOrder} {deferredMoves} onStageDeferredMove={stageDeferredMove} onCommitDeferredDestroy={commitDeferredDestroy} />
    </div>
    {#if !isViewingActiveHuman}
      <div class="waiting-indicator">
        <span class="waiting-spinner"></span>
        <span class="waiting-text">{aiThinking && selectedPlayerIndex === activePlayerIndex ? 'Thinking...' : 'Waiting...'}</span>
      </div>
    {/if}
  {/if}
</GameLayout>

<style>
  .waiting-indicator {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 1rem 0;
  }

  .waiting-text {
    font-family: 'Cinzel', serif;
    font-size: 0.85rem;
    color: rgba(245, 237, 224, 0.6);
    letter-spacing: 0.1em;
  }

  .waiting-spinner {
    display: inline-block;
    width: 14px;
    height: 14px;
    border: 2px solid rgba(201, 168, 76, 0.3);
    border-top-color: #c9a84c;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* ===== RESPONSIVE OVERRIDES (mobile-first) ===== */

  @media (min-width: 768px) {
    .waiting-indicator {
      padding: 2rem 0;
      gap: 10px;
    }

    .waiting-text {
      font-size: 1rem;
    }

    .waiting-spinner {
      width: 16px;
      height: 16px;
    }
  }
</style>
