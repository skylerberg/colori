<script lang="ts">
  import type { GameState } from '../data/types';
  import { calculateScores, determineWinner } from '../engine/scoring';

  let { gameState, onPlayAgain }: {
    gameState: GameState;
    onPlayAgain: () => void;
  } = $props();

  let scores = $derived(calculateScores(gameState.players));
  let winner = $derived(determineWinner(gameState.players));
  let sortedScores = $derived([...scores].sort((a, b) => b.score - a.score));
</script>

<div class="score-screen">
  <h2>Game Over!</h2>

  <div class="winner-banner">
    {winner} wins!
  </div>

  <div class="scores-list">
    {#each sortedScores as entry, i}
      <div class="score-row" class:winner={entry.name === winner}>
        <span class="rank">#{i + 1}</span>
        <span class="name">{entry.name}</span>
        <span class="score">{'*'.repeat(entry.score)} ({entry.score})</span>
      </div>
    {/each}
  </div>

  <button class="play-again-btn" onclick={onPlayAgain}>Play Again</button>
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

  .play-again-btn {
    padding: 12px 32px;
    font-size: 1.1rem;
    font-weight: 700;
    background: #2a6bcf;
    color: #fff;
    border: none;
    border-radius: 8px;
    margin-top: 1rem;
  }

  .play-again-btn:hover {
    background: #1e56a8;
  }
</style>
