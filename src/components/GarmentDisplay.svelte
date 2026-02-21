<script lang="ts">
  import type { CardInstance, GarmentCard } from '../data/types';
  import CardDisplay from './CardDisplay.svelte';

  let { garments, selectable = false, onSelect }: {
    garments: CardInstance<GarmentCard>[];
    selectable?: boolean;
    onSelect?: (instanceId: number) => void;
  } = $props();
</script>

<div class="garment-display">
  <h3 class="section-title">Garment Display</h3>
  <div class="garment-grid">
    {#each garments as gi (gi.instanceId)}
      <CardDisplay
        card={gi.card}
        onclick={selectable && onSelect ? () => onSelect!(gi.instanceId) : undefined}
      />
    {/each}
    {#if garments.length === 0}
      <div class="empty">No garments available</div>
    {/if}
  </div>
</div>

<style>
  .garment-display {
    padding: 8px 0;
  }

  .section-title {
    font-size: 0.85rem;
    color: #4a3728;
    margin-bottom: 6px;
    text-align: left;
  }

  .garment-grid {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    padding: 4px;
  }

  .empty {
    color: #999;
    font-style: italic;
    padding: 20px;
  }
</style>
