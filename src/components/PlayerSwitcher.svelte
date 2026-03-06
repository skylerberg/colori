<script lang="ts">
  import type { PlayerState } from '../data/types';
  import { getBuyerData } from '../data/cards';

  let { players, playerNames, selectedPlayerIndex, onSelectPlayer }: {
    players: PlayerState[];
    playerNames: string[];
    selectedPlayerIndex: number;
    onSelectPlayer: (index: number) => void;
  } = $props();

  let currentPlayer = $derived(players[selectedPlayerIndex]);
  let currentName = $derived(playerNames[selectedPlayerIndex]);

  let score = $derived(
    currentPlayer
      ? currentPlayer.completedBuyers.reduce((sum, buyer) => sum + getBuyerData(buyer.card).stars, 0) + currentPlayer.ducats
      : 0
  );

  let totalMaterials = $derived(
    currentPlayer
      ? Object.values(currentPlayer.materials).reduce((sum, n) => sum + n, 0)
      : 0
  );

  let buyerCount = $derived(currentPlayer ? currentPlayer.completedBuyers.length : 0);

  function prevPlayer() {
    const next = (selectedPlayerIndex - 1 + players.length) % players.length;
    onSelectPlayer(next);
  }

  function nextPlayer() {
    const next = (selectedPlayerIndex + 1) % players.length;
    onSelectPlayer(next);
  }
</script>

<button class="chevron chevron-left" onclick={prevPlayer} aria-label="Previous player">
  &#8249;
</button>

<button class="chevron chevron-right" onclick={nextPlayer} aria-label="Next player">
  &#8250;
</button>

<div class="player-info">
  <span class="player-name">{currentName}</span>
  <span class="player-stats">
    <span class="stat">&#9733; {score}</span>
    <span class="stat">Buyers: {buyerCount}</span>
    <span class="stat">Materials: {totalMaterials}</span>
    {#if currentPlayer && currentPlayer.ducats > 0}
      <span class="stat">Ducats: {currentPlayer.ducats}</span>
    {/if}
  </span>
</div>

<style>
  .chevron {
    position: fixed;
    top: 50%;
    transform: translateY(-50%);
    z-index: 100;
    background: rgba(20, 15, 10, 0.6);
    border: 1px solid rgba(201, 168, 76, 0.4);
    color: #c9a84c;
    font-size: 3rem;
    line-height: 1;
    width: 40px;
    height: 60px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background 0.2s, color 0.2s, border-color 0.2s;
    padding: 0;
  }

  .chevron:hover {
    background: rgba(201, 168, 76, 0.25);
    color: #e0c060;
    border-color: rgba(201, 168, 76, 0.7);
  }

  .chevron-left {
    left: 0;
    border-radius: 0 8px 8px 0;
  }

  .chevron-right {
    right: 0;
    border-radius: 8px 0 0 8px;
  }

  .player-info {
    position: fixed;
    bottom: 1rem;
    left: 50%;
    transform: translateX(-50%);
    z-index: 100;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    background: rgba(20, 15, 10, 0.8);
    border: 1px solid rgba(201, 168, 76, 0.4);
    border-radius: 8px;
    padding: 0.5rem 1.25rem;
  }

  .player-name {
    font-family: 'Cinzel', serif;
    font-size: 1.1rem;
    font-weight: 700;
    color: #c9a84c;
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  .player-stats {
    display: flex;
    gap: 0.75rem;
    font-size: 0.75rem;
    color: #d4c4a0;
  }

  .stat {
    font-family: 'Cinzel', serif;
    letter-spacing: 0.05em;
  }
</style>
