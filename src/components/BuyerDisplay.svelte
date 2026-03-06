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
    padding: 4px 0;
  }

  .section-title {
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-primary, #2c1e12);
    text-transform: uppercase;
    letter-spacing: 1.5px;
    margin-bottom: 6px;
    padding-bottom: 4px;
    border-bottom: 1px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    text-align: left;
  }

  .buyer-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 6px;
    padding: 2px;
  }

  .buyer-grid :global(.card) {
    width: auto;
    aspect-ratio: 5 / 7;
    height: auto;
  }

  .empty {
    color: var(--text-tertiary, #9a8775);
    font-style: italic;
    padding: 20px;
  }
</style>
