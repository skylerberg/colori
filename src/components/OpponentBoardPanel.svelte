<script lang="ts">
  import type { PlayerState } from '../data/types';
  import ColorWheelDisplay from './ColorWheelDisplay.svelte';
  import CardList from './CardList.svelte';

  let { player }: {
    player: PlayerState;
  } = $props();

  let expanded = $state(false);

  let score = $derived(player.completedBuyers.reduce((sum, g) => sum + g.card.stars, 0) + player.ducats);
  let buyerCount = $derived(player.completedBuyers.length);
  let totalMaterials = $derived(Object.values(player.materials).reduce((sum, n) => sum + n, 0));

  function toggle() {
    expanded = !expanded;
  }
</script>

<div class="opponent-panel">
  <button class="panel-header" onclick={toggle}>
    <span class="player-name">{player.name}</span>
    <span class="header-stats">
      <span>* {score}</span>
      <span>Buyers: {buyerCount}</span>
      <span>Materials: {totalMaterials}</span>
      {#if player.ducats > 0}<span>Ducats: {player.ducats}</span>{/if}
    </span>
    <span class="chevron" class:open={expanded}></span>
  </button>

  {#if expanded}
    <div class="panel-content">
      <div class="compact-row">
        <div class="color-wheel-section">
          <h4>Color Wheel</h4>
          <ColorWheelDisplay wheel={player.colorWheel} size={120} />
        </div>

        <div class="materials-section">
          <h4>Stored Materials</h4>
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
        <h4>Drafted Cards</h4>
        <CardList cards={player.draftedCards} />
      </div>

      <div class="card-section">
        <h4>Workshop</h4>
        <CardList cards={player.workshopCards} />
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

<style>
  .opponent-panel {
    border: 1px solid #ccc;
    border-radius: 8px;
    background: #fafafa;
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 8px 12px;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.85rem;
    text-align: left;
  }

  .panel-header:hover {
    background: #f0f0f0;
  }

  .player-name {
    font-weight: 700;
    color: #4a3728;
  }

  .header-stats {
    display: flex;
    gap: 10px;
    color: #666;
    font-size: 0.75rem;
  }

  .chevron {
    margin-left: auto;
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

  .panel-content {
    padding: 8px 12px 12px;
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

  .color-wheel-section,
  .materials-section {
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
</style>
