<script lang="ts">
  import type { PlayerState } from '../data/types';
  import { getBuyerData } from '../data/cards';
  import ColorWheelDisplay from './ColorWheelDisplay.svelte';
  import CardList from './CardList.svelte';

  let { player, playerName }: {
    player: PlayerState;
    playerName: string;
  } = $props();

  let expanded = $state(false);

  let score = $derived(player.completedBuyers.reduce((sum, buyer) => sum + getBuyerData(buyer.card).stars, 0) + player.ducats);
  let buyerCount = $derived(player.completedBuyers.length);
  let totalMaterials = $derived(Object.values(player.materials).reduce((sum, n) => sum + n, 0));

  function toggle() {
    expanded = !expanded;
  }
</script>

<div class="opponent-panel">
  <button class="panel-header" onclick={toggle}>
    <span class="player-name">{playerName}</span>
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
          <ColorWheelDisplay wheel={player.colorWheel} size={150} />
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
        <CardList cards={[...player.workshopCards, ...player.workshoppedCards]} rotatedIds={player.workshoppedCards.map(c => c.instanceId)} />
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
    border: 1px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 8px;
    background: var(--bg-panel, #ebe3d3);
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 6px 10px;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.8rem;
    text-align: left;
  }

  .panel-header:hover {
    background: #e5daca;
  }

  .player-name {
    font-family: var(--font-display, 'Cinzel', serif);
    font-weight: 600;
    color: var(--text-primary, #2c1e12);
  }

  .header-stats {
    display: flex;
    gap: 10px;
    color: var(--text-secondary, #6b5744);
    font-size: 0.75rem;
  }

  .chevron {
    margin-left: auto;
    width: 0;
    height: 0;
    border-left: 5px solid transparent;
    border-right: 5px solid transparent;
    border-top: 5px solid var(--text-tertiary, #9a8775);
    transition: transform 0.2s;
  }

  .chevron.open {
    transform: rotate(180deg);
  }

  .panel-content {
    padding: 6px 10px 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    border-top: 1px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    --card-width: 80px;
    --card-height: 112px;
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
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 0.7rem;
    color: var(--text-primary, #2c1e12);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin: 0 0 4px;
  }

  .material-counts {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.75rem;
    color: var(--accent-gold, #c9a84c);
  }

  .material-count {
    font-weight: 600;
  }

  .ducats-count {
    font-size: 0.75rem;
    color: var(--accent-gold, #c9a84c);
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
