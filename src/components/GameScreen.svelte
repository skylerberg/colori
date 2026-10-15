<script lang="ts">
  import type { GameState, Choice, Card, Ability, DrawEvent, StructuredLogEntry } from '../data/types';
  import { executeDrawPhase, applyChoice, simultaneousPick, advanceDraft, getChoiceLogMessage, cloneGameState } from '../engine/wasmEngine';
  import { getCardData } from '../data/cards';
  import { AIController, type DraftPicker } from '../ai/aiController';
  import { AIActionTurns } from '../ai/actionTurns';
  import type { GameLogAccumulator } from '../gameLog';
  import { actingPlayers, isTakingActionTurn } from '../gameUtils';
  import GameLayout from './GameLayout.svelte';
  import BuyersPhaseView from './BuyersPhaseView.svelte';
  import DraftPhaseView from './DraftPhaseView.svelte';
  import ActionPhaseView from './ActionPhaseView.svelte';
  import { startTutorial, cancelTutorial } from '../tutorial/tutorial';
  import { untrack } from 'svelte';

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
  // A buyers claim or draft pick being searched for.
  let aiThinking = $state(false);
  let aiError: string | null = $state(null);
  // svelte-ignore state_referenced_locally
  let gameLog: string[] = $state(initialGameLog);

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
  const aiActionTurns = new AIActionTurns(aiController);
  let thinkingInActionPhase: number[] = $state([]);

  $effect(() => {
    return () => {
      aiActionTurns.cancel();
      aiController.terminate();
    };
  });

  let humanPicked = $state(false);
  let humanPlayerIndex = $derived(gameState.aiPlayers.findIndex(ai => !ai));

  // When the phase is 'draw', automatically execute the draw phase.
  $effect(() => {
    if (gameState.phase.type === 'draw' && drawExecutedForRound !== gameState.round) {
      drawExecutedForRound = gameState.round;
      addLog(`Round ${gameState.round} began`);
      const drawPhaseDraws = executeDrawPhase(gameState);
      gameLogAccumulator?.recordDrawPhaseDraws(drawPhaseDraws);
      humanPicked = false;
      draftCardOrder = gameState.players.map(() => []);
      onGameUpdated(gameState, gameLog);
    }
  });

  let selectedPlayerIndex = $state(0);

  function isActingHuman(playerIndex: number): boolean {
    return !gameState.aiPlayers[playerIndex] && isTakingActionTurn(gameState, playerIndex);
  }

  // The human whose action view is mounted. It stays mounted while another
  // player's board is looked at, so a half-planned move survives the visit.
  let actionViewSeat = $state(-1);
  $effect(() => {
    if (isActingHuman(selectedPlayerIndex)) actionViewSeat = selectedPlayerIndex;
  });

  let isViewingActiveHuman = $derived.by(() => {
    switch (gameState.phase.type) {
      case 'draft':
        return selectedPlayerIndex === humanPlayerIndex && !humanPicked;
      case 'buyers':
        return selectedPlayerIndex === gameState.phase.buyersState.currentPlayerIndex
          && !gameState.aiPlayers[selectedPlayerIndex];
      case 'action':
        return selectedPlayerIndex === actionViewSeat && isActingHuman(selectedPlayerIndex);
      default:
        return false;
    }
  });

  let shownActingPlayers = $derived(
    gameState.phase.type === 'draft'
      ? (humanPlayerIndex >= 0 && !humanPicked ? [humanPlayerIndex] : [])
      : actingPlayers(gameState)
  );

  let thinkingPlayers = $derived.by(() => {
    switch (gameState.phase.type) {
      case 'buyers':
        return aiThinking ? [gameState.phase.buyersState.currentPlayerIndex] : [];
      case 'draft':
        return aiThinking ? gameState.aiPlayers.flatMap((ai, i) => (ai ? [i] : [])) : [];
      case 'action':
        return thinkingInActionPhase;
      default:
        return [];
    }
  });

  let waitingText = $derived.by(() => {
    if (gameState.phase.type === 'draft') return aiThinking ? 'Thinking...' : 'Waiting...';
    if (thinkingPlayers.includes(selectedPlayerIndex)) return 'Thinking...';
    if (gameState.phase.type === 'action' && !isTakingActionTurn(gameState, selectedPlayerIndex)) {
      return 'Turn over — waiting for the others...';
    }
    return 'Waiting...';
  });

  function selectPlayer(index: number) {
    selectedPlayerIndex = index;
  }

  // ── Compound choices ──
  // When a human destroys a drafted card with a compound-able ability, the
  // follow-up choice that answers it is logged together with the destroy as a
  // single compound choice. The UI flow is unchanged.

  // The follow-up choices that answer each ability a destroy can open with.
  const ANSWERS: Partial<Record<Choice['type'], Ability['type']>> = {
    mixAll: 'mixColors',
    selectSellCard: 'sell',
    workshop: 'workshop',
    skipWorkshop: 'workshop',
    moveToDraftPool: 'moveToDraftPool',
  };
  const COMPOUND_ABILITY_TYPES = new Set(Object.values(ANSWERS));

  function firstDestroyAbility(card: Card): Ability | null {
    const data = getCardData(card);
    return data != null && data.kind !== 'sellCard' ? data.destroyAbilities[0] : null;
  }

  function isCompoundableDestroy(choice: Choice): choice is { type: 'destroyDraftedCard'; card: Card } {
    if (choice.type !== 'destroyDraftedCard') return false;
    const first = firstDestroyAbility(choice.card as Card);
    return first !== null && COMPOUND_ABILITY_TYPES.has(first.type);
  }

  function buildCompoundChoice(destroyCard: Card, followUp: Choice): Choice | null {
    // A two-part card whose first ability fizzled is answered for its second;
    // that is its own choice, not part of the destroy.
    if (ANSWERS[followUp.type] !== firstDestroyAbility(destroyCard)?.type) return null;
    switch (followUp.type) {
      case 'mixAll':
        return { type: 'destroyAndMix', card: destroyCard, mixes: followUp.mixes };
      case 'selectSellCard':
        return { type: 'destroyAndSell', card: destroyCard, sellCard: followUp.sellCard };
      case 'workshop':
        return { type: 'destroyAndWorkshop', card: destroyCard, workshopCards: followUp.cardTypes };
      case 'skipWorkshop':
        return { type: 'destroyAndWorkshop', card: destroyCard, workshopCards: [] };
      case 'moveToDraftPool':
        return { type: 'destroyAndMoveToDraftPool', card: destroyCard, target: followUp.card };
      default:
        return null;
    }
  }

  // ── Action phase ──
  // Everyone acts at once, so moves from different players interleave. Each
  // step is one logged choice by one player, kept so that a player can undo
  // their own moves by replaying everyone else's from the start of the phase.

  interface ActionStep {
    playerIndex: number;
    // As applied to the engine: a destroy, then the follow-up answering it.
    choices: Choice[];
    draws: DrawEvent[][];
    entry: StructuredLogEntry | null;
  }

  let actionPhase: { round: number; start: GameState; logLength: number } | null = null;
  let actionSteps: ActionStep[] = $state.raw([]);
  // Each player's destroy that is still waiting for the choice answering it.
  const awaitingFollowUp = new Map<number, ActionStep>();

  function beginActionPhase() {
    actionPhase = { round: gameState.round, start: cloneGameState(gameState), logLength: gameLog.length };
    actionSteps = [];
    awaitingFollowUp.clear();
  }

  function loggedChoice(step: ActionStep): Choice {
    const [first, followUp] = step.choices;
    return followUp ? buildCompoundChoice((first as { card: Card }).card, followUp)! : first;
  }

  function reviseEntry(step: ActionStep) {
    if (!step.entry) return;
    const draws = step.draws.flat();
    step.entry.choice = loggedChoice(step);
    step.entry.draws = draws.length > 0 ? draws : undefined;
  }

  function syncActionLog() {
    const lines = actionSteps.flatMap(step => {
      const message = getChoiceLogMessage(gameState, loggedChoice(step), step.playerIndex);
      return message ? message.split('\n') : [];
    });
    gameLog = [...gameLog.slice(0, actionPhase!.logLength), ...lines];
  }

  function playActionChoice(playerIndex: number, choice: Choice) {
    const draws = applyChoice(gameState, playerIndex, choice);
    const awaiting = awaitingFollowUp.get(playerIndex);
    awaitingFollowUp.delete(playerIndex);

    if (awaiting && buildCompoundChoice((awaiting.choices[0] as { card: Card }).card, choice)) {
      awaiting.choices.push(choice);
      awaiting.draws.push(draws);
      reviseEntry(awaiting);
      actionSteps = [...actionSteps];
    } else {
      const entry = gameLogAccumulator?.recordChoice(actionPhase!.start, choice, playerIndex, draws) ?? null;
      const step: ActionStep = { playerIndex, choices: [choice], draws: [draws], entry };
      actionSteps = [...actionSteps, step];
      if (isCompoundableDestroy(choice)) awaitingFollowUp.set(playerIndex, step);
    }

    syncActionLog();
    onGameUpdated(gameState, gameLog);
  }

  function lastStepOf(playerIndex: number): number {
    for (let i = actionSteps.length - 1; i >= 0; i--) {
      if (actionSteps[i].playerIndex === playerIndex) return i;
    }
    return -1;
  }

  function canUndo(playerIndex: number): boolean {
    return isTakingActionTurn(gameState, playerIndex) && lastStepOf(playerIndex) >= 0;
  }

  // Take back the player's last choice, then replay the phase without it.
  function undoActionChoice(playerIndex: number) {
    if (!canUndo(playerIndex)) return;
    const index = lastStepOf(playerIndex);
    const step = actionSteps[index];
    if (step.choices.length > 1) {
      step.choices.pop();
      step.draws.pop();
      reviseEntry(step);
      actionSteps = [...actionSteps];
    } else {
      if (step.entry) gameLogAccumulator?.removeEntry(step.entry);
      actionSteps = actionSteps.filter((_, i) => i !== index);
    }

    const state = cloneGameState(actionPhase!.start);
    for (const s of actionSteps) {
      for (const choice of s.choices) applyChoice(state, s.playerIndex, choice);
    }
    gameState = state;

    // A destroy left as the player's last move is again waiting for its follow-up.
    awaitingFollowUp.delete(playerIndex);
    const last = actionSteps[lastStepOf(playerIndex)];
    if (last && last.choices.length === 1 && isCompoundableDestroy(last.choices[0])) {
      const stack = state.phase.type === 'action' ? state.phase.actionState.turns[playerIndex].abilityStack : [];
      if (stack.at(-1)?.type === firstDestroyAbility(last.choices[0].card)?.type) {
        awaitingFollowUp.set(playerIndex, last);
      }
    }

    syncActionLog();
    onGameUpdated(gameState, gameLog);
  }

  // Start each AI's turn as the phase opens, and again after a failed one is retried.
  $effect(() => {
    if (gameState.phase.type !== 'action' || aiError) return;
    untrack(() => {
      if (actionPhase?.round !== gameState.round) beginActionPhase();
      startAITurns();
    });
  });

  function startAITurns() {
    const aiSeats = gameState.aiPlayers.flatMap((ai, i) => (ai ? [i] : []));
    aiActionTurns.start(
      gameState,
      aiSeats,
      seat => aiIterations[seat],
      (seat, choice) => {
        if (!isTakingActionTurn(gameState, seat)) return false;
        playActionChoice(seat, choice);
        return true;
      },
      () => { thinkingInActionPhase = aiActionTurns.thinking; },
      e => {
        console.error('AI action error:', e);
        aiError = String(e);
      },
    );
  }

  // ── Buyers and draft ──

  // Unified action handler for both human UI and AI choices
  function handleAction(playerIndex: number, choice: Choice) {
    // Draft picks use simultaneous picking
    if (gameState.phase.type === 'draft' && choice.type === 'draftPick') {
      handleDraftPick(choice);
      return;
    }
    if (gameState.phase.type === 'action') {
      playActionChoice(playerIndex, choice);
      return;
    }

    const logMsg = getChoiceLogMessage(gameState, choice, playerIndex);
    gameLogAccumulator?.recordChoice(gameState, choice, playerIndex);
    const draws = applyChoice(gameState, playerIndex, choice);
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
    humanPicked = true;
    onGameUpdated(gameState, gameLog);
  }

  function aiDraftPickers(): DraftPicker[] {
    if (gameState.phase.type !== 'draft') return [];
    const { hands } = gameState.phase.draftState;
    return gameState.aiPlayers.flatMap((isAI, playerIndex) =>
      isAI && hands[playerIndex].length > 0 ? [{ playerIndex, iterations: aiIterations[playerIndex] }] : []
    );
  }

  async function resolveAIDraftPicks() {
    aiThinking = true;
    try {
      const results = await Promise.all(
        aiDraftPickers().map(async (picker) => ({
          playerIdx: picker.playerIndex,
          choice: await aiController.draftPick(gameState, picker),
        }))
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
      humanPicked = false;
      onGameUpdated(gameState, gameLog);
    } catch (e) {
      console.error('AI draft error:', e);
      aiError = String(e);
    } finally {
      aiThinking = false;
    }
  }

  // The AI players' draft picks are searched while the human decides and
  // applied once they have picked, or straight away when nobody is human.
  $effect(() => {
    if (gameState.phase.type !== 'draft') {
      aiController.cancelDraftPicks();
      return;
    }
    if (aiThinking || aiError) return;
    if (humanPlayerIndex >= 0 && !humanPicked) {
      aiController.startDraftPicks(gameState, aiDraftPickers());
    } else {
      resolveAIDraftPicks();
    }
  });

  // Trigger the AI's claim when an AI is seated in the buyers phase
  $effect(() => {
    if (aiThinking || aiError) return;
    if (gameState.phase.type !== 'buyers') return;

    const playerIdx = gameState.phase.buyersState.currentPlayerIndex;
    if (!gameState.aiPlayers[playerIdx]) return;

    aiThinking = true;

    aiController.getAIChoice(gameState, playerIdx, aiIterations[playerIdx]).then((choice) => {
      aiThinking = false;
      handleAction(playerIdx, choice);
    }).catch((e) => {
      console.error('AI buyers error:', e);
      aiThinking = false;
      aiError = String(e);
    });
  });
</script>

<GameLayout {gameState} actingPlayers={shownActingPlayers} {thinkingPlayers} controlledPlayerIndex={isViewingActiveHuman ? selectedPlayerIndex : -1} {elapsedSeconds} {gameLog} onLeaveGame={onLeaveGame} {selectedPlayerIndex} onSelectPlayer={selectPlayer} {aiError} onRetryAI={() => { aiError = null; }} hidePlayerCards={isViewingActiveHuman && gameState.phase.type === 'action'} {draftCardOrder}>
  {#if gameState.phase.type === 'buyers'}
    {#if isViewingActiveHuman}
      <BuyersPhaseView {gameState} onAction={(choice) => handleAction(selectedPlayerIndex, choice)} />
    {:else}
      <div class="waiting-indicator">
        <span class="waiting-spinner"></span>
        <span class="waiting-text">{waitingText}</span>
      </div>
    {/if}
  {:else if gameState.phase.type === 'draft'}
    {#if isViewingActiveHuman}
      <DraftPhaseView {gameState} onAction={(choice) => handleAction(humanPlayerIndex, choice)} playerIndex={humanPlayerIndex} selectable={!humanPicked} />
    {:else}
      <div class="waiting-indicator">
        <span class="waiting-spinner"></span>
        <span class="waiting-text">{waitingText}</span>
      </div>
    {/if}
  {:else if gameState.phase.type === 'action'}
    {#if actionViewSeat >= 0 && isActingHuman(actionViewSeat)}
      {#key actionViewSeat}
        <div style:display={isViewingActiveHuman ? 'contents' : 'none'}>
          <ActionPhaseView
            {gameState}
            playerIndex={actionViewSeat}
            onAction={(choice) => handleAction(actionViewSeat, choice)}
            onUndo={() => undoActionChoice(actionViewSeat)}
            undoAvailable={canUndo(actionViewSeat)}
            {draftCardOrder}
          />
        </div>
      {/key}
    {/if}
    {#if !isViewingActiveHuman}
      <div class="waiting-indicator">
        <span class="waiting-spinner"></span>
        <span class="waiting-text">{waitingText}</span>
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
