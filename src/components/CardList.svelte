<script lang="ts">
  import CardDisplay from './CardDisplay.svelte';

  let { cards, selectable = false, selectedIds = [], rotatedIds = [], onCardClick }: {
    cards: { instanceId: number; card: string }[];
    selectable?: boolean;
    selectedIds?: number[];
    rotatedIds?: number[];
    onCardClick?: (instanceId: number) => void;
  } = $props();
</script>

<div class="card-list">
  {#each cards as ci (ci.instanceId)}
    {@const isRotated = rotatedIds.includes(ci.instanceId)}
    <CardDisplay
      card={ci.card}
      selected={selectedIds.includes(ci.instanceId)}
      rotated={isRotated}
      onclick={!isRotated && selectable && onCardClick ? () => onCardClick!(ci.instanceId) : undefined}
    />
  {/each}
  {#if cards.length === 0}
    <div class="empty">No cards</div>
  {/if}
</div>

<style>
  .card-list {
    display: flex;
    gap: 6px;
    overflow-x: auto;
    padding: 10px 2px;
    min-height: 176px;
    align-items: flex-start;
  }

  .empty {
    color: var(--text-tertiary, #9a8775);
    font-style: italic;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    min-height: 168px;
  }
</style>
