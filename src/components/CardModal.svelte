<script lang="ts">
  import type { CardInstance } from '../data/types';
  import { getAnyCardData } from '../data/cards';
  import CardDisplay from './CardDisplay.svelte';

  let { title, cards, onClose }: {
    title: string;
    cards: CardInstance[];
    onClose: () => void;
  } = $props();

  let hasCardData = $derived(cards.length > 0 && cards[0]?.card != null);

  let sortedCards = $derived(
    hasCardData
      ? [...cards].sort((a, b) => {
          const nameA = getAnyCardData(a.card)?.name ?? a.card;
          const nameB = getAnyCardData(b.card)?.name ?? b.card;
          return nameA.localeCompare(nameB);
        })
      : cards
  );
</script>

<div class="modal-overlay" onclick={onClose}>
  <div class="modal-box" onclick={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <span class="modal-title">{title}</span>
      <button class="close-btn" onclick={onClose}>&times;</button>
    </div>
    <div class="modal-body">
      {#if cards.length === 0}
        <div class="empty-text">No cards</div>
      {:else if !hasCardData}
        <div class="hidden-text">{cards.length} cards (hidden)</div>
      {:else}
        <div class="card-grid">
          {#each sortedCards as cardInstance (cardInstance.instanceId)}
            <CardDisplay card={cardInstance.card} />
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal-box {
    background: rgba(20, 15, 10, 0.95);
    border: 1px solid rgba(201, 168, 76, 0.5);
    border-radius: 10px;
    width: 900px;
    max-width: 90vw;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    --card-width: 175px;
    --card-height: 245px;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid rgba(201, 168, 76, 0.3);
  }

  .modal-title {
    font-family: 'Cinzel', serif;
    font-size: 1rem;
    font-weight: 600;
    color: #c9a84c;
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  .close-btn {
    background: none;
    border: none;
    color: rgba(245, 237, 224, 0.6);
    font-size: 1.4rem;
    cursor: pointer;
    padding: 0 4px;
    line-height: 1;
  }

  .close-btn:hover {
    color: #f5ede0;
  }

  .modal-body {
    overflow-y: auto;
    padding: 1rem;
  }

  .card-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: center;
  }

  .empty-text,
  .hidden-text {
    color: rgba(245, 237, 224, 0.4);
    font-style: italic;
    font-size: 0.85rem;
    text-align: center;
    padding: 2rem 0;
  }
</style>
