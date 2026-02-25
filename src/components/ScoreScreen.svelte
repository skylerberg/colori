<script lang="ts">
  import type { GameState } from '../data/types';
  import { calculateScores, determineWinner } from '../engine/wasmEngine';
  import { downloadGameLog, type StructuredGameLog } from '../gameLog';
  import ColorWheelDisplay from './ColorWheelDisplay.svelte';
  import CardList from './CardList.svelte';

  let { gameState, gameStartTime, onPlayAgain, structuredLog }: {
    gameState: GameState;
    gameStartTime: number | null;
    onPlayAgain: () => void;
    structuredLog?: StructuredGameLog | null;
  } = $props();

  function formatTime(seconds: number): string {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    if (h > 0) {
      return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
    }
    return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  }

  let finalTime = $derived(
    gameStartTime !== null ? formatTime(Math.floor((Date.now() - gameStartTime) / 1000)) : null
  );

  let rounds = $derived(gameState.round - 1);
  let scores = $derived(calculateScores(gameState.players, gameState.playerNames));
  let winner = $derived(determineWinner(gameState.players, gameState.playerNames));
  let sortedScores = $derived([...scores].sort((a, b) => b.score - a.score));

  let expandedPlayers = $state(new Set<string>());

  function togglePlayer(name: string) {
    if (expandedPlayers.has(name)) {
      expandedPlayers.delete(name);
    } else {
      expandedPlayers.add(name);
    }
    expandedPlayers = new Set(expandedPlayers);
  }

  function getPlayer(name: string) {
    const idx = gameState.playerNames.indexOf(name);
    return gameState.players[idx];
  }
</script>

<div class="score-screen">
  <h2>Game Over!</h2>

  <div class="winner-banner">
    {winner} wins!
  </div>

  <div class="game-time">
    {#if finalTime}Game time: {finalTime} | {/if}Rounds: {rounds}
  </div>

  <div class="scores-list">
    {#each sortedScores as entry, i}
      {@const player = getPlayer(entry.name)}
      {@const expanded = expandedPlayers.has(entry.name)}
      <div class="score-entry" class:winner={entry.name === winner}>
        <button class="score-row" onclick={() => togglePlayer(entry.name)}>
          <span class="rank">#{i + 1}</span>
          <span class="name">{entry.name}</span>
          <span class="score">{'*'.repeat(entry.score)} ({entry.score})</span>
          <span class="chevron" class:open={expanded}></span>
        </button>

        {#if expanded}
          <div class="player-details">
            <div class="compact-row">
              <div class="detail-section">
                <h4>Color Wheel</h4>
                <ColorWheelDisplay wheel={player.colorWheel} size={120} />
              </div>

              <div class="detail-section">
                <h4>Materials</h4>
                <div class="material-counts">
                  {#each Object.entries(player.materials) as [material, count]}
                    <span class="material-count">{material}: {count}</span>
                  {/each}
                </div>
                {#if player.ducats > 0}
                  <div class="ducats-count">Ducats: {player.ducats}</div>
                {/if}
              </div>
            </div>

            <div class="card-section">
              <h4>Full Deck</h4>
              <CardList cards={[...player.deck, ...player.discard]} />
            </div>

            {#if player.completedBuyers.length > 0}
              <div class="card-section">
                <h4>Completed Buyers</h4>
                <CardList cards={player.completedBuyers} />
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <div class="button-row">
    {#if structuredLog}
      <button class="download-btn" onclick={() => downloadGameLog(structuredLog!)}>Download Game Log</button>
    {/if}
    <button class="play-again-btn" onclick={onPlayAgain}>Play Again</button>
  </div>
</div>

<style>
  .score-screen {
    max-width: 700px;
    margin: 2rem auto;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    align-items: center;
  }

  h2 {
    color: #4a3728;
    font-size: 1.8rem;
  }

  .winner-banner {
    font-size: 1.4rem;
    font-weight: 700;
    color: #d4a017;
    padding: 12px 24px;
    border: 3px solid #d4a017;
    border-radius: 12px;
    background: #fffde7;
  }

  .game-time {
    font-size: 1rem;
    color: #666;
  }

  .scores-list {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .score-entry {
    border: 2px solid #ddd;
    border-radius: 8px;
    background: #fff;
    overflow: hidden;
  }

  .score-entry.winner {
    border-color: #d4a017;
    background: #fffde7;
  }

  .score-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    width: 100%;
    background: none;
    border: none;
    cursor: pointer;
    font: inherit;
    text-align: left;
  }

  .score-row:hover {
    background: rgba(0, 0, 0, 0.03);
  }

  .rank {
    font-weight: 700;
    font-size: 1rem;
    color: #888;
    min-width: 30px;
  }

  .winner .rank {
    color: #d4a017;
  }

  .name {
    font-weight: 600;
    flex: 1;
    text-align: left;
  }

  .score {
    color: #d4a017;
    font-weight: 600;
  }

  .chevron {
    width: 0;
    height: 0;
    border-left: 5px solid transparent;
    border-right: 5px solid transparent;
    border-top: 5px solid #999;
    transition: transform 0.2s;
  }

  .chevron.open {
    transform: rotate(180deg);
  }

  .player-details {
    padding: 8px 16px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-top: 1px solid #e0e0e0;
  }

  .compact-row {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .detail-section {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  h4 {
    font-size: 0.75rem;
    color: #4a3728;
    margin: 0 0 4px;
  }

  .material-counts {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.75rem;
    color: #8b6914;
  }

  .material-count {
    font-weight: 600;
  }

  .ducats-count {
    font-size: 0.75rem;
    color: #d4a017;
    font-weight: 600;
    margin-top: 4px;
  }

  .card-section {
    text-align: left;
  }

  .card-section :global(.card-list) {
    min-height: auto;
  }

  .button-row {
    display: flex;
    gap: 12px;
    align-items: center;
    margin-top: 1rem;
  }

  .play-again-btn {
    padding: 12px 32px;
    font-size: 1.1rem;
    font-weight: 700;
    background: #2a6bcf;
    color: #fff;
    border: none;
    border-radius: 8px;
  }

  .play-again-btn:hover {
    background: #1e56a8;
  }

  .download-btn {
    padding: 12px 24px;
    font-size: 1rem;
    font-weight: 600;
    background: #666;
    color: #fff;
    border: none;
    border-radius: 8px;
    cursor: pointer;
  }

  .download-btn:hover {
    background: #555;
  }
</style>
