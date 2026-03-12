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

  let score = $derived(player.completedBuyers.reduce((sum, buyer) => sum + getBuyerData(buyer.card).stars, 0) + player.ducats);
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
    gap: 1px;
    padding: 6px 8px;
    border: 2px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 6px;
    background: var(--bg-panel, #ebe3d3);
    min-width: 100px;
    min-height: 44px;
    cursor: pointer;
    text-align: left;
    font-family: inherit;
    font-size: inherit;
  }

  .player-status:hover {
    border-color: var(--border-gold-medium, rgba(201, 168, 76, 0.5));
    background: #e5daca;
  }

  .player-status:active {
    border-color: var(--border-gold-medium, rgba(201, 168, 76, 0.5));
    background: #e5daca;
  }

  .player-status.selected {
    border-color: var(--accent-gold, #c9a84c);
    background: rgba(201, 168, 76, 0.1);
    box-shadow: 0 0 8px rgba(201, 168, 76, 0.3);
  }

  .player-name {
    font-family: var(--font-display, 'Cinzel', serif);
    font-weight: 600;
    font-size: 0.75rem;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .active .player-name {
    color: var(--accent-gold, #c9a84c);
  }

  .turn-dot {
    display: inline-block;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent-gold, #c9a84c);
    flex-shrink: 0;
  }

  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 3px 5px;
    font-size: 0.6rem;
    color: var(--text-secondary, #6b5744);
  }

  .stat {
    white-space: nowrap;
  }

  .ai-badge {
    font-size: 0.6rem;
    font-weight: 700;
    background: var(--accent-crimson, #8b2020);
    color: var(--text-on-dark, #f5ede0);
    padding: 1px 4px;
    border-radius: 4px;
    vertical-align: middle;
  }

  .thinking-spinner {
    display: inline-block;
    width: 10px;
    height: 10px;
    border: 2px solid var(--bg-panel, #ebe3d3);
    border-top-color: var(--accent-gold, #c9a84c);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    flex-shrink: 0;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @media (min-width: 768px) {
    .player-status {
      padding: 5px 10px;
      min-width: 120px;
    }

    .player-name {
      font-size: 0.8rem;
    }

    .turn-dot {
      width: 8px;
      height: 8px;
    }

    .stats {
      gap: 4px 6px;
      font-size: 0.65rem;
    }

    .ai-badge {
      font-size: 0.65rem;
      padding: 1px 5px;
    }

    .thinking-spinner {
      width: 12px;
      height: 12px;
    }
  }
</style>
