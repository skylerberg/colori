<script lang="ts">
  import type { GameState, Choice, StructuredGameLog } from '../data/types';
  import type { SanitizedGameState } from '../network/types';
  import { DEFAULT_AI_ITERATIONS } from '../data/aiDifficulty';
  import type { HostController } from '../network/hostController';
  import type { GuestController } from '../network/guestController';
  import { sanitizedToGameState } from '../network/stateAdapter';
  import { AIController } from '../ai/aiController';
  import { AIActionTurns } from '../ai/actionTurns';
  import { actingPlayers, isTakingActionTurn } from '../gameUtils';
  import { reportError } from '../stores/errorReport.svelte';
  import { untrack } from 'svelte';
  import type { LogEntry } from '../logEntries';
  import GameLayout from './GameLayout.svelte';
  import BuyersPhaseView from './BuyersPhaseView.svelte';
  import DraftPhaseView from './DraftPhaseView.svelte';
  import ActionPhaseView from './ActionPhaseView.svelte';
  import WaitingNotice from './WaitingNotice.svelte';

  let { role, hostController, guestController, initialGuestState, onGameOver, gameStartTime, onLeaveGame }: {
    role: 'host' | 'guest';
    hostController?: HostController | null;
    guestController?: GuestController | null;
    initialGuestState?: SanitizedGameState | null;
    onGameOver: (gameState: GameState, structuredLog?: StructuredGameLog) => void;
    gameStartTime: number;
    onLeaveGame: () => void;
  } = $props();

  // Host state
  let hostGameState: GameState | null = $state(null);
  let hostGameLog: LogEntry[] = $state([]);

  // Guest state — seed from the initial SanitizedGameState so the screen renders immediately
  // after gameStarted (before the first stateUpdate arrives).
  // svelte-ignore state_referenced_locally
  let guestSanitizedState: SanitizedGameState | null = $state(initialGuestState ?? null);
  // svelte-ignore state_referenced_locally
  let guestGameLog: LogEntry[] = $state(initialGuestState?.logEntries ? [...initialGuestState.logEntries] : []);

  // Shared state
  // A buyers claim or draft picks being searched for.
  let aiThinking = $state(false);
  // svelte-ignore state_referenced_locally
  let effectiveStartTime = $state(gameStartTime);
  let connectionStatus = $state<'ok' | 'stalled' | 'lost'>('ok');
  let latencyMs = $state(0);
  let connectionBanner: { text: string; kind: 'info' | 'warn' } | null = $state(null);
  let connectionBannerTimer: ReturnType<typeof setTimeout> | null = null;

  // Timer
  let elapsedSeconds = $state(0);

  $effect(() => {
    elapsedSeconds = Math.floor((Date.now() - effectiveStartTime) / 1000);
    const interval = setInterval(() => {
      elapsedSeconds = Math.floor((Date.now() - effectiveStartTime) / 1000);
    }, 1000);
    return () => clearInterval(interval);
  });

  // AI controller (host only)
  // svelte-ignore state_referenced_locally
  const aiController = role === 'host' ? new AIController() : null;
  const aiActionTurns = aiController ? new AIActionTurns(aiController) : null;
  let thinkingInActionPhase: number[] = $state([]);
  let aiGenerationSeq = 0;

  $effect(() => {
    return () => {
      aiActionTurns?.cancel();
      aiController?.terminate();
    };
  });

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

  let acting = $derived(gameState ? actingPlayers(gameState) : []);

  // Whether this player has a buyers claim or an action turn to make.
  let isMyTurn = $derived(
    gameState !== null && gameState.phase.type !== 'draft' && acting.includes(myPlayerIndex)
  );

  let thinkingPlayers = $derived.by(() => {
    if (!gameState) return [];
    switch (gameState.phase.type) {
      case 'buyers':
        return aiThinking ? acting : [];
      case 'draft':
        return aiThinking ? gameState.aiPlayers.flatMap((ai, i) => (ai ? [i] : [])) : [];
      case 'action':
        return thinkingInActionPhase;
      default:
        return [];
    }
  });

  let waitingFor = $derived(
    gameState ? acting.filter(i => i !== myPlayerIndex).map(i => gameState!.playerNames[i]).join(', ') : ''
  );

  let selectedPlayerIndex = $state(0);

  $effect(() => {
    if (myPlayerIndex >= 0) {
      selectedPlayerIndex = myPlayerIndex;
    }
  });

  function selectPlayer(index: number) {
    selectedPlayerIndex = index;
  }

  function showBanner(text: string, kind: 'info' | 'warn' = 'info', durationMs = 4000) {
    if (connectionBannerTimer) clearTimeout(connectionBannerTimer);
    connectionBanner = { text, kind };
    connectionBannerTimer = setTimeout(() => {
      connectionBanner = null;
    }, durationMs);
  }

  // Draft card order tracking — updated imperatively from state-change callbacks
  // (not via $effect — that pattern self-loops because it reads+writes the same $state).
  let draftCardOrder: number[][] = $state([]);
  let lastPickNumber: number | null = null;
  let hasPicked = $state(false);

  function syncFromGameState(gs: GameState) {
    // Keep draftCardOrder in sync with the latest drafted cards per player.
    if (draftCardOrder.length !== gs.players.length) {
      draftCardOrder = gs.players.map(() => []);
    } else {
      for (let i = 0; i < gs.players.length; i++) {
        const drafted = gs.players[i].draftedCards;
        if (drafted.length === 0 && draftCardOrder[i].length > 0) {
          draftCardOrder[i] = [];
          continue;
        }
        const knownIds = new Set(draftCardOrder[i]);
        let appended: number[] | null = null;
        for (const c of drafted) {
          if (!knownIds.has(c.instanceId)) {
            if (appended === null) appended = [...draftCardOrder[i]];
            appended.push(c.instanceId);
            knownIds.add(c.instanceId);
          }
        }
        if (appended !== null) draftCardOrder[i] = appended;
      }
    }

    // Reset optimistic hasPicked lock when a new draft pick round begins.
    if (gs.phase.type === 'draft') {
      const pn = gs.phase.draftState.pickNumber;
      if (lastPickNumber !== null && pn !== lastPickNumber) {
        hasPicked = false;
      }
      lastPickNumber = pn;
    }
  }

  // Setup host controller callbacks
  // svelte-ignore state_referenced_locally
  if (role === 'host' && hostController) {
    hostController.onGameStateChanged = (state) => {
      hostGameState = state;
      syncFromGameState(state);
    };
    hostController.onLogUpdated = (log) => {
      hostGameLog = [...log];
    };
    hostController.onGameOver = (state) => {
      const structuredLog = hostController!.getStructuredLog() ?? undefined;
      onGameOver(state, structuredLog);
    };
    hostController.onPlayerDisconnected = (_idx, name) => {
      showBanner(`${name} disconnected — waiting up to 60s before AI takes over`, 'warn', 6000);
    };
    hostController.onPlayerReconnected = (_idx, name) => {
      showBanner(`${name} reconnected`, 'info');
    };
    hostController.onPlayerReplacedByAI = (_idx, name) => {
      showBanner(`${name} replaced by AI`, 'warn', 6000);
    };

    hostGameState = hostController.getGameState();
    hostGameLog = [...hostController.getGameLog()];
    effectiveStartTime = hostController.getGameStartTime() || gameStartTime;
    if (hostGameState) syncFromGameState(hostGameState);
  }

  // Setup guest controller callbacks
  // svelte-ignore state_referenced_locally
  if (role === 'guest' && guestController) {
    guestController.onSanitizedStateChanged = (state) => {
      guestSanitizedState = state;
      guestGameLog = [...guestController!.getGameLog()];
      if (state.gameStartTime) effectiveStartTime = state.gameStartTime;
      syncFromGameState(sanitizedToGameState(state));
    };
    guestController.onGameOver = (state) => {
      guestSanitizedState = state;
      guestGameLog = [...guestController!.getGameLog()];
      onGameOver(sanitizedToGameState(state));
    };
    guestController.onError = (message, context) => {
      if (context === 'draftPick') {
        hasPicked = false;
      }
      showBanner(message, 'warn', 5000);
    };
    guestController.onPlayerDisconnected = (_idx, name) => {
      showBanner(`${name} disconnected`, 'warn', 5000);
    };
    guestController.onPlayerReconnected = (_idx, name) => {
      showBanner(`${name} reconnected`, 'info');
    };
    guestController.onPlayerReplacedByAI = (_idx, name) => {
      showBanner(`${name} replaced by AI`, 'warn', 5000);
    };
    guestController.onLatencyChange = (ms, stalled) => {
      latencyMs = ms;
      connectionStatus = stalled ? 'stalled' : 'ok';
    };
    if (guestSanitizedState) syncFromGameState(sanitizedToGameState(guestSanitizedState));
  }

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

    if (gameState.phase.type === 'draft') {
      const ds = gameState.phase.draftState;
      const aiPlayerIndices = gameState.aiPlayers
        .map((isAI, idx) => isAI ? idx : -1)
        .filter(idx => idx >= 0)
        .filter(idx => !hostController!.hasSubmittedDraftPick(idx))
        .filter(idx => ds.hands[idx].length > 0);

      if (aiPlayerIndices.length === 0) return;

      aiThinking = true;
      const mySeq = ++aiGenerationSeq;
      const snapshotAI = [...gameState.aiPlayers];
      Promise.all(
        aiPlayerIndices.map(playerIdx =>
          aiController!.draftPick(gameState!, { playerIndex: playerIdx, iterations: DEFAULT_AI_ITERATIONS }).then(choice => ({
            playerIdx,
            choice,
          }))
        )
      ).then(results => {
        if (mySeq !== aiGenerationSeq) {
          aiThinking = false;
          return;
        }
        try {
          for (const { playerIdx, choice } of results) {
            if (!hostController) break;
            const curState = hostController.getGameState();
            if (!curState || curState.phase.type !== 'draft') break;
            if (!curState.aiPlayers[playerIdx]) continue;
            if (!snapshotAI[playerIdx]) continue;
            if (hostController.hasSubmittedDraftPick(playerIdx)) continue;
            // Stale-pick guard: if hands rotated between when the AI computed this
            // choice and now, the chosen card is no longer in this player's hand.
            // Apply it anyway and we'd corrupt the next pick.
            const hand = curState.phase.draftState.hands[playerIdx];
            const card = (choice as { type: 'draftPick'; card: unknown }).card;
            if (!hand || !hand.some(c => c.card === card)) continue;
            hostController.applyAction(choice, playerIdx);
          }
        } finally {
          // Only release aiThinking after applying all picks. Releasing before the
          // loop lets the AI effect re-trigger on each intermediate state update
          // and spawn a second concurrent AI batch.
          aiThinking = false;
        }
      }).catch((e) => {
        reportError('An AI draft pick failed', e, gameState);
        aiThinking = false;
      });
      return;
    }

    if (gameState.phase.type === 'action') {
      untrack(() => startAITurns());
      return;
    }

    if (gameState.phase.type !== 'buyers') return;
    const playerIdx = gameState.phase.buyersState.currentPlayerIndex;
    if (!gameState.aiPlayers[playerIdx]) return;
    aiThinking = true;
    const mySeq = ++aiGenerationSeq;

    aiController!.getAIChoice(gameState, playerIdx, DEFAULT_AI_ITERATIONS).then((choice) => {
      aiThinking = false;
      if (mySeq !== aiGenerationSeq) return;
      if (!hostController) return;
      const curState = hostController.getGameState();
      if (!curState || curState.phase.type !== 'buyers') return;
      // Abandon if this player is no longer AI (reconnected) or no longer the one claiming.
      if (!curState.aiPlayers[playerIdx]) return;
      if (curState.phase.buyersState.currentPlayerIndex !== playerIdx) return;
      hostController.applyAction(choice, playerIdx);
    }).catch((e) => {
      reportError('An AI buyer claim failed', e, gameState);
      aiThinking = false;
    });
  });

  // Each AI seat plays its action turn on its own view, as the phase opens or
  // as it takes over from a player who left.
  function startAITurns() {
    if (!gameState || !hostController || !aiActionTurns) return;
    const aiSeats = gameState.aiPlayers.flatMap((ai, i) => (ai ? [i] : []));
    aiActionTurns.start(
      gameState,
      aiSeats,
      () => DEFAULT_AI_ITERATIONS,
      (seat, choice) => {
        const curState = hostController!.getGameState();
        // A player who reconnected takes their turn back.
        if (!curState?.aiPlayers[seat] || !isTakingActionTurn(curState, seat)) return false;
        return hostController!.applyAction(choice, seat);
      },
      () => { thinkingInActionPhase = aiActionTurns.thinking; },
      e => reportError('An AI turn failed', e, gameState),
    );
  }

