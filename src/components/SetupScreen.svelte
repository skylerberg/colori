<script lang="ts">
  import type { GameState } from '../data/types';
  import { createInitialGameState } from '../engine/wasmEngine';

  const DIFFICULTY_LEVELS: { label: string; iterations: number }[] = [
    { label: 'Easy', iterations: 10000 },
    { label: 'Medium', iterations: 50000 },
    { label: 'Hard', iterations: 100000 },
    { label: 'Very Hard', iterations: 200000 },
    { label: 'Insane', iterations: 400000 },
    { label: 'Godly', iterations: 1000000 },
  ];

  let { onGameStarted }: {
    onGameStarted: (state: GameState, aiIterations: number[]) => void;
  } = $props();

  let playerCount = $state(2);
  let playerNames: string[] = $state(['Player 1', 'Player 2', 'Player 3', 'Player 4', 'Player 5']);
  let isAI: boolean[] = $state([false, false, false, false, false]);
  let aiIterations: number[] = $state([100000, 100000, 100000, 100000, 100000]);

  function updatePlayerCount(count: number) {
    playerCount = count;
  }

  function handleNameInput(index: number, event: Event) {
    const target = event.target as HTMLInputElement;
    playerNames[index] = target.value;
  }

  function toggleAI(index: number) {
    isAI[index] = !isAI[index];
    if (isAI[index]) {
      aiIterations[index] = 100000;
      if (playerNames[index] === `Player ${index + 1}`) {
        playerNames[index] = `AI Player ${index + 1}`;
      }
    } else if (playerNames[index] === `AI Player ${index + 1}`) {
      playerNames[index] = `Player ${index + 1}`;
    }
  }

  function handleDifficultyChange(index: number, event: Event) {
    const target = event.target as HTMLSelectElement;
    aiIterations[index] = Number(target.value);
  }

  function startGame() {
    const names = playerNames.slice(0, playerCount).map((n, i) => n.trim() || `Player ${i + 1}`);
    const aiPlayers = isAI.slice(0, playerCount);
    const state = createInitialGameState(names, aiPlayers);
    onGameStarted(state, aiIterations.slice(0, playerCount));
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
        <button
          class="ai-toggle"
          class:ai-active={isAI[i]}
          onclick={() => toggleAI(i)}
        >
          {isAI[i] ? 'AI' : 'Human'}
        </button>
        {#if isAI[i]}
          <select
            class="difficulty-select"
            value={aiIterations[i]}
            onchange={(e: Event) => handleDifficultyChange(i, e)}
          >
            {#each DIFFICULTY_LEVELS as level}
              <option value={level.iterations}>{level.label}</option>
            {/each}
          </select>
        {/if}
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
    font-family: var(--font-display, 'Cinzel', serif);
    color: var(--text-primary, #2c1e12);
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
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 1.2rem;
    font-weight: 700;
    border-radius: 50%;
    border: 2px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    background: var(--bg-panel, #ebe3d3);
  }

  .count-btn.active {
    border-color: var(--accent-gold, #c9a84c);
    background: var(--accent-gold, #c9a84c);
    color: var(--bg-deep, #2c1e12);
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
    border: 2px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 6px;
    font-size: 0.9rem;
  }

  .name-input-row input:focus {
    outline: none;
    border-color: var(--accent-gold, #c9a84c);
  }

  .ai-toggle {
    padding: 6px 12px;
    font-size: 0.8rem;
    font-weight: 600;
    border: 2px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 6px;
    background: var(--bg-panel, #ebe3d3);
    min-width: 64px;
    cursor: pointer;
  }

  .ai-toggle.ai-active {
    border-color: var(--accent-crimson, #8b2020);
    background: var(--accent-crimson, #8b2020);
    color: var(--text-on-dark, #f5ede0);
  }

  .difficulty-select {
    padding: 6px 8px;
    font-size: 0.8rem;
    border: 2px solid var(--accent-crimson, #8b2020);
    border-radius: 6px;
    background: #fff;
    cursor: pointer;
  }

  .start-btn {
    padding: 12px 24px;
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 1.05rem;
    font-weight: 600;
    letter-spacing: 1.5px;
    background: var(--bg-deep, #2c1e12);
    color: var(--text-on-dark, #f5ede0);
    border: none;
    border-radius: 8px;
    transition: background 0.2s, transform 0.2s;
  }

  .start-btn:hover {
    background: #3a2a1e;
    transform: translateY(-2px);
  }
</style>
