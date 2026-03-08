<script lang="ts">
  import type { Card, BuyerCard } from '../data/types';
  import { getAnyCardData } from '../data/cards';
  import { getCardArtUrl, getBuyerArtUrl } from '../data/cardArt';

  let { card, selected = false, onclick }: {
    card: string;
    selected?: boolean;
    onclick?: () => void;
  } = $props();

  let data = $derived(getAnyCardData(card));
  let cardLabel = $derived(data && 'name' in data ? data.name : card);

  let artUrl = $derived(
    data
      ? data.kind === 'buyer'
        ? getBuyerArtUrl(card as BuyerCard)
        : getCardArtUrl(card as Card)
      : ''
  );
</script>

{#if data}
<button
  class="card"
  class:selected
  class:clickable={!!onclick}
  style="background-image: url('{artUrl}')"
  title={cardLabel}
  onclick={onclick}
  disabled={!onclick}
>
</button>
{/if}

<style>
  .card {
    width: var(--card-width, 220px);
    height: var(--card-height, 308px);
    border: none;
    border-radius: 6px;
    padding: 0;
    background-color: #1a1a1a;
    background-size: cover;
    background-position: center;
    flex-shrink: 0;
    cursor: default;
    overflow: hidden;
    position: relative;
    box-shadow: var(--shadow-card, 0 2px 8px rgba(44, 30, 18, 0.2));
    transition: transform 0.2s, box-shadow 0.2s, border-color 0.2s;
  }

  .card.clickable {
    cursor: pointer;
  }

  .card.clickable:hover {
    transform: translateY(-4px) scale(1.02);
    box-shadow: var(--shadow-card-hover, 0 8px 20px rgba(44, 30, 18, 0.3));
    border-color: var(--accent-gold, #c9a84c);
  }

  .card.selected {
    transform: translateY(-8px);
    border-color: var(--accent-gold, #c9a84c);
    box-shadow: var(--shadow-card-selected, 0 0 12px rgba(201, 168, 76, 0.6), 0 0 24px rgba(201, 168, 76, 0.3));
  }

  .card:disabled {
    opacity: 1;
    cursor: default;
  }
</style>
