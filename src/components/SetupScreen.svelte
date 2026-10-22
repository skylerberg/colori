<script lang="ts">
  import type { BotSettings, GameState } from '../data/types';
  import { BOT_DIFFICULTY_LEVELS, botPlayerName, humanPlayerName } from '../data/botDifficulty';
  import { BOT_PARAMS_CHOICES, END_OF_GAME_LOOKAHEAD, PLAYER_COUNTS } from '../data/gameSetup';
  import { createInitialGameState } from '../engine/wasmEngine';
  import { loadSetup, saveSetup } from '../persistence';
  import CharacterPortrait from './CharacterPortrait.svelte';

  let { onGameStarted }: {
    onGameStarted: (state: GameState, botIterations: number[], botSettings: BotSettings) => void;
  } = $props();

  const lastSetup = loadSetup();
  let playerCount = $state(lastSetup.playerCount);
  let isBot: boolean[] = $state(lastSetup.isBot);
  let botIterations: number[] = $state(lastSetup.botIterations);
  let playerNames: string[] = $state(
    lastSetup.names.map((name, i) => name ?? generatedName(i, lastSetup.isBot[i], lastSetup.botIterations[i]))
  );
  // A name the player typed is theirs to keep: only auto-generated names follow the Bot toggle
  // and the difficulty select.
  let autoNamed: boolean[] = $state(lastSetup.names.map(name => name === null));
  let botParams = $state(lastSetup.params);
  let rolloutLookahead = $state(lastSetup.rolloutLookahead);
  // Open when the remembered bot settings are not the defaults, so they don't carry over unseen.
  let advancedOpen = $state(lastSetup.params !== 'default' || lastSetup.rolloutLookahead !== null);

  function generatedName(seat: number, ai: boolean, iterations: number): string {
    return ai ? botPlayerName(seat, iterations) : humanPlayerName(seat);
  }

  function defaultName(index: number): string {
    return generatedName(index, isBot[index], botIterations[index]);
  }

  function updatePlayerCount(count: number) {
    playerCount = count;
  }

  function handleNameInput(index: number, event: Event) {
    const target = event.target as HTMLInputElement;
    playerNames[index] = target.value;
    autoNamed[index] = false;
  }

  function toggleBot(index: number) {
    isBot[index] = !isBot[index];
    if (autoNamed[index]) playerNames[index] = defaultName(index);
  }

  function handleDifficultyChange(index: number, event: Event) {
    const target = event.target as HTMLSelectElement;
    botIterations[index] = Number(target.value);
    if (autoNamed[index]) playerNames[index] = defaultName(index);
  }

  function startGame() {
    saveSetup({
      playerCount,
      isBot,
      botIterations,
      names: playerNames.map((name, i) => (autoNamed[i] ? null : name)),
      params: botParams,
      rolloutLookahead,
    });
    const names = playerNames.slice(0, playerCount).map((n, i) => n.trim() || defaultName(i));
    const botPlayers = isBot.slice(0, playerCount);
    const state = createInitialGameState(names, botPlayers);
    onGameStarted(state, botIterations.slice(0, playerCount), { params: botParams, rolloutLookahead });
  }
</script>

