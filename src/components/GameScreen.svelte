<script lang="ts">
  import type { GameState, Choice, Card } from '../data/types';
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

  let draftCardOrder: number[][] = $state(gameState.players.map(() => []));
  let drawExecutedForRound: number | null = $state(null);
  let aiThinking = $state(false);
  let aiError: string | null = $state(null);
  let gameLog: string[] = $state(initialGameLog);

  let undoStack: { gameState: GameState; logLength: number; accumulatorLength: number; draftCardOrder: number[][] }[] = $state([]);
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
    onGameUpdated(gameState, gameLog);
  }

  $effect(() => {
    if (gameState.phase.type !== 'action') {
      undoStack = [];
      undoPlayerIndex = null;
    }
  });

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
    gameLog.push(entry);
  }

  const aiController = new AIController();
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
      case 'selectGlass':
        return { type: 'destroyAndSelectGlass', card: destroyCard, glass: followUp.glass, payColor: followUp.payColor };
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

      // If still in draft and human's hand is empty (GlassKeepBoth), auto-advance
      if (gameState.phase.type === 'draft') {
        const ds2 = gameState.phase.draftState;
        if (ds2.hands[humanPlayerIndex].length === 0) {
          submittedDraftPicks.add(humanPlayerIndex);
          await resolveAIDraftPicks();
          return;
        }
      }
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
      <ActionPhaseView {gameState} onAction={handleAction} onUndo={performUndo} undoAvailable={undoStack.length > 0} {draftCardOrder} />
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
  .readonly-cards {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .section-box {
    border: 1px solid rgba(201, 168, 76, 0.4);
    border-radius: 6px;
    padding: 6px 8px;
    background: rgba(20, 15, 10, 0.6);
    text-align: left;
  }

  .section-box h3 {
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 0.75rem;
    color: #c9a84c;
    text-transform: uppercase;
    letter-spacing: 1px;
    margin-bottom: 4px;
  }

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

    .section-box {
      padding: 6px 8px;
    }

    .section-box h3 {
      font-size: 0.75rem;
    }
  }
</style>