</script>

{#if gameState}
  <GameLayout {gameState} actingPlayers={acting} {thinkingPlayers} viewerIndex={myPlayerIndex} {elapsedSeconds} {gameLog} onLeaveGame={onLeaveGame} {selectedPlayerIndex} onSelectPlayer={selectPlayer} {draftCardOrder}>
    {#if connectionBanner}
      <div class="connection-banner" class:warn={connectionBanner.kind === 'warn'}>{connectionBanner.text}</div>
    {/if}

    {#if role === 'guest' && connectionStatus === 'stalled'}
      <div class="connection-banner warn">Connection to host appears stalled{latencyMs > 0 ? ` (last ping ${latencyMs}ms)` : ''}…</div>
    {/if}

    {#if gameState.phase.type === 'buyers'}
      {#if isMyTurn}
        <BuyersPhaseView {gameState} playerIndex={myPlayerIndex} onAction={handleAction} />
      {:else}
        <WaitingNotice text="{waitingFor} is claiming a buyer" detail="Players take one buyer each in turn until every open slot is full." />
      {/if}
    {:else if gameState.phase.type === 'draft'}
      <DraftPhaseView {gameState} onAction={handleAction} playerIndex={myPlayerIndex} selectable={!hasPicked} />
      {#if hasPicked}
        <WaitingNotice text="Waiting for the others to pick" />
      {/if}
    {:else if gameState.phase.type === 'action'}
      {#if isMyTurn}
        <ActionPhaseView {gameState} playerIndex={myPlayerIndex} onAction={handleAction} />
      {:else}
        <WaitingNotice text="Your turn is over" detail={waitingFor ? `Waiting for ${waitingFor}…` : undefined} />
      {/if}
    {/if}
  </GameLayout>
{/if}

<style>
  .connection-banner {
    padding: 8px 12px;
    margin-bottom: 8px;
    border-radius: 6px;
    background: rgba(201, 168, 76, 0.15);
    color: #f5ede0;
    font-size: 0.9rem;
    text-align: center;
  }

  .connection-banner.warn {
    background: rgba(139, 32, 32, 0.35);
    color: #f3c4c4;
    font-weight: 600;
  }
</style>
