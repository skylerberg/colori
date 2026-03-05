<script lang="ts">
  import type { GameState, PlayerState, Choice } from '../data/types';
  import Scene from './3d/Scene.svelte';

  let { gameState, activePlayerIndex, aiThinking, elapsedSeconds, gameLog, onLeaveGame, sidebarPlayer, selectedPlayerIndex, onSelectPlayer, onAction }: {
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
  } = $props();

  function handleLeaveGame() {
    if (confirm('Are you sure you want to leave this game? Your progress will be lost.')) {
      onLeaveGame();
    }
  }
</script>

<div class="game-3d-container">
  <div class="canvas-layer">
    <Scene {gameState} {activePlayerIndex} {onAction} {elapsedSeconds} />
  </div>

  <button class="leave-btn-3d" onclick={handleLeaveGame}>Leave</button>
</div>

<style>
  .game-3d-container {
    position: fixed;
    inset: 0;
    background: #d8e4f0;
    z-index: 100;
  }

  .canvas-layer {
    position: absolute;
    inset: 0;
  }

  .leave-btn-3d {
    position: fixed;
    top: 12px;
    right: 12px;
    padding: 4px 12px;
    font-size: 0.75rem;
    background: rgba(231, 76, 60, 0.85);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    backdrop-filter: blur(4px);
    z-index: 10;
  }

  .leave-btn-3d:hover {
    background: #c0392b;
  }
</style>
