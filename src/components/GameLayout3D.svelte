<script lang="ts">
  import type { GameState, PlayerState, Choice } from '../data/types';
  import type { Snippet } from 'svelte';
  import { formatTime } from '../gameUtils';
  import PlayerStatus from './PlayerStatus.svelte';
  import GameLog from './GameLog.svelte';
  import Scene from './3d/Scene.svelte';

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

  function handleLeaveGame() {
    if (confirm('Are you sure you want to leave this game? Your progress will be lost.')) {
      onLeaveGame();
    }
  }
</script>

<div class="game-3d-container">
  <!-- Layer 1: Threlte 3D Canvas -->
  <div class="canvas-layer">
    <Scene {gameState} {activePlayerIndex} {onAction} />
  </div>

  <!-- Layer 2: HTML Overlay -->
  <div class="overlay-layer">
    <!-- Top bar -->
    <div class="overlay-top">
      <div class="top-bar-3d">
        <div class="top-bar-row-3d">
          <div class="round-indicator-3d">Round {gameState.round} &mdash; {formatTime(elapsedSeconds)}</div>
          <button class="leave-btn-3d" onclick={handleLeaveGame}>Leave Game</button>
        </div>
        <div class="player-bar-3d">
          {#each gameState.players as player, i}
            <PlayerStatus {player} playerName={gameState.playerNames[i]} active={i === activePlayerIndex} selected={selectedPlayerIndex !== undefined && i === selectedPlayerIndex} isAI={gameState.aiPlayers[i]} thinking={aiThinking && i === activePlayerIndex} onclick={onSelectPlayer ? () => onSelectPlayer(i) : undefined} />
          {/each}
        </div>
      </div>
    </div>

    <!-- Middle: phase-specific prompts -->
    <div class="overlay-middle">
      {@render children()}
    </div>

    <!-- Bottom: game log -->
    <div class="overlay-bottom">
      {#if gameLog.length > 0}
        <GameLog entries={gameLog} />
      {/if}
    </div>
  </div>
</div>

<style>
  .game-3d-container {
    position: fixed;
    inset: 0;
    background: #1a1410;
    z-index: 100;
  }

  .canvas-layer {
    position: absolute;
    inset: 0;
  }

  .overlay-layer {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    pointer-events: none;
    z-index: 1;
  }

  .overlay-top {
    pointer-events: auto;
    padding: 8px 12px;
    background: linear-gradient(to bottom, rgba(26, 20, 16, 0.85), transparent);
  }

  .top-bar-3d {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .top-bar-row-3d {
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
  }

  .round-indicator-3d {
    font-size: 0.85rem;
    font-weight: 600;
    color: #ffe8cc;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
  }

  .leave-btn-3d {
    position: absolute;
    right: 0;
    padding: 4px 12px;
    font-size: 0.75rem;
    background: rgba(231, 76, 60, 0.85);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    backdrop-filter: blur(4px);
  }

  .leave-btn-3d:hover {
    background: #c0392b;
  }

  .player-bar-3d {
    display: flex;
    gap: 8px;
    justify-content: center;
    flex-wrap: wrap;
  }

  .overlay-middle {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
  }

  .overlay-middle > :global(*) {
    pointer-events: auto;
  }

  .overlay-bottom {
    pointer-events: auto;
    padding: 0 12px 8px;
    background: linear-gradient(to top, rgba(26, 20, 16, 0.85), transparent);
    max-height: 200px;
    overflow-y: auto;
  }
</style>
