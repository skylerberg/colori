<script lang="ts">
  import type { GameState } from '../data/types';
  import { formatTime } from '../gameUtils';
  import { getBuyerData } from '../data/cards';
  import type { Snippet } from 'svelte';
  import BuyerDisplay from './BuyerDisplay.svelte';
  import ColorWheelDisplay from './ColorWheelDisplay.svelte';
  import CardList from './CardList.svelte';
  import { mixWheelState } from '../stores/mixWheelState.svelte';

  let { gameState, activePlayerIndex, aiThinking, elapsedSeconds, gameLog, onLeaveGame, selectedPlayerIndex = 0, onSelectPlayer, aiError, onRetryAI, hidePlayerCards = false, children }: {
    gameState: GameState;
    activePlayerIndex: number;
    aiThinking: boolean;
    elapsedSeconds: number;
    gameLog: string[];
    onLeaveGame: () => void;
    selectedPlayerIndex?: number;
    onSelectPlayer?: (index: number) => void;
    aiError?: string | null;
    onRetryAI?: () => void;
    hidePlayerCards?: boolean;
    children: Snippet;
  } = $props();

  let showContent = $derived(
    gameState.phase.type === 'draft' || gameState.phase.type === 'action'
  );

  // Current player data
  let currentPlayer = $derived(gameState.players[selectedPlayerIndex]);
  let currentPlayerName = $derived(gameState.playerNames[selectedPlayerIndex]);
  let isAI = $derived(gameState.aiPlayers[selectedPlayerIndex]);
  let isActive = $derived(selectedPlayerIndex === activePlayerIndex);

  // Stats
  let score = $derived(currentPlayer.completedBuyers.reduce((sum, buyer) => sum + getBuyerData(buyer.card).stars, 0) + currentPlayer.ducats);

  let showLog = $state(false);

  function handleLeaveGame() {
    if (confirm('Are you sure you want to leave this game? Your progress will be lost.')) {
      onLeaveGame();
    }
  }

</script>

