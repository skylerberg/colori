<script lang="ts">
  import type { Card, Buyer } from '../data/types';
  import { getAnyCardData } from '../data/cards';
  import { CARD_BACK_URL, getCardArtUrl, getBuyerArtUrl } from '../data/cardArt';
  import { cardPreviewState } from '../stores/cardPreviewState.svelte';

  // The full card appears only once the pointer has rested on it a while.
  // Buyer cards are small, so theirs comes sooner.
  const SELL_CARD_HOVER_MS = 700;
  const CARD_HOVER_MS = 1000;
  const LONG_PRESS_MS = 450;

  let { card, instanceId, selected = false, faceDown = false, hint, onclick }: {
    card: string;
    // Picks the character portrait on a starter card.
    instanceId?: number;
    selected?: boolean;
    faceDown?: boolean;
    // Tooltip saying what clicking does; the card's name otherwise.
    hint?: string;
    onclick?: () => void;
  } = $props();

  let data = $derived(getAnyCardData(card));
  let cardLabel = $derived(faceDown ? 'Face-down card' : data && 'name' in data ? data.name : card);

  let artUrl = $derived(
    faceDown
      ? CARD_BACK_URL
      : data
        ? data.kind === 'buyer'
          ? getBuyerArtUrl(card as Buyer)
          : getCardArtUrl(card as Card, instanceId)
        : ''
  );

  let element: HTMLButtonElement | undefined = $state();

  // Long-press to preview
  let pressTimer: ReturnType<typeof setTimeout> | null = null;
  let didLongPress = false;
  let hoverTimer: ReturnType<typeof setTimeout> | null = null;

  function onPointerDown(e: PointerEvent) {
    if (faceDown || (e.pointerType === 'mouse' && e.button !== 0)) return;
    didLongPress = false;
    pressTimer = setTimeout(() => {
      didLongPress = true;
      endHover();
      if (artUrl && cardLabel) {
        cardPreviewState.open(card, artUrl, cardLabel);
      }
    }, LONG_PRESS_MS);
  }

  function clearPress() {
    if (pressTimer) {
      clearTimeout(pressTimer);
      pressTimer = null;
    }
  }

  function onPointerEnter(e: PointerEvent) {
    if (e.pointerType !== 'mouse' || faceDown) return;
    hoverTimer = setTimeout(() => {
      if (element) cardPreviewState.showHover(artUrl, cardLabel, element.getBoundingClientRect());
    }, data?.kind === 'buyer' ? SELL_CARD_HOVER_MS : CARD_HOVER_MS);
  }

  function endHover() {
    if (hoverTimer) {
      clearTimeout(hoverTimer);
      hoverTimer = null;
    }
    cardPreviewState.hideHover();
  }

  $effect(() => endHover);

  function handleClick() {
    if (didLongPress) {
      didLongPress = false;
      return;
    }
    if (onclick) {
      endHover();
      onclick();
    }
  }
</script>

{#if data || faceDown}
<button
  bind:this={element}
  class="card"
  class:selected
  class:clickable={!!onclick}
  class:inactive={!onclick}
  style="background-image: url('{artUrl}')"
  title={hint ?? cardLabel}
  onclick={handleClick}
  onpointerdown={onPointerDown}
  onpointerup={clearPress}
  onpointercancel={clearPress}
  onpointerenter={onPointerEnter}
  onpointerleave={() => { clearPress(); endHover(); }}
  oncontextmenu={(e) => e.preventDefault()}
>
</button>
{/if}

<style>
  /* Bridge and mini cards share a shape, narrower than the 5:7 the layout's
     card box assumes, so the width follows from the height. */
  .card {
    --height: var(--card-height, 126px);
    --width: calc(var(--height) * 57 / 87);
    width: var(--width);
    height: var(--height);
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
