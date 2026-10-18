<script lang="ts">
  import type { GameState } from '../data/types';
  import { formatTime } from '../gameUtils';
  import type { Snippet } from 'svelte';
  import BuyersBoard from './BuyersBoard.svelte';
  import PlayerBoard from './PlayerBoard.svelte';
  import PlayerTabs from './PlayerTabs.svelte';
  import ErrorReport from './ErrorReport.svelte';
  import type { ErrorReport as Report } from '../errorReport';
  import { emptyTableInteraction, provideTableInteraction } from '../stores/tableInteraction';

  let { gameState, actingPlayers, thinkingPlayers, viewerIndex, elapsedSeconds, gameLog, onLeaveGame, selectedPlayerIndex = 0, onSelectPlayer, aiError, onRetryAI, draftCardOrder, children }: {
    gameState: GameState;
    // Players with a decision to make; in the action phase, everyone whose
    // turn is not over.
    actingPlayers: number[];
    thinkingPlayers: number[];
    // The seat whose controls this screen shows, or -1.
    viewerIndex: number;
    elapsedSeconds: number;
    gameLog: string[];
    onLeaveGame: () => void;
    selectedPlayerIndex?: number;
    onSelectPlayer?: (index: number) => void;
    aiError?: Report | null;
    onRetryAI?: () => void;
    draftCardOrder?: number[][];
    // The controls panel: the phase view for the viewer.
    children: Snippet;
  } = $props();

  const interaction = $state(emptyTableInteraction());
  provideTableInteraction(interaction);

  let phaseLabel = $derived.by(() => {
    switch (gameState.phase.type) {
      case 'buyers': return 'Buyers';
      case 'draft': return 'Draft';
      case 'action': return 'Action';
      default: return '';
    }
  });

  let shownPlayer = $derived(gameState.players[selectedPlayerIndex]);
  let boardInteraction = $derived(interaction.seat === selectedPlayerIndex ? interaction : null);
  // The viewer has a move to make on a board they are not looking at.
  let awayFromOwnBoard = $derived(
    interaction.seat >= 0 && interaction.seat !== selectedPlayerIndex && gameState.phase.type === 'action'
  );

  let showLog = $state(false);

  function handleLeaveGame() {
    if (confirm('Are you sure you want to leave this game? Your progress will be lost.')) {
      onLeaveGame();
    }
  }

  $effect(() => {
    document.body.classList.add('game-active');
    return () => {
      document.body.classList.remove('game-active');
    };
  });
</script>

