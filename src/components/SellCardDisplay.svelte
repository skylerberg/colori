<script lang="ts">
  import type { SellCardInstance } from '../data/types';
  import CardDisplay from './CardDisplay.svelte';

  let { sellCards, selectable = false, selectedId, onSelect }: {
    sellCards: SellCardInstance[];
    selectable?: boolean;
    selectedId?: number;
    onSelect?: (instanceId: number) => void;
  } = $props();

  // Maintain a stable display order so cards don't jump when the engine
  // uses swap_remove internally. New cards replace removed cards in-place.
  let stableOrder: number[] = $state([]);

  $effect(() => {
    const currentIds = new Set(sellCards.map(c => c.instanceId));
    const prevIds = new Set(stableOrder);

    const added = sellCards.filter(c => !prevIds.has(c.instanceId)).map(c => c.instanceId);
    const removedSet = new Set(stableOrder.filter(id => !currentIds.has(id)));

    if (removedSet.size === 0 && added.length === 0) return;

    let addIdx = 0;
    const newOrder: number[] = [];
    for (const id of stableOrder) {
      if (removedSet.has(id)) {
        if (addIdx < added.length) {
          newOrder.push(added[addIdx++]);
        }
      } else {
        newOrder.push(id);
      }
    }
    while (addIdx < added.length) {
      newOrder.push(added[addIdx++]);
    }
    stableOrder = newOrder;
  });

  let stableSellCards = $derived.by(() => {
    const map = new Map(sellCards.map(c => [c.instanceId, c]));
    return stableOrder.map(id => map.get(id)!).filter(Boolean);
  });
</script>

<div class="sell-card-display">
  <h3 class="section-title">Sell Card Display</h3>
  <div class="sell-card-scroll">
    <div class="sell-card-row">
      {#each stableSellCards as sellCard (sellCard.instanceId)}
        <CardDisplay
          card={sellCard.card}
          selected={selectedId === sellCard.instanceId}
          onclick={selectable && onSelect ? () => onSelect!(sellCard.instanceId) : undefined}
        />
      {/each}
      {#if sellCards.length === 0}
        <div class="empty">No sell cards available</div>
      {/if}
    </div>
  </div>
</div>

<style>
  .sell-card-display {
    background: rgba(20, 15, 10, 0.75);
    border: 1px solid rgba(201, 168, 76, 0.4);
    border-radius: 8px;
    padding: 0.5rem;
  }

  .section-title {
    font-family: 'Cinzel', serif;
    font-size: 0.8rem;
    font-weight: 600;
    color: #c9a84c;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    margin-bottom: 0.375rem;
  }

  .sell-card-scroll {
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: thin;
  }

  .sell-card-row {
    display: flex;
    gap: 6px;
    padding: 0.5rem 2px;
    flex-wrap: nowrap;
    width: max-content;
  }

  .sell-card-row :global(.card.clickable:hover) {
    transform: none;
    box-shadow: 0 0 12px rgba(201, 168, 76, 0.6), 0 0 24px rgba(201, 168, 76, 0.3);
    border: 1px solid var(--accent-gold, #c9a84c);
  }

  .empty {
    color: #9a8775;
    font-style: italic;
    padding: 1rem;
    text-align: center;
    font-size: 0.85rem;
  }

  @media (min-width: 768px) {
    .sell-card-display {
      padding: 0.75rem;
    }

    .section-title {
      font-size: 0.85rem;
      margin-bottom: 0.5rem;
    }

    .sell-card-row {
      gap: 8px;
      padding: 10px 2px;
    }

    .empty {
      padding: 20px;
    }
  }
</style>
