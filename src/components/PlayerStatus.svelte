<script lang="ts">
  import type { PlayerState } from '../data/types';

  let { player, active = false, isAI = false }: {
    player: PlayerState;
    active?: boolean;
    isAI?: boolean;
  } = $props();

  let score = $derived(player.completedGarments.reduce((sum, g) => sum + g.card.stars, 0));
  let garmentCount = $derived(player.completedGarments.length);
  let totalMaterials = $derived(Object.values(player.materials).reduce((sum, n) => sum + n, 0));
</script>

<div class="player-status" class:active>
  <div class="player-name">{player.name}{#if isAI} <span class="ai-badge">AI</span>{/if}</div>
  <div class="stats">
    <span class="stat" title="Score (stars)">{'*'} {score}</span>
    <span class="stat" title="Completed garments">Garments: {garmentCount}</span>
    <span class="stat" title="Stored materials">Materials: {totalMaterials}</span>
    <span class="stat" title="Deck size">Deck: {player.deck.length}</span>
    <span class="stat" title="Discard size">Discard: {player.discard.length}</span>
  </div>
</div>

<style>
  .player-status {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 12px;
    border: 2px solid #ccc;
    border-radius: 8px;
    background: #fff;
    min-width: 140px;
  }

  .player-status.active {
    border-color: #2a6bcf;
    background: #eef3ff;
    box-shadow: 0 0 8px rgba(42, 107, 207, 0.3);
  }

  .player-name {
    font-weight: 700;
    font-size: 0.9rem;
  }

  .active .player-name {
    color: #2a6bcf;
  }

  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    font-size: 0.7rem;
    color: #666;
  }

  .stat {
    white-space: nowrap;
  }

  .ai-badge {
    font-size: 0.65rem;
    font-weight: 700;
    background: #e67e22;
    color: #fff;
    padding: 1px 5px;
    border-radius: 4px;
    vertical-align: middle;
  }
</style>
