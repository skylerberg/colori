<script lang="ts">
  import type { GameState } from '../data/types';
  import { formatTime, orderByDraftOrder } from '../gameUtils';
  import { getBuyerData } from '../data/cards';
  import { getGlassCardData, GLASS_CARD_ORDER } from '../data/glassCards';
  import type { Snippet } from 'svelte';
  import BuyerDisplay from './BuyerDisplay.svelte';
  import ColorWheelDisplay from './ColorWheelDisplay.svelte';
  import CardList from './CardList.svelte';
  import { mixWheelState } from '../stores/mixWheelState.svelte';
  import CardModal from './CardModal.svelte';

  let { gameState, activePlayerIndex, aiThinking, elapsedSeconds, gameLog, onLeaveGame, selectedPlayerIndex = 0, onSelectPlayer, aiError, onRetryAI, hidePlayerCards = false, draftCardOrder, children }: {
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
    draftCardOrder?: number[][];
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
  let showDeckModal = $state(false);
  let showDiscardModal = $state(false);

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
      <button class="stat stat-clickable" onclick={() => showDeckModal = true}>Deck: {currentPlayer.deck.length}</button>
      <span class="stat-sep">|</span>
      <button class="stat stat-clickable" onclick={() => showDiscardModal = true}>Discard: {currentPlayer.discard.length}</button>
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

          {#if gameState.expansions?.glass}
            <div class="section-panel glass-display-panel">
              <div class="section-title">Glass Display</div>
              <div class="glass-cards">
                {#if gameState.glassDisplay.length > 0}
                  {#each gameState.glassDisplay as glass}
                    {@const data = getGlassCardData(glass.card)}
                    <div class="glass-card" title={data.description}>
                      <span class="glass-card-name">{data.name}</span>
                      <span class="glass-card-desc">{data.description}</span>
                    </div>
                  {/each}
                {:else}
                  <div class="empty-text">No glass cards available</div>
                {/if}
              </div>
            </div>
          {/if}

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

              {#if gameState.expansions?.glass}
                <div class="section-panel completed-glass-panel">
                  <div class="section-title">Completed Glass</div>
                  <div class="glass-cards">
                    {#if currentPlayer.completedGlass && currentPlayer.completedGlass.length > 0}
                      {#each currentPlayer.completedGlass as glass}
                        {@const data = getGlassCardData(glass.card)}
                        {@const used = gameState.phase.type === 'action' && (() => {
                          const idx = GLASS_CARD_ORDER.indexOf(glass.card);
                          return idx >= 0 && (gameState.phase.actionState.usedGlass & (1 << idx)) !== 0;
                        })()}
                        <div class="glass-card" class:glass-used={used} title={data.description}>
                          <span class="glass-card-name">{data.name}</span>
                          <span class="glass-card-desc">{data.description}</span>
                        </div>
                      {/each}
                    {:else}
                      <div class="empty-text">None yet</div>
                    {/if}
                  </div>
                </div>
              {/if}

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
                  wheel={selectedPlayerIndex === activePlayerIndex ? (mixWheelState.simulatedWheel ?? currentPlayer.colorWheel) : currentPlayer.colorWheel}
                  size={400}
                  interactive={selectedPlayerIndex === activePlayerIndex && !!mixWheelState.onColorClick}
                  onColorClick={selectedPlayerIndex === activePlayerIndex ? (mixWheelState.onColorClick ?? undefined) : undefined}
                  selectedColors={selectedPlayerIndex === activePlayerIndex ? mixWheelState.selectedColors : []}
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
                  <CardList cards={draftCardOrder ? orderByDraftOrder(currentPlayer.draftedCards, draftCardOrder[selectedPlayerIndex]) : currentPlayer.draftedCards} />
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

  {#if showDeckModal}
    <CardModal title="Deck" cards={currentPlayer.deck} onClose={() => showDeckModal = false} />
  {/if}
  {#if showDiscardModal}
    <CardModal title="Discard Pile" cards={currentPlayer.discard} onClose={() => showDiscardModal = false} />
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
    padding: 0.35rem 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    height: 100%;
    box-sizing: border-box;
    overflow-y: auto;
    overflow-x: hidden;
    overscroll-behavior: none;
  }

  /* Top info bar */
  .top-info-bar {
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .round-indicator {
    font-family: 'Cinzel', serif;
    font-size: 0.75rem;
    font-weight: 600;
    color: #c9a84c;
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  .top-bar-actions {
    position: static;
    flex-shrink: 0;
    display: flex;
    gap: 6px;
  }

  .top-btn {
    padding: 6px 10px;
    font-size: 0.65rem;
    font-family: 'Cinzel', serif;
    color: #f5ede0;
    border-radius: 4px;
    cursor: pointer;
    min-height: 36px;
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
    border: none;
    border-radius: 0;
    width: 100%;
    max-width: 100%;
    max-height: 100%;
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
    cursor: pointer;
    padding: 4px 8px;
    line-height: 1;
    min-height: 44px;
    min-width: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
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
    justify-content: flex-start;
    gap: 4px;
    overflow-x: auto;
    overflow-y: hidden;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: none;
    flex-wrap: nowrap;
    padding-bottom: 4px;
  }

  .player-tabs::-webkit-scrollbar {
    display: none;
  }

  .player-tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 8px;
    font-family: 'Cinzel', serif;
    font-size: 0.8rem;
    font-weight: 600;
    color: rgba(245, 237, 224, 0.4);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
    outline: none;
    border-radius: 0;
    min-height: 44px;
    white-space: nowrap;
    flex-shrink: 0;
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
    flex-wrap: wrap;
    gap: 4px 10px;
    font-family: 'Cormorant Garamond', serif;
    font-size: 1.05rem;
    color: rgba(245, 237, 224, 0.8);
    margin-top: 2px;
  }

  .stat-clickable {
    background: none;
    border: none;
    cursor: pointer;
    font: inherit;
    color: inherit;
    padding: 2px 6px;
    min-height: 36px;
    display: inline-flex;
    align-items: center;
  }

  .stat {
    padding: 2px 6px;
    min-height: 36px;
    display: inline-flex;
    align-items: center;
  }

  .stat-clickable:hover {
    color: #c9a84c;
    text-decoration: underline;
  }

  .stat-sep {
    color: rgba(201, 168, 76, 0.4);
    display: none;
  }

  /* Section panels */
  .section-panel {
    background: rgba(20, 15, 10, 0.75);
    border: 1px solid rgba(201, 168, 76, 0.4);
    border-radius: 8px;
    padding: 0.5rem;
  }

  .section-title {
    font-family: 'Cinzel', serif;
    color: #c9a84c;
    text-transform: uppercase;
    font-size: 0.75rem;
    letter-spacing: 0.1em;
    margin-bottom: 0.35rem;
    font-weight: 600;
  }

  /* Two-column layout — mobile-first: single column stacked */
  .main-columns {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    overscroll-behavior: none;
  }

  .left-col {
    flex: none;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    min-height: 0;
    overflow-y: visible;
    overflow-x: hidden;
  }

  .player-info-row {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .info-left-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    min-width: 120px;
  }

  .player-info-row > .color-wheel-panel {
    flex: none;
    width: 100%;
  }

  .right-col {
    flex: none;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    min-height: 0;
    overflow-y: visible;
    overflow-x: hidden;
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
    max-width: 220px;
    margin: 0 auto;
    width: 100%;
  }

  .wheel-container :global(svg) {
    width: 100%;
    height: auto;
  }

  .completed-buyers-content {
    height: auto;
    max-height: calc(var(--card-height, 126px) + 32px);
    overflow-y: auto;
  }

  .drafted-cards-content {
    height: auto;
    max-height: calc(var(--card-height, 126px) + 32px);
    overflow-y: auto;
  }

  .completed-buyers-panel :global(.card-list) {
    min-height: auto;
    flex-wrap: wrap;
  }

  /* Materials */
  .materials-grid {
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    gap: 6px;
    padding: 8px 0;
  }

  .material-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 8px;
    background: rgba(40, 28, 16, 0.5);
    border-radius: 6px;
    flex: 1 1 auto;
    min-width: 80px;
  }

  .material-count {
    font-family: 'Cinzel', serif;
    font-size: 1.1rem;
    font-weight: bold;
    color: #c9a84c;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.7);
    min-width: 2ch;
    text-align: right;
  }

  .material-label {
    font-family: 'Cormorant Garamond', serif;
    font-size: 0.75rem;
    color: #c9a84c;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .glass-cards {
    display: flex;
    flex-wrap: nowrap;
    gap: 6px;
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: none;
    padding-bottom: 4px;
  }

  .glass-cards::-webkit-scrollbar {
    display: none;
  }

  .glass-card {
    display: flex;
    flex-direction: column;
    padding: 4px 10px;
    font-family: 'Cormorant Garamond', serif;
    color: rgba(245, 237, 224, 0.9);
    background: rgba(100, 160, 200, 0.25);
    border: 1px solid rgba(100, 160, 200, 0.5);
    border-radius: 4px;
    flex-shrink: 0;
  }

  .glass-card-name {
    font-size: 0.8rem;
    font-weight: 600;
  }

  .glass-card-desc {
    font-size: 0.65rem;
    color: rgba(245, 237, 224, 0.5);
  }

  .glass-used {
    opacity: 0.4;
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
    padding: 8px 20px;
    font-size: 0.8rem;
    background: #e74c3c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    min-height: 44px;
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

  /* ===== RESPONSIVE OVERRIDES (mobile-first) ===== */

  /* Tablet (768px+) */
  @media (min-width: 768px) {
    .game-content {
      padding: 0.5rem 0.75rem;
      gap: 0.5rem;
    }

    .top-info-bar {
      flex-wrap: nowrap;
    }

    .top-bar-actions {
      position: absolute;
      right: 0;
    }

    .top-btn {
      padding: 4px 12px;
      font-size: 0.7rem;
      min-height: unset;
    }

    .round-indicator {
      font-size: 0.85rem;
    }

    .player-tabs {
      gap: 12px;
      justify-content: center;
      overflow-x: visible;
    }

    .player-tab {
      font-size: 0.95rem;
      padding: 6px 4px;
    }

    .player-stats {
      gap: 12px;
      font-size: 1.3rem;
    }

    .stat-sep {
      display: inline;
    }

    .stat, .stat-clickable {
      padding: 0;
      min-height: unset;
      display: inline;
    }

    .section-panel {
      padding: 0.75rem;
    }

    .section-title {
      font-size: 0.85rem;
      margin-bottom: 0.5rem;
    }

    .player-info-row {
      flex-direction: row;
    }

    .player-info-row > .color-wheel-panel {
      flex: 1 1 0;
    }

    .info-left-col {
      flex: 1 1 0;
    }

    .wheel-container {
      max-width: 250px;
    }

    .materials-grid {
      flex-direction: column;
      gap: 12px;
    }

    .material-item {
      flex: unset;
      min-width: unset;
      padding: 6px 12px;
    }

    .material-count {
      font-size: 1.4rem;
    }

    .material-label {
      font-size: 0.85rem;
    }

    .glass-cards {
      flex-wrap: wrap;
      overflow-x: visible;
    }

    .completed-buyers-content {
      height: calc(var(--card-height, 154px) + 28px);
      max-height: none;
    }

    .drafted-cards-content {
      height: calc(var(--card-height, 154px) + 28px);
      max-height: none;
    }

    .log-modal {
      background: rgba(20, 15, 10, 0.95);
      border: 1px solid rgba(201, 168, 76, 0.5);
      border-radius: 10px;
      width: 600px;
      max-width: 90vw;
      max-height: 70vh;
      height: auto;
    }

    .log-close-btn {
      font-size: 1.4rem;
      padding: 0 4px;
      min-height: unset;
      min-width: unset;
      display: inline;
    }

    .ai-error-banner {
      padding: 12px 16px;
    }

    .retry-btn {
      min-height: unset;
      padding: 6px 16px;
    }
  }

  /* Desktop (1024px+): restore two-column layout */
  @media (min-width: 1024px) {
    .game-content {
      overflow: hidden;
    }

    .main-columns {
      flex-direction: row;
      overflow: hidden;
    }

    .left-col {
      flex: 0 0 50%;
      overflow-y: auto;
    }

    .right-col {
      flex: 1;
      overflow-y: auto;
    }

    .wheel-container {
      max-width: 320px;
    }

    .completed-buyers-content {
      height: calc(var(--card-height, 175px) + 28px);
    }

    .drafted-cards-content {
      height: calc(var(--card-height, 175px) + 28px);
    }
  }
</style>
