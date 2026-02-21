<script lang="ts">
  import type { GameState } from '../data/types';
  import CardList from './CardList.svelte';

  let { gameState, onContinue }: {
    gameState: GameState;
    onContinue: () => void;
  } = $props();
</script>

<div class="draw-phase">
  <h2>Draw Phase - Round {gameState.round}</h2>
  <p class="subtitle">Each player drew 5 cards from their deck.</p>

  <div class="players-drawn">
    {#each gameState.players as player, i}
      <div class="player-draw">
        <h3>{player.name}'s Drawn Cards</h3>
        <CardList cards={player.drawnCards} />
      </div>
    {/each}
  </div>

  <button class="continue-btn" onclick={onContinue}>Continue to Draft</button>
</div>

<style>
  .draw-phase {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    align-items: center;
  }

  h2 {
    color: #4a3728;
    font-size: 1.3rem;
  }

  .subtitle {
    color: #666;
    font-size: 0.9rem;
  }

  .players-drawn {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .player-draw {
    border: 1px solid #ddd;
    border-radius: 8px;
    padding: 8px 12px;
    background: #fff;
    text-align: left;
  }

  .player-draw h3 {
    font-size: 0.9rem;
    color: #4a3728;
    margin-bottom: 4px;
  }

  .continue-btn {
    padding: 10px 24px;
    font-size: 1rem;
    font-weight: 700;
    background: #2a6bcf;
    color: #fff;
    border: none;
    border-radius: 8px;
  }

  .continue-btn:hover {
    background: #1e56a8;
  }
</style>