<div class="game-screen">
  <div class="game-overlay"></div>

  <div class="game-content">
    <!-- Top info bar -->
    <div class="top-info-bar">
      <div class="round-indicator">Round {gameState.round} &mdash; {formatTime(elapsedSeconds)}</div>
      <div class="top-bar-actions">
        {#if gameLog.length > 0}
          <button class="top-btn log-btn" onclick={() => showLog = !showLog}>Game Log ({gameLog.length})</button>
        {/if}
        <button class="top-btn leave-btn" onclick={handleLeaveGame}>Leave Game</button>
      </div>
    </div>

    <!-- Player tabs -->
    <div class="player-tabs">
      {#each gameState.players as _, i}
        <button
          class="player-tab"
          class:active={i === selectedPlayerIndex}
          class:is-turn={i === activePlayerIndex}
          onclick={() => onSelectPlayer?.(i)}
        >
          {#if i === activePlayerIndex}<span class="turn-dot"></span>{/if}
          {gameState.playerNames[i]}
          {#if gameState.aiPlayers[i]}<span class="ai-badge">AI</span>{/if}
          {#if aiThinking && i === activePlayerIndex}<span class="thinking-spinner"></span>{/if}
        </button>
      {/each}
    </div>

    <!-- Selected player stats -->
    <div class="player-stats">
      <span class="stat">Score: {score}</span>
      <span class="stat-sep">|</span>
      <span class="stat">Ducats: {currentPlayer.ducats}</span>
      <span class="stat-sep">|</span>
      <span class="stat">Deck: {currentPlayer.deck.length}</span>
      <span class="stat-sep">|</span>
      <span class="stat">Discard: {currentPlayer.discard.length}</span>
    </div>

    {#if aiError}
      <div class="ai-error-banner">
        <strong>AI encountered an error</strong>
        <pre>{aiError}</pre>
        {#if onRetryAI}
          <button class="retry-btn" onclick={onRetryAI}>Retry</button>
        {/if}
      </div>
    {/if}

    {#if showContent}
      <div class="main-columns">
        <!-- Left half: buyer display + player info row -->
        <div class="left-col">
          <BuyerDisplay buyers={gameState.buyerDisplay} />

          <div class="player-info-row">
            <div class="info-left-col">
              <div class="section-panel completed-buyers-panel">
                <div class="section-title">Completed Buyers</div>
                <div class="completed-buyers-content">
                  {#if currentPlayer.completedBuyers.length > 0}
                    <CardList cards={currentPlayer.completedBuyers} />
                  {:else}
                    <div class="empty-text">None yet</div>
                  {/if}
                </div>
              </div>

              <div class="section-panel materials-panel">
                <div class="section-title">Materials</div>
                <div class="materials-grid">
                  <div class="material-item">
                    <span class="material-count">{currentPlayer.materials.Textiles}</span>
                    <span class="material-label">Textiles</span>
                  </div>
                  <div class="material-item">
                    <span class="material-count">{currentPlayer.materials.Ceramics}</span>
                    <span class="material-label">Ceramics</span>
                  </div>
                  <div class="material-item">
                    <span class="material-count">{currentPlayer.materials.Paintings}</span>
                    <span class="material-label">Paintings</span>
                  </div>
                </div>
              </div>
            </div>

            <div class="section-panel color-wheel-panel">
              <div class="section-title">Color Wheel</div>
              <div class="wheel-container">
                <ColorWheelDisplay
                  wheel={mixWheelState.simulatedWheel ?? currentPlayer.colorWheel}
                  size={400}
                  interactive={!!mixWheelState.onColorClick}
                  onColorClick={mixWheelState.onColorClick ?? undefined}
                  selectedColors={mixWheelState.selectedColors}
                />
              </div>
            </div>
          </div>
        </div>

        <!-- Right main area: cards -->
        <div class="right-col">

          <div class="section-panel draft-section">
            {@render children()}
          </div>

          {#if !hidePlayerCards}
            <div class="section-panel drafted-section">
              <div class="section-title">Drafted Cards</div>
              <div class="drafted-cards-content">
                {#if currentPlayer.draftedCards.length > 0}
                  <CardList cards={currentPlayer.draftedCards} />
                {:else}
                  <div class="empty-text">None yet</div>
                {/if}
              </div>
            </div>

            <div class="section-panel workshop-section">
              <div class="section-title">Workshop</div>
              <CardList
                cards={[...currentPlayer.workshopCards, ...currentPlayer.workshoppedCards]}
                rotatedIds={currentPlayer.workshoppedCards.map(c => c.instanceId)}
              />
            </div>
          {/if}
        </div>
      </div>
    {:else}
      <div class="section-panel draft-section">
        {@render children()}
      </div>
    {/if}

  </div>

  {#if showLog}
    <div class="log-overlay" onclick={() => showLog = false}>
      <div class="log-modal" onclick={(e) => e.stopPropagation()}>
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
    background: rgba(10, 8, 5, 0.6);
    pointer-events: none;
    z-index: 0;
  }

  .game-content {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: none;
    padding: 0.5rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    height: 100vh;
    box-sizing: border-box;
    overflow: hidden;
  }

  /* Top info bar */
  .top-info-bar {
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
  }

  .round-indicator {
    font-family: 'Cinzel', serif;
    font-size: 0.85rem;
    font-weight: 600;
    color: #c9a84c;
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  .top-bar-actions {
    position: absolute;
    right: 0;
    display: flex;
    gap: 6px;
  }

  .top-btn {
    padding: 4px 12px;
    font-size: 0.7rem;
    font-family: 'Cinzel', serif;
    color: #f5ede0;
    border-radius: 4px;
    cursor: pointer;
  }

  .log-btn,
  .leave-btn {
    background: rgba(139, 32, 32, 0.8);
    border: 1px solid rgba(139, 32, 32, 0.6);
  }

  .log-btn:hover,
  .leave-btn:hover {
    background: rgba(107, 24, 24, 0.9);
  }

  /* Game Log Modal */
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
    border: 1px solid rgba(201, 168, 76, 0.5);
    border-radius: 10px;
    width: 500px;
    max-width: 90vw;
    max-height: 70vh;
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
    font-size: 1.4rem;
    cursor: pointer;
    padding: 0 4px;
    line-height: 1;
  }

  .log-close-btn:hover {
    color: #f5ede0;
  }

  .log-entries {
    overflow-y: auto;
    padding: 0.5rem 0;
  }

  .log-entry {
    padding: 5px 1rem;
    font-family: 'Cormorant Garamond', serif;
    font-size: 0.85rem;
    color: rgba(245, 237, 224, 0.8);
    border-bottom: 1px solid rgba(201, 168, 76, 0.1);
  }

  .log-entry:last-child {
    border-bottom: none;
  }

  /* Player tabs */
  .player-tabs {
    display: flex;
    justify-content: center;
    gap: 24px;
  }

  .player-tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 2px;
    font-family: 'Cinzel', serif;
    font-size: 0.95rem;
    font-weight: 600;
    color: rgba(245, 237, 224, 0.4);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
    outline: none;
    border-radius: 0;
  }

  .player-tab:hover {
    color: rgba(245, 237, 224, 0.7);
  }

  .player-tab.active {
    color: #f5ede0;
    border-bottom-color: #c9a84c;
  }

  .player-tab.is-turn {
    color: rgba(201, 168, 76, 0.6);
  }

  .player-tab.active.is-turn {
    color: #c9a84c;
  }

  .turn-dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #c9a84c;
    box-shadow: 0 0 6px rgba(201, 168, 76, 0.6);
  }

  .ai-badge {
    font-size: 0.6rem;
    font-weight: 700;
    background: rgba(139, 32, 32, 0.8);
    color: #f5ede0;
    padding: 1px 5px;
    border-radius: 3px;
  }

  .thinking-spinner {
    display: inline-block;
    width: 12px;
    height: 12px;
    border: 2px solid rgba(201, 168, 76, 0.3);
    border-top-color: #c9a84c;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .player-stats {
    display: flex;
    justify-content: center;
    gap: 12px;
    font-family: 'Cormorant Garamond', serif;
    font-size: 1.3rem;
    color: rgba(245, 237, 224, 0.8);
    margin-top: 2px;
  }

  .stat-sep {
    color: rgba(201, 168, 76, 0.4);
  }

  /* Section panels */
  .section-panel {
    background: rgba(20, 15, 10, 0.75);
    border: 1px solid rgba(201, 168, 76, 0.4);
    border-radius: 8px;
    padding: 0.75rem;
  }

  .section-title {
    font-family: 'Cinzel', serif;
    color: #c9a84c;
    text-transform: uppercase;
    font-size: 0.85rem;
    letter-spacing: 0.1em;
    margin-bottom: 0.5rem;
    font-weight: 600;
  }

  /* Two-column layout */
  .main-columns {
    display: flex;
    gap: 0.75rem;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .left-col {
    flex: 0 0 50%;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    min-height: 0;
    overflow-y: auto;
  }

  .player-info-row {
    display: flex;
    gap: 0.5rem;
  }

  .info-left-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    min-width: 0;
  }

  .player-info-row > .color-wheel-panel {
    flex: 0 0 auto;
  }

  .right-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    min-height: 0;
    overflow-y: auto;
  }

  /* Draft / prompt section */
  .draft-section {
    /* Container for draft or action prompts */
  }

  .color-wheel-panel {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .wheel-container {
    display: flex;
    justify-content: center;
    align-items: center;
    flex: 1;
  }

  .completed-buyers-content {
    height: calc(var(--card-height, 175px) + 28px);
    overflow-y: auto;
  }

  .drafted-cards-content {
    height: calc(var(--card-height, 175px) + 28px);
    overflow-y: auto;
  }

  .completed-buyers-panel :global(.card-list) {
    min-height: auto;
    flex-wrap: wrap;
  }

  /* Materials */
  .materials-grid {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 8px 0;
  }

  .material-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 12px;
    background: rgba(40, 28, 16, 0.5);
    border-radius: 6px;
  }

  .material-count {
    font-family: 'Cinzel', serif;
    font-size: 1.4rem;
    font-weight: bold;
    color: #c9a84c;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.7);
    min-width: 2ch;
    text-align: right;
  }

  .material-label {
    font-family: 'Cormorant Garamond', serif;
    font-size: 0.85rem;
    color: #c9a84c;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .empty-text {
    color: rgba(245, 237, 224, 0.4);
    font-style: italic;
    font-size: 0.85rem;
    padding: 8px 0;
  }

  /* Workshop */
  .workshop-section :global(.card-list) {
    min-height: auto;
  }


  /* AI Error */
  .ai-error-banner {
    background: rgba(254, 242, 242, 0.9);
    border: 1px solid #e74c3c;
    border-radius: 8px;
    padding: 12px 16px;
  }

  .ai-error-banner strong {
    color: #c0392b;
    font-size: 0.9rem;
  }

  .ai-error-banner pre {
    margin: 8px 0;
    padding: 8px;
    background: #fff;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 0.75rem;
    white-space: pre-wrap;
    word-break: break-all;
    user-select: all;
  }

  .retry-btn {
    padding: 6px 16px;
    font-size: 0.8rem;
    background: #e74c3c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .retry-btn:hover {
    background: #c0392b;
  }

  /* Global overrides for child component text colors on dark background */
  .game-content :global(h2),
  .game-content :global(h3) {
    color: #c9a84c;
  }

  .game-content :global(.section-box) {
    background: rgba(20, 15, 10, 0.6);
    border-color: rgba(201, 168, 76, 0.4);
  }

  .game-content :global(.section-box h3) {
    color: #c9a84c;
  }

  .game-content :global(.empty) {
    color: rgba(245, 237, 224, 0.4);
  }

  .game-content :global(.card-list .empty) {
    color: rgba(245, 237, 224, 0.4);
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
