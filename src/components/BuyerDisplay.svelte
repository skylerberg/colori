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

<style>
  .buyer-display {
    background: rgba(20, 15, 10, 0.75);
    border: 1px solid rgba(201, 168, 76, 0.4);
    border-radius: 8px;
    padding: 0.75rem;
  }

  .section-title {
    font-family: 'Cinzel', serif;
    font-size: 0.85rem;
    font-weight: 600;
    color: #c9a84c;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    margin-bottom: 0.5rem;
  }

  .buyer-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 2px;
    overflow-y: auto;
  }

  .empty {
    color: #9a8775;
    font-style: italic;
    padding: 20px;
    text-align: center;
  }
</style>
