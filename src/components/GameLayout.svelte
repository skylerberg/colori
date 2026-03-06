<script lang="ts">
  import type { GameState, PlayerState } from '../data/types';
  import { formatTime } from '../gameUtils';
  import type { Snippet } from 'svelte';
  import PlayerStatus from './PlayerStatus.svelte';
  import GameLog from './GameLog.svelte';
  import BuyerDisplay from './BuyerDisplay.svelte';
  import PlayerTableau from './PlayerTableau.svelte';

  let { gameState, activePlayerIndex, aiThinking, elapsedSeconds, gameLog, onLeaveGame, sidebarPlayer, selectedPlayerIndex, onSelectPlayer, aiError, onRetryAI, children }: {
    gameState: GameState;
    activePlayerIndex: number;
    aiThinking: boolean;
    elapsedSeconds: number;
    gameLog: string[];
    onLeaveGame: () => void;
    sidebarPlayer: PlayerState | null;
    selectedPlayerIndex?: number;
    onSelectPlayer?: (index: number) => void;
    aiError?: string | null;
    onRetryAI?: () => void;
    children: Snippet;
  } = $props();

  let showSidebar = $derived(
    sidebarPlayer !== null &&
    (gameState.phase.type === 'draft' || gameState.phase.type === 'action')
  );

  function handleLeaveGame() {
    if (confirm('Are you sure you want to leave this game? Your progress will be lost.')) {
      onLeaveGame();
    }
  }
</script>

<div class="game-screen">
  <div class="top-bar">
    <div class="top-bar-row">
      <div class="round-indicator">Round {gameState.round} &mdash; {formatTime(elapsedSeconds)}</div>
      <button class="leave-btn" onclick={handleLeaveGame}>Leave Game</button>
    </div>
    <div class="player-bar">
      {#each gameState.players as player, i}
        <PlayerStatus {player} playerName={gameState.playerNames[i]} active={i === activePlayerIndex} selected={selectedPlayerIndex !== undefined && i === selectedPlayerIndex} isAI={gameState.aiPlayers[i]} thinking={aiThinking && i === activePlayerIndex} onclick={onSelectPlayer ? () => onSelectPlayer(i) : undefined} />
      {/each}
    </div>
  </div>

  <div class="game-body">
    <div class="main-area">
      {#if aiError}
        <div class="ai-error-banner">
          <strong>AI encountered an error</strong>
          <pre>{aiError}</pre>
          {#if onRetryAI}
            <button class="retry-btn" onclick={onRetryAI}>Retry</button>
          {/if}
        </div>
      {/if}

      <div class="phase-content">
        {@render children()}
      </div>

      {#if showSidebar && sidebarPlayer}
        <div class="tableau-wrapper">
          <PlayerTableau
            player={sidebarPlayer}
            draftedCards={sidebarPlayer.draftedCards}
            workshopCards={sidebarPlayer.workshopCards}
            completedBuyers={sidebarPlayer.completedBuyers}
          />
        </div>
      {/if}
    </div>

    {#if showSidebar}
      <aside class="sidebar">
        <BuyerDisplay buyers={gameState.buyerDisplay} />
      </aside>
    {/if}
  </div>

  {#if gameLog.length > 0}
    <GameLog entries={gameLog} />
  {/if}
</div>

<style>
  .game-screen {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .top-bar {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding-bottom: 4px;
    border-bottom: 2px solid var(--border-gold, rgba(201, 168, 76, 0.3));
  }

  .top-bar-row {
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
  }

  .round-indicator {
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-primary, #2c1e12);
    text-align: center;
  }

  .leave-btn {
    position: absolute;
    right: 0;
    padding: 3px 10px;
    font-size: 0.7rem;
    font-family: var(--font-display, 'Cinzel', serif);
    background: var(--accent-crimson, #8b2020);
    color: var(--text-on-dark, #f5ede0);
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .leave-btn:hover {
    background: #6b1818;
  }

  .player-bar {
    display: flex;
    gap: 6px;
    overflow-x: auto;
    justify-content: center;
    flex-wrap: wrap;
  }

  .game-body {
    display: flex;
    gap: 0.75rem;
  }

  .main-area {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
  }

  .phase-content {
    width: 100%;
  }

  .tableau-wrapper {
    width: 100%;
    max-width: 700px;
    padding: 80px 0 40px 0;
  }

  .sidebar {
    width: 220px;
    flex-shrink: 0;
    border-left: 2px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    padding-left: 0.75rem;
  }

  .ai-error-banner {
    background: #fef2f2;
    border: 1px solid #e74c3c;
    border-radius: 8px;
    padding: 12px 16px;
    width: 100%;
    box-sizing: border-box;
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
</style>
