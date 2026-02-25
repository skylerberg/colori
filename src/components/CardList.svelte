<script lang="ts">
  import CardDisplay from './CardDisplay.svelte';

  let { cards, selectable = false, selectedIds = [], onCardClick }: {
    cards: { instanceId: number; card: string }[];
    selectable?: boolean;
    selectedIds?: number[];
    onCardClick?: (instanceId: number) => void;
  } = $props();
</script>

<div class="card-list">
  {#each cards as ci (ci.instanceId)}
    <CardDisplay
      card={ci.card}
      selected={selectedIds.includes(ci.instanceId)}
      onclick={selectable && onCardClick ? () => onCardClick!(ci.instanceId) : undefined}
    />
  {/each}
  {#if cards.length === 0}
    <div class="empty">No cards</div>
  {/if}
</div>

<style>
  .card-list {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    padding: 8px 4px;
    min-height: 176px;
    align-items: flex-start;
  }

  .empty {
    color: #999;
    font-style: italic;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    min-height: 160px;
  }
</style>
