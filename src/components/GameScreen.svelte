<script lang="ts">
  import type { BotSettings, GameState, Choice, Card, Ability, DrawEvent, StructuredLogEntry } from '../data/types';
  import { applyChoice, simultaneousPick, advanceDraft, cloneGameState } from '../engine/wasmEngine';
  import { getCardData } from '../data/cards';
  import { BotController, type DraftPicker } from '../bots/botController';
  import { BotActionTurns } from '../bots/actionTurns';
  import type { GameLogAccumulator } from '../gameLog';
  import { type LogEntry, topCardEntries, playEntry, roundEntry, snapshot } from '../logEntries';
  import { actingPlayers, isTakingActionTurn } from '../gameUtils';
  import GameLayout from './GameLayout.svelte';
  import BuyersPhaseView from './BuyersPhaseView.svelte';
  import DraftPhaseView from './DraftPhaseView.svelte';
  import ActionPhaseView from './ActionPhaseView.svelte';
  import WaitingNotice from './WaitingNotice.svelte';
  import { startTutorial, cancelTutorial } from '../tutorial/tutorial';
  import { createErrorReport, type ErrorReport } from '../errorReport';
  import { untrack } from 'svelte';

  let { gameState, gameStartTime, onGameUpdated, initialGameLog, onLeaveGame, gameLogAccumulator, botIterations, botSettings }: {
    gameState: GameState;
    gameStartTime: number | null;
    onGameUpdated: (state: GameState, log: LogEntry[]) => void;
    initialGameLog: LogEntry[];
    onLeaveGame: () => void;
    gameLogAccumulator: GameLogAccumulator | null;
    botIterations: number[];
    botSettings: BotSettings;
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
  // A buyers claim or draft pick being searched for.
  let botThinking = $state(false);
  let botError: ErrorReport | null = $state(null);
  // svelte-ignore state_referenced_locally
  let gameLog: LogEntry[] = $state(initialGameLog);

  // Start tutorial on first mount (checks localStorage internally)
  let tutorialStarted = false;
  $effect(() => {
    if (!tutorialStarted) {
      tutorialStarted = true;
      startTutorial();
    }
  });

  // Clean up tutorial on unmount
  $effect(() => {
    return () => cancelTutorial();
  });

  function addLog(entry: LogEntry | null) {
    if (entry) gameLog = [...gameLog, entry];
  }

  // A round opens inside the choice that ends the last turn of the one before,
  // so the log and the draft tracking are rolled over here rather than by a
  // phase of their own.
  function noteRoundChange(round: number) {
    if (gameState.round === round) return;
    addLog(roundEntry(gameState.round));
    humanPicked = false;
    draftCardOrder = gameState.players.map(() => []);
  }

  // Apply a choice, logging what it did.
  function applyLogged(playerIndex: number, choice: Choice): DrawEvent[] {
    const before = snapshot(gameState.players[playerIndex]);
    const round = gameState.round;
    const draws = applyChoice(gameState, playerIndex, choice);
    addLog(playEntry(round, playerIndex, choice, before, gameState.players[playerIndex]));
    noteRoundChange(round);
    return draws;
  }

  // Nothing draws a round in any more, so the log opens at round 1.
  $effect(() => {
    if (gameLog.length === 0) addLog(roundEntry(gameState.round));
  });

  const botController = new BotController();
  // svelte-ignore state_referenced_locally
  botController.botSettings = botSettings;
  const botActionTurns = new BotActionTurns(botController);
  let thinkingInActionPhase: number[] = $state([]);

  $effect(() => {
    return () => {
      botActionTurns.cancel();
      botController.terminate();
    };
  });

  let humanPicked = $state(false);
  let humanPlayerIndex = $derived(gameState.botPlayers.findIndex(bot => !bot));

  let selectedPlayerIndex = $state(0);

  function isActingHuman(playerIndex: number): boolean {
    return playerIndex >= 0 && !gameState.botPlayers[playerIndex] && isTakingActionTurn(gameState, playerIndex);
  }

  // The human whose action controls are showing: the one whose board is
  // being looked at if they still have a turn to take, and otherwise the
  // first who does. It stays put while a bot's board is looked at, so a
  // half-planned move survives the visit.
  let actionViewSeat = $state(-1);
  $effect(() => {
    if (gameState.phase.type !== 'action') return;
    if (isActingHuman(selectedPlayerIndex)) {
      actionViewSeat = selectedPlayerIndex;
    } else if (!isActingHuman(actionViewSeat)) {
      actionViewSeat = gameState.botPlayers.findIndex((ai, i) => !ai && isTakingActionTurn(gameState, i));
    }
  });

  let buyersClaimant = $derived(gameState.phase.type === 'buyers' ? gameState.phase.buyersState.currentPlayerIndex : -1);

  // Each decision opens on the board of the human making it.
  let boardShownFor = '';
  $effect(() => {
    const seat = gameState.phase.type === 'action'
      ? gameState.botPlayers.findIndex((ai, i) => !ai && isTakingActionTurn(gameState, i))
      : gameState.phase.type === 'buyers' && !gameState.botPlayers[buyersClaimant] ? buyersClaimant
      : gameState.phase.type === 'draft' ? humanPlayerIndex
      : -1;
    const key = `${gameState.round}:${gameState.phase.type}:${seat}`;
    if (seat < 0 || key === boardShownFor) return;
    boardShownFor = key;
    untrack(() => { selectedPlayerIndex = seat; });
  });

  let viewerIndex = $derived(
    gameState.phase.type === 'action' && actionViewSeat >= 0 ? actionViewSeat
      : buyersClaimant >= 0 && !gameState.botPlayers[buyersClaimant] ? buyersClaimant
      : humanPlayerIndex
  );

  let shownActingPlayers = $derived(
    gameState.phase.type === 'draft'
      ? (humanPlayerIndex >= 0 && !humanPicked ? [humanPlayerIndex] : [])
      : actingPlayers(gameState)
  );

  let thinkingPlayers = $derived.by(() => {
    switch (gameState.phase.type) {
      case 'buyers':
        return botThinking ? [gameState.phase.buyersState.currentPlayerIndex] : [];
      case 'draft':
        return botThinking ? gameState.botPlayers.flatMap((ai, i) => (ai ? [i] : [])) : [];
      case 'action':
        return thinkingInActionPhase;
      default:
        return [];
    }
  });

  let stillPlaying = $derived(
    actingPlayers(gameState).filter(i => i !== viewerIndex).map(i => gameState.playerNames[i]).join(', ')
  );

  function selectPlayer(index: number) {
    selectedPlayerIndex = index;
  }

  // ── Compound choices ──
  // When a human trashes a top card with a compound-able ability, the
  // follow-up choice that answers it is logged together with the trash as a
  // single compound choice. The UI flow is unchanged.

  // The follow-up choices that answer each ability a trash can open with.
  const ANSWERS: Partial<Record<Choice['type'], Ability['type']>> = {
    mixAll: 'mixColors',
    sellToBuyer: 'sell',
    workshop: 'workshop',
    skipWorkshop: 'workshop',
    liftCard: 'liftCard',
  };
  const COMPOUND_ABILITY_TYPES = new Set(Object.values(ANSWERS));

  function firstTrashAbility(card: Card): Ability | null {
    const data = getCardData(card);
    return data != null && data.kind !== 'buyer' ? data.topActions[0] : null;
  }

  function isCompoundableTrash(choice: Choice): choice is { type: 'trashTopCard'; card: Card } {
    if (choice.type !== 'trashTopCard') return false;
    const first = firstTrashAbility(choice.card as Card);
    return first !== null && COMPOUND_ABILITY_TYPES.has(first.type);
  }

  function buildCompoundChoice(trashCard: Card, followUp: Choice): Choice | null {
    // A two-part card whose first ability fizzled is answered for its second;
    // that is its own choice, not part of the trash.
    if (ANSWERS[followUp.type] !== firstTrashAbility(trashCard)?.type) return null;
    switch (followUp.type) {
      case 'mixAll':
        return { type: 'trashAndMix', card: trashCard, mixes: followUp.mixes };
      case 'sellToBuyer':
        return { type: 'trashAndSell', card: trashCard, buyer: followUp.buyer };
      case 'workshop':
        return { type: 'trashAndWorkshop', card: trashCard, workshopCards: followUp.cardTypes };
      case 'skipWorkshop':
        return { type: 'trashAndWorkshop', card: trashCard, workshopCards: [] };
      case 'liftCard':
        return { type: 'trashAndLift', card: trashCard, target: followUp.card };
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
    // As applied to the engine: a trash, then the follow-up answering it.
    choices: Choice[];
    draws: DrawEvent[][];
    entry: StructuredLogEntry | null;
    // What the log shows for each of `choices`.
    logged: LogEntry[];
  }

  let actionPhase: { round: number; start: GameState; logLength: number } | null = null;
  let actionSteps: ActionStep[] = $state.raw([]);
  // Each player's trash that is still waiting for the choice answering it.
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
    gameLog = [...gameLog.slice(0, actionPhase!.logLength), ...actionSteps.flatMap(step => step.logged)];
  }

  function playActionChoice(playerIndex: number, choice: Choice) {
    const before = snapshot(gameState.players[playerIndex]);
    const round = gameState.round;
    const draws = applyChoice(gameState, playerIndex, choice);
    const logged = playEntry(round, playerIndex, choice, before, gameState.players[playerIndex]);
    const awaiting = awaitingFollowUp.get(playerIndex);
    awaitingFollowUp.delete(playerIndex);

    if (awaiting && buildCompoundChoice((awaiting.choices[0] as { card: Card }).card, choice)) {
      awaiting.choices.push(choice);
      awaiting.draws.push(draws);
      if (logged) awaiting.logged.push(logged);
      reviseEntry(awaiting);
      actionSteps = [...actionSteps];
    } else {
      const entry = gameLogAccumulator?.recordChoice(actionPhase!.start, choice, playerIndex, draws) ?? null;
      const step: ActionStep = { playerIndex, choices: [choice], draws: [draws], entry, logged: logged ? [logged] : [] };
      actionSteps = [...actionSteps, step];
      if (isCompoundableTrash(choice)) awaitingFollowUp.set(playerIndex, step);
    }

    noteRoundChange(round);
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
      step.logged.splice(step.choices.length);
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

    // A trash left as the player's last move is again waiting for its follow-up.
    awaitingFollowUp.delete(playerIndex);
    const last = actionSteps[lastStepOf(playerIndex)];
    if (last && last.choices.length === 1 && isCompoundableTrash(last.choices[0])) {
      const stack = state.phase.type === 'action' ? state.phase.actionState.turns[playerIndex].abilityStack : [];
      if (stack.at(-1)?.type === firstTrashAbility(last.choices[0].card)?.type) {
        awaitingFollowUp.set(playerIndex, last);
      }
    }

    syncActionLog();
    onGameUpdated(gameState, gameLog);
  }

  // Start each bot's turn as the phase opens, and again after a failed one is retried.
  $effect(() => {
    if (gameState.phase.type !== 'action' || botError) return;
    untrack(() => {
      if (actionPhase?.round !== gameState.round) beginActionPhase();
      startAITurns();
    });
  });

  function startAITurns() {
    const botSeats = gameState.botPlayers.flatMap((ai, i) => (ai ? [i] : []));
    botActionTurns.start(
      gameState,
      botSeats,
      seat => botIterations[seat],
      (seat, choice) => {
        if (!isTakingActionTurn(gameState, seat)) return false;
        playActionChoice(seat, choice);
        return true;
      },
      () => { thinkingInActionPhase = botActionTurns.thinking; },
      e => {
        console.error('bot action error:', e);
        botError = createErrorReport("A bot's turn failed", e, gameState);
      },
    );
  }

  // ── Buyers and draft ──

  // Unified action handler for both the human UI and bot choices
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

    gameLogAccumulator?.recordChoice(gameState, choice, playerIndex);
    const draws = applyLogged(playerIndex, choice);
    gameLogAccumulator?.attachDrawsToLastEntry(draws);

    onGameUpdated(gameState, gameLog);
  }

  function handleDraftPick(choice: Choice) {
    gameLogAccumulator?.recordChoice(gameState, choice, humanPlayerIndex);
    simultaneousPick(gameState, humanPlayerIndex, (choice as { type: 'draftPick'; card: any }).card);
    // Track the newly top card's order
    const knownIds = new Set(draftCardOrder[humanPlayerIndex]);
    const newCard = gameState.players[humanPlayerIndex].topCards.find(
      c => !knownIds.has(c.instanceId)
    );
    if (newCard) {
      draftCardOrder[humanPlayerIndex] = [...draftCardOrder[humanPlayerIndex], newCard.instanceId];
    }
    humanPicked = true;
    onGameUpdated(gameState, gameLog);
  }

  function botDraftPickers(): DraftPicker[] {
    if (gameState.phase.type !== 'draft') return [];
    const { hands } = gameState.phase.draftState;
    return gameState.botPlayers.flatMap((isBot, playerIndex) =>
      isBot && hands[playerIndex].length > 0 ? [{ playerIndex, iterations: botIterations[playerIndex] }] : []
    );
  }

  async function resolveAIDraftPicks() {
    botThinking = true;
    try {
      const results = await Promise.all(
        botDraftPickers().map(async (picker) => ({
          playerIdx: picker.playerIndex,
          choice: await botController.draftPick(gameState, picker),
        }))
      );

      // Apply every bot's pick
      for (const { playerIdx, choice } of results) {
        gameLogAccumulator?.recordChoice(gameState, choice, playerIdx);
        simultaneousPick(gameState, playerIdx, (choice as { type: 'draftPick'; card: any }).card);
        // Track the newly top card's order
        const knownIds = new Set(draftCardOrder[playerIdx]);
        const newCard = gameState.players[playerIdx].topCards.find(
          c => !knownIds.has(c.instanceId)
        );
        if (newCard) {
          draftCardOrder[playerIdx] = [...draftCardOrder[playerIdx], newCard.instanceId];
        }
      }

      // Advance draft (rotate hands)
      advanceDraft(gameState);
      if (gameState.phase.type !== 'draft') {
        for (const entry of topCardEntries(gameState, draftCardOrder)) addLog(entry);
      }
      humanPicked = false;
      onGameUpdated(gameState, gameLog);
    } catch (e) {
      console.error('bot draft error:', e);
      botError = createErrorReport("A bot's draft pick failed", e, gameState);
    } finally {
      botThinking = false;
    }
  }

  // The bots' draft picks are searched while the human decides and
  // applied once they have picked, or straight away when nobody is human.
  $effect(() => {
    if (gameState.phase.type !== 'draft') {
      botController.cancelDraftPicks();
      return;
    }
    if (botThinking || botError) return;
    if (humanPlayerIndex >= 0 && !humanPicked) {
      botController.startDraftPicks(gameState, botDraftPickers());
    } else {
      resolveAIDraftPicks();
    }
  });

  // Trigger a bot's claim when a bot is seated in the buyers phase
  $effect(() => {
    if (botThinking || botError) return;
    if (gameState.phase.type !== 'buyers') return;

    const playerIdx = gameState.phase.buyersState.currentPlayerIndex;
    if (!gameState.botPlayers[playerIdx]) return;

    botThinking = true;

    botController.getBotChoice(gameState, playerIdx, botIterations[playerIdx]).then((choice) => {
      botThinking = false;
      handleAction(playerIdx, choice);
    }).catch((e) => {
      console.error('bot buyers error:', e);
      botThinking = false;
      botError = createErrorReport("A bot's buyer claim failed", e, gameState);
    });
  });
