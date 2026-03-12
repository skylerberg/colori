<script lang="ts">
  import type { BuyerInstance } from '../data/types';
  import CardDisplay from './CardDisplay.svelte';

  let { buyers, selectable = false, selectedId, onSelect }: {
    buyers: BuyerInstance[];
    selectable?: boolean;
    selectedId?: number;
    onSelect?: (instanceId: number) => void;
  } = $props();
</script>

<div class="buyer-display">
  <h3 class="section-title">Buyer Display</h3>
  <div class="buyer-scroll">
    <div class="buyer-row">
      {#each buyers as buyer (buyer.instanceId)}
        <CardDisplay
          card={buyer.card}
          selected={selectedId === buyer.instanceId}
          onclick={selectable && onSelect ? () => onSelect!(buyer.instanceId) : undefined}
        />
      {/each}
      {#if buyers.length === 0}
        <div class="empty">No buyers available</div>
      {/if}
    </div>
  </div>
</div>

<style>
  .buyer-display {
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

  .buyer-scroll {
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: thin;
  }

  .buyer-row {
    display: flex;
    gap: 6px;
    padding: 0.5rem 2px;
    flex-wrap: nowrap;
    width: max-content;
  }

  .buyer-row :global(.card.clickable:hover) {
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
    .buyer-display {
      padding: 0.75rem;
    }

    .section-title {
      font-size: 0.85rem;
      margin-bottom: 0.5rem;
    }

    .buyer-row {
      gap: 8px;
      padding: 10px 2px;
    }

    .empty {
      padding: 20px;
    }
  }
</style>
