<script lang="ts">
  import type { GameState } from './data/types';
  import SetupScreen from './components/SetupScreen.svelte';
  import GameScreen from './components/GameScreen.svelte';
  import ScoreScreen from './components/ScoreScreen.svelte';

  let gameState: GameState | null = $state(null);
  let gameStartTime: number | null = $state(null);

  function handleGameStarted(state: GameState) {
    gameState = state;
    gameStartTime = Date.now();
  }

  function handleGameUpdated(state: GameState) {
    gameState = state;
  }

  function handlePlayAgain() {
    gameState = null;
    gameStartTime = null;
  }
</script>

<main>
  <h1>Colori</h1>
  {#if gameState === null}
    <SetupScreen onGameStarted={handleGameStarted} />
  {:else if gameState.phase.type === 'gameOver'}
    <ScoreScreen {gameState} {gameStartTime} onPlayAgain={handlePlayAgain} />
  {:else}
    <GameScreen {gameState} {gameStartTime} onGameUpdated={handleGameUpdated} />
  {/if}
</main>

<style>
  main {
    text-align: center;
  }
  h1 {
    font-size: 2rem;
    margin-bottom: 1rem;
    color: #4a3728;
  }
</style>