</script>

<GameLayout {gameState} actingPlayers={shownActingPlayers} {thinkingPlayers} {viewerIndex} {elapsedSeconds} {gameLog} onLeaveGame={onLeaveGame} {selectedPlayerIndex} onSelectPlayer={selectPlayer} {botError} onRetryAI={() => { botError = null; }} {draftCardOrder}>
  {#if gameState.phase.type === 'buyers'}
    {#if !gameState.botPlayers[buyersClaimant]}
      <BuyersPhaseView {gameState} playerIndex={buyersClaimant} onAction={(choice) => handleAction(buyersClaimant, choice)} />
    {:else}
      <WaitingNotice text="{gameState.playerNames[buyersClaimant]} is claiming a buyer" detail="Players take one buyer each in turn until every open slot is full." />
    {/if}
  {:else if gameState.phase.type === 'draft'}
    {#if humanPlayerIndex >= 0}
      <DraftPhaseView {gameState} onAction={(choice) => handleAction(humanPlayerIndex, choice)} playerIndex={humanPlayerIndex} selectable={!humanPicked} />
      {#if humanPicked}
        <WaitingNotice text="Waiting for the others to pick" />
      {/if}
    {:else}
      <WaitingNotice text="The bots are drafting" />
    {/if}
  {:else if gameState.phase.type === 'action'}
    {#if actionViewSeat >= 0 && isActingHuman(actionViewSeat)}
      {#key actionViewSeat}
        <ActionPhaseView
          {gameState}
          playerIndex={actionViewSeat}
          onAction={(choice) => handleAction(actionViewSeat, choice)}
          onUndo={() => undoActionChoice(actionViewSeat)}
          undoAvailable={canUndo(actionViewSeat)}
        />
      {/key}
    {:else}
      <WaitingNotice
        text={humanPlayerIndex >= 0 ? 'Your turn is over' : 'The bots are taking their turns'}
        detail={stillPlaying ? `Waiting for ${stillPlaying}…` : undefined}
      />
    {/if}
  {/if}
</GameLayout>