<div class="game-screen">
  <div class="game-overlay"></div>

  <div class="game-content">
    <header class="top-bar">
      <div class="round-indicator">
        <span>Round {gameState.round} of {gameState.maxRounds}</span>
        {#if phaseLabel}<span class="phase-label">{phaseLabel}</span>{/if}
        <span class="clock">{formatTime(elapsedSeconds)}</span>
      </div>
      <PlayerTabs
        {gameState}
        {selectedPlayerIndex}
        onSelect={(i) => onSelectPlayer?.(i)}
        {actingPlayers}
        {thinkingPlayers}
        {viewerIndex}
      />
      <div class="top-bar-actions">
        {#if gameLog.length > 0}
          <button class="top-btn" onclick={() => showLog = !showLog}>Log ({gameLog.length})</button>
        {/if}
        <button class="top-btn" onclick={handleLeaveGame}>Leave</button>
      </div>
    </header>

    <div class="table" class:market-first={gameState.phase.type === 'buyers'}>
      <section class="controls" aria-label="Your move">
        {#if aiError}
          <ErrorReport report={aiError}>
            {#snippet actions()}
              {#if onRetryAI}
                <button class="retry-btn" onclick={onRetryAI}>Retry</button>
              {/if}
            {/snippet}
          </ErrorReport>
        {/if}
        {@render children()}
      </section>

      <section class="market" aria-label="Buyer's board">
        <BuyersBoard {gameState} market={interaction.market} />
      </section>

      <section class="board-area" aria-label="{gameState.playerNames[selectedPlayerIndex]}'s board">
        {#if awayFromOwnBoard}
          <button class="back-to-board" onclick={() => onSelectPlayer?.(interaction.seat)}>
            You're looking at {gameState.playerNames[selectedPlayerIndex]}'s board — back to yours to play
          </button>
        {/if}
        <div class="board-fit">
          <div class="board-box">
            {#key selectedPlayerIndex}
              <PlayerBoard
                player={shownPlayer}
                seat={selectedPlayerIndex}
                round={gameState.round}
                draftOrder={draftCardOrder?.[selectedPlayerIndex]}
                faceDownDrafted={gameState.phase.type === 'draft' && selectedPlayerIndex !== viewerIndex}
                claiming={gameState.phase.type === 'buyers' && gameState.phase.buyersState.currentPlayerIndex === selectedPlayerIndex}
                interaction={boardInteraction}
              />
            {/key}
          </div>
        </div>
      </section>
    </div>
  </div>

  {#if showLog}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="log-overlay" onclick={() => showLog = false} role="presentation">
      <div class="log-modal" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
        <div class="log-modal-header">
          <span class="log-modal-title">Game Log</span>
          <button class="log-close-btn" onclick={() => showLog = false}>&times;</button>
        </div>
        <div class="log-entries">
          {#each [...gameLog].reverse() as entry}
            <div class="log-entry">{entry}</div>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .game-screen {
    position: fixed;
    inset: 0;
    background-image: url('/new-background.webp');
    background-size: cover;
    background-position: center;
    overflow: hidden;
  }

  .game-overlay {
    position: fixed;
    inset: 0;
    background: rgba(10, 8, 5, 0.72);
    pointer-events: none;
    z-index: 0;
  }

  .game-content {
    position: relative;
    z-index: 1;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px;
    overflow-y: auto;
    overflow-x: hidden;
    overscroll-behavior: none;
  }

  /* ── Top bar ── */
  .top-bar {
    display: grid;
    grid-template-columns: 1fr auto;
    grid-template-areas:
      'round actions'
      'tabs tabs';
    align-items: center;
    gap: 6px 12px;
    flex-shrink: 0;
  }

  .round-indicator {
    grid-area: round;
    display: flex;
    align-items: center;
    gap: 10px;
    font-family: 'Cinzel', serif;
    font-size: 0.8rem;
    font-weight: 600;
    color: #c9a84c;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    white-space: nowrap;
  }

  .phase-label {
    padding: 2px 8px;
    border-radius: 4px;
    background: rgba(201, 168, 76, 0.18);
    color: #e8c766;
  }

  .clock {
    color: rgba(245, 237, 224, 0.5);
    font-variant-numeric: tabular-nums;
  }

  .top-bar :global(.player-tabs) {
    grid-area: tabs;
  }

  .top-bar-actions {
    grid-area: actions;
    display: flex;
    gap: 6px;
  }

  .top-btn {
    padding: 4px 12px;
    min-height: 32px;
    font-family: 'Cinzel', serif;
    font-size: 0.7rem;
    color: #f5ede0;
    background: rgba(20, 15, 10, 0.7);
    border: 1px solid rgba(201, 168, 76, 0.4);
    border-radius: 4px;
  }

  .top-btn:hover {
    background: rgba(139, 32, 32, 0.8);
  }

  /* ── Table: stacked on narrow screens ── */
  .table {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .controls {
    background: rgba(20, 15, 10, 0.82);
    border: 1px solid rgba(201, 168, 76, 0.4);
    border-radius: 10px;
    padding: 10px 12px;
    color: #f5ede0;
    text-align: left;
  }

  .board-area {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .back-to-board {
    align-self: center;
    padding: 6px 14px;
    min-height: 0;
    font-family: 'Cinzel', serif;
    font-size: 0.75rem;
    color: #2c1e12;
    background: #e8c766;
    border: none;
    border-radius: 999px;
    box-shadow: 0 0 16px rgba(232, 199, 102, 0.5);
  }

  .back-to-board:hover {
    background: #f3dc8a;
  }

  .board-box {
    width: 100%;
    margin: 0 auto;
  }

  /* Stacked, the board comes straight after the controls, unless claiming
     from the market is the move. */
  .market {
    order: 2;
  }

  .table.market-first .market {
    order: 0;
  }

  /* Wide screens: the market and the controls on the left, the board filling the rest. */
  @media (min-width: 900px) {
    .game-content {
      overflow: hidden;
      padding: 8px 12px 10px;
    }

    .top-bar {
      grid-template-columns: auto 1fr auto;
      grid-template-areas: 'round tabs actions';
    }

    .top-bar :global(.player-tabs) {
      justify-content: center;
    }

    .table {
      flex: 1;
      min-height: 0;
      display: grid;
      grid-template-columns: clamp(320px, 36vw, 720px) 1fr;
      grid-template-rows: auto 1fr;
      grid-template-areas:
        'market board'
        'controls board';
      gap: 12px 16px;
    }

    .market {
      grid-area: market;
      /* Leave the controls at least half the column. */
      width: min(100%, calc(48dvh * 1.5));
      justify-self: center;
    }

    .controls {
      grid-area: controls;
      min-height: 0;
      overflow-y: auto;
    }

    .board-area {
      grid-area: board;
      min-height: 0;
      min-width: 0;
    }

    .board-fit {
      flex: 1;
      min-height: 0;
      container-type: size;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .board-box {
      /* As large as fits, keeping the board's shape with cards tucked top and bottom. */
      width: min(100cqw, 100cqh * 2000 / 1618.4);
    }
  }

  /* ── Log ── */
  .log-overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .log-modal {
    background: rgba(20, 15, 10, 0.95);
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .log-modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid rgba(201, 168, 76, 0.3);
  }

  .log-modal-title {
    font-family: 'Cinzel', serif;
    font-size: 1rem;
    font-weight: 600;
    color: #c9a84c;
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  .log-close-btn {
    background: none;
    border: none;
    color: rgba(245, 237, 224, 0.6);
    font-size: 1.6rem;
    padding: 4px 8px;
    line-height: 1;
    min-width: 44px;
  }

  .log-close-btn:hover {
    color: #f5ede0;
  }

  .log-entries {
    overflow-y: auto;
    padding: 0.5rem 0;
    overscroll-behavior: none;
  }

  .log-entry {
    padding: 5px 1rem;
    font-family: 'Cormorant Garamond', serif;
    font-size: 0.9rem;
    color: rgba(245, 237, 224, 0.8);
    border-bottom: 1px solid rgba(201, 168, 76, 0.1);
  }

  .retry-btn {
    padding: 8px 20px;
    font-size: 0.8rem;
    background: #e74c3c;
    color: white;
    border: none;
    border-radius: 4px;
    min-height: 44px;
  }

  @media (min-width: 768px) {
    .log-modal {
      border: 1px solid rgba(201, 168, 76, 0.5);
      border-radius: 10px;
      width: 600px;
      max-width: 90vw;
      max-height: 70vh;
      height: auto;
    }
  }
</style>
