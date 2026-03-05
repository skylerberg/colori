<script lang="ts">
  import type { GameState, PlayerState, Choice } from '../data/types';
  import { formatTime } from '../gameUtils';
  import type { Snippet } from 'svelte';
  import PlayerStatus from './PlayerStatus.svelte';
  import GameLog from './GameLog.svelte';
  import ColorWheelDisplay from './ColorWheelDisplay.svelte';
  import BuyerDisplay from './BuyerDisplay.svelte';
  import GameLayout3D from './GameLayout3D.svelte';
  import { getViewMode } from '../stores/viewMode.svelte';

  let { gameState, activePlayerIndex, aiThinking, elapsedSeconds, gameLog, onLeaveGame, sidebarPlayer, selectedPlayerIndex, onSelectPlayer, onAction, children }: {
    gameState: GameState;
    activePlayerIndex: number;
    aiThinking: boolean;
    elapsedSeconds: number;
    gameLog: string[];
    onLeaveGame: () => void;
    sidebarPlayer: PlayerState | null;
    selectedPlayerIndex?: number;
    onSelectPlayer?: (index: number) => void;
    onAction?: (choice: Choice) => void;
    children: Snippet;
  } = $props();

  let viewMode = $derived(getViewMode());

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

{#if viewMode === '3d'}
  <GameLayout3D {gameState} {activePlayerIndex} {aiThinking} {elapsedSeconds} {gameLog} {onLeaveGame} {sidebarPlayer} {selectedPlayerIndex} {onSelectPlayer} {onAction}>
    {@render children()}
  </GameLayout3D>
{:else}
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
    {#if showSidebar && sidebarPlayer}
      <aside class="left-sidebar">
        <div class="sidebar-section">
          <h3>Color Wheel</h3>
          <ColorWheelDisplay wheel={sidebarPlayer.colorWheel} size={160} />
        </div>

        <div class="sidebar-section">
          <h3>Stored Materials</h3>
          <div class="material-counts">
            {#each Object.entries(sidebarPlayer.materials) as [material, count]}
              <span class="material-count">{material}: {count}</span>
            {/each}
          </div>
          {#if sidebarPlayer.ducats > 0}
            <div class="ducats-count">Ducats: {sidebarPlayer.ducats}</div>
          {/if}
        </div>
      </aside>
    {/if}

    <div class="main-column">
      <div class="phase-content">
        {@render children()}
      </div>

      {#if gameLog.length > 0}
        <GameLog entries={gameLog} />
      {/if}
    </div>

    {#if showSidebar}
      <aside class="sidebar">
        <div class="sidebar-section">
          <BuyerDisplay buyers={gameState.buyerDisplay} />
        </div>
      </aside>
    {/if}
  </div>
</div>
{/if}

<style>
  .game-screen {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .top-bar {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-bottom: 8px;
    border-bottom: 2px solid #e0d5c5;
  }

  .top-bar-row {
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
  }

  .round-indicator {
    font-size: 0.85rem;
    font-weight: 600;
    color: #4a3728;
    text-align: center;
  }

  .leave-btn {
    position: absolute;
    right: 0;
    padding: 4px 12px;
    font-size: 0.75rem;
    background: #e74c3c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .leave-btn:hover {
    background: #c0392b;
  }

  .player-bar {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    justify-content: center;
    flex-wrap: wrap;
  }

  .game-body {
    display: flex;
    gap: 1rem;
  }

  .main-column {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .phase-content {
    min-height: 300px;
  }

  .left-sidebar {
    width: 250px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .sidebar {
    width: 250px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .sidebar-section {
    border: 1px solid #ddd;
    border-radius: 8px;
    padding: 10px 12px;
    background: #fff;
  }

  .sidebar-section h3 {
    font-size: 0.85rem;
    color: #4a3728;
    margin-bottom: 6px;
  }

  .material-counts {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.8rem;
    color: #8b6914;
  }

  .material-count {
    font-weight: 600;
  }

  .ducats-count {
    font-size: 0.8rem;
    color: #d4a017;
    font-weight: 600;
    margin-top: 4px;
  }
</style>
