<script lang="ts">
  import CardDisplay from './CardDisplay.svelte';

  let { cards, selectable = false, selectedIds = [], rotatedIds = [], destroyingIds = [], onCardClick }: {
    cards: { instanceId: number; card: string }[];
    selectable?: boolean;
    selectedIds?: number[];
    rotatedIds?: number[];
    destroyingIds?: number[];
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
      destroying={destroyingIds.includes(ci.instanceId)}
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
    gap: 4px;
    overflow-x: auto;
    padding: 0.5rem 2px;
    min-height: calc(var(--card-height, 126px) + 1.25rem);
    align-items: flex-end;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: thin;
  }

  .empty {
    color: var(--text-tertiary, #9a8775);
    font-style: italic;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    min-height: calc(var(--card-height, 126px) + 0.5rem);
  }

  @media (min-width: 768px) {
    .card-list {
      gap: 6px;
      padding: 10px 2px;
    }
  }

  @media (min-width: 1024px) {
    .card-list {
      gap: 8px;
    }
  }
</style>