<div class="setup-screen">
  <h2>New Game</h2>

  <div class="player-count-section">
    <!-- svelte-ignore a11y_label_has_associated_control -->
    <label>Number of Players:</label>
    <div class="count-buttons">
      {#each PLAYER_COUNTS as count}
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
        <CharacterPortrait seat={i} size={36} />
        <label for="player-{i}">Player {i + 1}:</label>
        <input
          id="player-{i}"
          type="text"
          value={playerNames[i]}
          oninput={(e: Event) => handleNameInput(i, e)}
          placeholder={defaultName(i)}
        />
        <button
          class="bot-toggle"
          class:bot-active={isBot[i]}
          onclick={() => toggleBot(i)}
        >
          {isBot[i] ? 'Bot' : 'Human'}
        </button>
        {#if isBot[i]}
          <select
            class="difficulty-select"
            value={botIterations[i]}
            onchange={(e: Event) => handleDifficultyChange(i, e)}
          >
            {#each BOT_DIFFICULTY_LEVELS as level}
              <option value={level.iterations}>{level.label}</option>
            {/each}
          </select>
        {/if}
      </div>
    {/each}
  </div>

  <div class="advanced-section">
    <button class="advanced-header" aria-expanded={advancedOpen} onclick={() => advancedOpen = !advancedOpen}>
      <span>Advanced</span>
      <span class="chevron" class:open={advancedOpen}></span>
    </button>

    {#if advancedOpen}
      <div class="advanced-content">
        <label for="bot-params">Bot Parameters:</label>
        <select id="bot-params" bind:value={botParams}>
          {#each BOT_PARAMS_CHOICES as choice}
            <option value={choice.value}>{choice.label}</option>
          {/each}
        </select>

        <label for="rollout-lookahead">Rollout Lookahead:</label>
        <select id="rollout-lookahead" bind:value={rolloutLookahead}>
          <option value={null}>As trained</option>
          {#each { length: END_OF_GAME_LOOKAHEAD } as _, rounds}
            <option value={rounds}>{rounds} {rounds === 1 ? 'round' : 'rounds'}</option>
          {/each}
          <option value={END_OF_GAME_LOOKAHEAD}>End of game</option>
        </select>
      </div>
    {/if}
  </div>

  <button class="start-btn" onclick={startGame}>Start Game</button>
</div>

<style>
  .setup-screen {
    width: 100%;
    max-width: 400px;
    margin: 1.5rem auto;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    padding: 0 1rem;
  }

  h2 {
    font-family: var(--font-display, 'Cinzel', serif);
    color: var(--text-primary, #2c1e12);
    font-size: clamp(1.3rem, 3.5vw, 1.5rem);
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
    gap: 10px;
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
    gap: 12px;
  }

  .name-input-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .name-input-row label {
    font-size: 0.85rem;
    min-width: 70px;
    text-align: left;
  }

  .name-input-row input {
    flex: 1;
    min-width: 0;
    padding: 8px 10px;
    border: 2px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 6px;
    font-size: 0.9rem;
    min-height: 44px;
  }

  .name-input-row input:focus {
    outline: none;
    border-color: var(--accent-gold, #c9a84c);
  }

  .bot-toggle {
    padding: 8px 12px;
    font-size: 0.8rem;
    font-weight: 600;
    border: 2px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 6px;
    background: var(--bg-panel, #ebe3d3);
    min-width: 64px;
    min-height: 44px;
    cursor: pointer;
  }

  .bot-toggle.bot-active {
    border-color: var(--accent-crimson, #8b2020);
    background: var(--accent-crimson, #8b2020);
    color: var(--text-on-dark, #f5ede0);
  }

  .difficulty-select {
    padding: 8px 8px;
    font-size: 0.8rem;
    border: 2px solid var(--accent-crimson, #8b2020);
    border-radius: 6px;
    background: #fff;
    cursor: pointer;
    min-height: 44px;
    width: auto;
  }

  .advanced-section {
    border: 2px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 8px;
    background: var(--bg-panel, #ebe3d3);
    overflow: hidden;
  }

  .advanced-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 12px 16px;
    background: none;
    border: none;
    font-size: 0.95rem;
    font-weight: 600;
    cursor: pointer;
    min-height: 44px;
    color: inherit;
  }

  .advanced-header:hover {
    background: rgba(201, 168, 76, 0.1);
  }

  .chevron {
    display: inline-block;
    width: 0;
    height: 0;
    border-left: 5px solid transparent;
    border-right: 5px solid transparent;
    border-top: 6px solid currentColor;
    transition: transform 0.2s;
  }

  .chevron.open {
    transform: rotate(180deg);
  }

  .advanced-content {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: center;
    gap: 10px;
    padding: 0 16px 12px;
  }

  .advanced-content label {
    font-size: 0.95rem;
    font-weight: 600;
    text-align: left;
  }

  .advanced-content select {
    min-width: 0;
    padding: 8px 8px;
    font-size: 0.8rem;
    border: 2px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 6px;
    background: #fff;
    cursor: pointer;
    min-height: 44px;
  }

  .start-btn {
    padding: 14px 24px;
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 1.05rem;
    font-weight: 600;
    letter-spacing: 1.5px;
    background: var(--bg-deep, #2c1e12);
    color: var(--text-on-dark, #f5ede0);
    border: none;
    border-radius: 8px;
    min-height: 48px;
    transition: background 0.2s, transform 0.2s;
  }

  .start-btn:hover {
    background: #3a2a1e;
    transform: translateY(-2px);
  }

  @media (min-width: 768px) {
    .setup-screen {
      margin: 2rem auto;
      padding: 0;
    }

    .name-input-row label {
      text-align: right;
    }
  }
</style>
