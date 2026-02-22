<script lang="ts">
  import type { GameState } from './data/types';
  import SetupScreen from './components/SetupScreen.svelte';
  import GameScreen from './components/GameScreen.svelte';
  import ScoreScreen from './components/ScoreScreen.svelte';
  import { saveGame, loadGame, clearSavedGame } from './persistence';

  const saved = loadGame();
  let gameState: GameState | null = $state(saved?.gameState ?? null);
  let gameStartTime: number | null = $state(saved?.gameStartTime ?? null);
  let savedGameLog: string[] = $state(saved?.gameLog ?? []);

  function handleGameStarted(state: GameState) {
    gameState = state;
    gameStartTime = Date.now();
    savedGameLog = [];
    saveGame(state, gameStartTime!, []);
  }

  function handleGameUpdated(state: GameState, log: string[]) {
    gameState = state;
    if (state.phase.type === 'gameOver') {
      clearSavedGame();
    } else {
      saveGame(state, gameStartTime!, log);
    }
  }

  function handlePlayAgain() {
    gameState = null;
    gameStartTime = null;
    savedGameLog = [];
    clearSavedGame();
  }
</script>

<main>
  <h1>Colori</h1>
  {#if gameState === null}
    <SetupScreen onGameStarted={handleGameStarted} />
  {:else if gameState.phase.type === 'gameOver'}
    <ScoreScreen {gameState} {gameStartTime} onPlayAgain={handlePlayAgain} />
  {:else}
    <GameScreen {gameState} {gameStartTime} onGameUpdated={handleGameUpdated} initialGameLog={savedGameLog} />
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
