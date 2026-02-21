<script lang="ts">
  import type { GameState } from '../data/types';
  import { createInitialGameState } from '../engine/setupPhase';

  let { onGameStarted }: {
    onGameStarted: (state: GameState) => void;
  } = $props();

  let playerCount = $state(2);
  let playerNames: string[] = $state(['Player 1', 'Player 2', 'Player 3', 'Player 4', 'Player 5']);

  function updatePlayerCount(count: number) {
    playerCount = count;
  }

  function handleNameInput(index: number, event: Event) {
    const target = event.target as HTMLInputElement;
    playerNames[index] = target.value;
  }

  function startGame() {
    const names = playerNames.slice(0, playerCount).map((n, i) => n.trim() || `Player ${i + 1}`);
    const state = createInitialGameState(names);
    onGameStarted(state);
  }
</script>

<div class="setup-screen">
  <h2>New Game</h2>

  <div class="player-count-section">
    <!-- svelte-ignore a11y_label_has_associated_control -->
    <label>Number of Players:</label>
    <div class="count-buttons">
      {#each [2, 3, 4, 5] as count}
        <button
          class="count-btn"
          class:active={playerCount === count}
          onclick={() => updatePlayerCount(count)}
        >
          {count}
        </button>
      {/each}
    </div>
  </div>

  <div class="names-section">
    {#each { length: playerCount } as _, i}
      <div class="name-input-row">
        <label for="player-{i}">Player {i + 1}:</label>
        <input
          id="player-{i}"
          type="text"
          value={playerNames[i]}
          oninput={(e: Event) => handleNameInput(i, e)}
          placeholder="Player {i + 1}"
        />
      </div>
    {/each}
  </div>

  <button class="start-btn" onclick={startGame}>Start Game</button>
</div>

<style>
  .setup-screen {
    max-width: 400px;
    margin: 2rem auto;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  h2 {
    color: #4a3728;
    font-size: 1.5rem;
  }

  .player-count-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: center;
  }

  .player-count-section label {
    font-weight: 600;
    font-size: 0.95rem;
  }

  .count-buttons {
    display: flex;
    gap: 8px;
  }

  .count-btn {
    width: 48px;
    height: 48px;
    font-size: 1.2rem;
    font-weight: 700;
    border-radius: 50%;
    border: 2px solid #999;
    background: #fff;
  }

  .count-btn.active {
    border-color: #2a6bcf;
    background: #2a6bcf;
    color: #fff;
  }

  .names-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .name-input-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .name-input-row label {
    font-size: 0.85rem;
    min-width: 70px;
    text-align: right;
  }

  .name-input-row input {
    flex: 1;
    padding: 6px 10px;
    border: 2px solid #ccc;
    border-radius: 6px;
    font-size: 0.9rem;
  }

  .name-input-row input:focus {
    outline: none;
    border-color: #2a6bcf;
  }

  .start-btn {
    padding: 12px 24px;
    font-size: 1.1rem;
    font-weight: 700;
    background: #2a6bcf;
    color: #fff;
    border: none;
    border-radius: 8px;
  }

  .start-btn:hover {
    background: #1e56a8;
  }
</style>
