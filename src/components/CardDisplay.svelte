<script lang="ts">
  import type { Card, SellCard } from '../data/types';
  import { getAnyCardData } from '../data/cards';
  import { getCardArtUrl, getSellCardArtUrl } from '../data/cardArt';
  import { cardPreviewState } from '../stores/cardPreviewState.svelte';

  let { card, selected = false, rotated = false, onclick }: {
    card: string;
    selected?: boolean;
    rotated?: boolean;
    onclick?: () => void;
  } = $props();

  let data = $derived(getAnyCardData(card));
  let cardLabel = $derived(data && 'name' in data ? data.name : card);

  let artUrl = $derived(
    data
      ? data.kind === 'sellCard'
        ? getSellCardArtUrl(card as SellCard)
        : getCardArtUrl(card as Card)
      : ''
  );

  // Long-press to preview
  let pressTimer: ReturnType<typeof setTimeout> | null = null;
  let didLongPress = false;

  function onPointerDown() {
    didLongPress = false;
    pressTimer = setTimeout(() => {
      didLongPress = true;
      if (artUrl && cardLabel) {
        cardPreviewState.open(card, artUrl, cardLabel);
      }
    }, 400);
  }

  function onPointerUp() {
    if (pressTimer) {
      clearTimeout(pressTimer);
      pressTimer = null;
    }
  }

  function onPointerCancel() {
    if (pressTimer) {
      clearTimeout(pressTimer);
      pressTimer = null;
    }
  }

  function handleClick() {
    if (didLongPress) {
      didLongPress = false;
      return;
    }
    if (!rotated && onclick) onclick();
  }
</script>

{#if data}
<button
  class="card"
  class:selected
  class:clickable={!!onclick && !rotated}
  class:rotated
  class:inactive={rotated || !onclick}
  style="background-image: url('{artUrl}')"
  title={cardLabel}
  onclick={handleClick}
  onpointerdown={onPointerDown}
  onpointerup={onPointerUp}
  onpointercancel={onPointerCancel}
  onpointerleave={onPointerCancel}
  oncontextmenu={(e) => e.preventDefault()}
>
</button>
{/if}

<style>
  .card {
    width: var(--card-width, 90px);
    height: var(--card-height, 126px);
    border: none;
    border-radius: 4px;
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

  /* Ensure touch devices get visual feedback on active, not just hover */
  .card.clickable:active {
    transform: translateY(-2px) scale(1.01);
    box-shadow: var(--shadow-card-hover, 0 8px 20px rgba(44, 30, 18, 0.3));
    border-color: var(--accent-gold, #c9a84c);
  }

  .card.selected {
    transform: translateY(-6px);
    border-color: var(--accent-gold, #c9a84c);
    box-shadow: 0 0 8px rgba(201, 168, 76, 0.5), 0 0 16px rgba(201, 168, 76, 0.25);
  }

  .card.inactive {
    opacity: 1;
    cursor: default;
  }

  .card.rotated {
    transform: rotate(90deg);
    margin: calc((var(--card-width, 90px) - var(--card-height, 126px)) / 2) calc((var(--card-height, 126px) - var(--card-width, 90px)) / 2);
    opacity: 0.7;
  }

  @media (min-width: 768px) {
    .card {
      border-radius: 5px;
    }

    .card.selected {
      transform: translateY(-8px);
      box-shadow: var(--shadow-card-selected, 0 0 12px rgba(201, 168, 76, 0.6), 0 0 24px rgba(201, 168, 76, 0.3));
    }
  }

  @media (min-width: 1024px) {
    .card {
      border-radius: 6px;
    }
  }
</style>
