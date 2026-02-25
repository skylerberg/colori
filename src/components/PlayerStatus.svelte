<script lang="ts">
  import type { PlayerState } from '../data/types';
  import { getBuyerData } from '../data/cards';

  let { player, playerName, active = false, selected = false, isAI = false, thinking = false, onclick }: {
    player: PlayerState;
    playerName: string;
    active?: boolean;
    selected?: boolean;
    isAI?: boolean;
    thinking?: boolean;
    onclick?: () => void;
  } = $props();

  let score = $derived(player.completedBuyers.reduce((sum, g) => sum + getBuyerData(g.card).stars, 0) + player.ducats);
  let buyerCount = $derived(player.completedBuyers.length);
  let totalMaterials = $derived(Object.values(player.materials).reduce((sum, n) => sum + n, 0));
  let ducats = $derived(player.ducats);
</script>

<button class="player-status" class:active class:selected {onclick} type="button">
  <div class="player-name">
    {#if active}<span class="turn-dot"></span>{/if}
    {playerName}{#if isAI} <span class="ai-badge">AI</span>{/if}{#if thinking}<span class="thinking-spinner"></span>{/if}
  </div>
  <div class="stats">
    <span class="stat" title="Score (stars)">{'*'} {score}</span>
    <span class="stat" title="Completed buyers">Buyers: {buyerCount}</span>
    <span class="stat" title="Stored materials">Materials: {totalMaterials}</span>
    {#if ducats > 0}<span class="stat" title="Ducats">Ducats: {ducats}</span>{/if}
    <span class="stat" title="Deck size">Deck: {player.deck.length}</span>
    <span class="stat" title="Discard size">Discard: {player.discard.length}</span>
  </div>
</button>

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
    cursor: pointer;
    text-align: left;
    font-family: inherit;
    font-size: inherit;
  }

  .player-status:hover {
    border-color: #aaa;
    background: #fafafa;
  }

  .player-status.selected {
    border-color: #2a6bcf;
    background: #eef3ff;
    box-shadow: 0 0 8px rgba(42, 107, 207, 0.3);
  }

  .player-name {
    font-weight: 700;
    font-size: 0.9rem;
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .active .player-name {
    color: #2a6bcf;
  }

  .turn-dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #2a6bcf;
    flex-shrink: 0;
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

  .thinking-spinner {
    display: inline-block;
    width: 12px;
    height: 12px;
    border: 2px solid #e0d5c5;
    border-top-color: #e67e22;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    flex-shrink: 0;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
