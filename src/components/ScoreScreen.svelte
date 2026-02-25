<script lang="ts">
  import type { GameState } from '../data/types';
  import { calculateScores, determineWinner } from '../engine/scoring';
  import { downloadGameLog, type StructuredGameLog } from '../gameLog';

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

  let scores = $derived(calculateScores(gameState.players));
  let winner = $derived(determineWinner(gameState.players));
  let sortedScores = $derived([...scores].sort((a, b) => b.score - a.score));
</script>

<div class="score-screen">
  <h2>Game Over!</h2>

  <div class="winner-banner">
    {winner} wins!
  </div>

  {#if finalTime}
    <div class="game-time">Game time: {finalTime}</div>
  {/if}

  <div class="scores-list">
    {#each sortedScores as entry, i}
      <div class="score-row" class:winner={entry.name === winner}>
        <span class="rank">#{i + 1}</span>
        <span class="name">{entry.name}</span>
        <span class="score">{'*'.repeat(entry.score)} ({entry.score})</span>
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
    max-width: 500px;
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

  .score-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    border: 2px solid #ddd;
    border-radius: 8px;
    background: #fff;
  }

  .score-row.winner {
    border-color: #d4a017;
    background: #fffde7;
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
