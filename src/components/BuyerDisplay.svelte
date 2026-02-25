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
  <div class="buyer-grid">
    {#each buyers as gi (gi.instanceId)}
      <CardDisplay
        card={gi.card}
        selected={selectedId === gi.instanceId}
        onclick={selectable && onSelect ? () => onSelect!(gi.instanceId) : undefined}
      />
    {/each}
    {#if buyers.length === 0}
      <div class="empty">No buyers available</div>
    {/if}
  </div>
</div>

<style>
  .buyer-display {
    padding: 8px 0;
  }

  .section-title {
    font-size: 0.85rem;
    color: #4a3728;
    margin-bottom: 6px;
    text-align: left;
  }

  .buyer-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    padding: 4px;
  }

  .buyer-grid :global(.card) {
    width: auto;
  }

  .empty {
    color: #999;
    font-style: italic;
    padding: 20px;
  }
</style>
