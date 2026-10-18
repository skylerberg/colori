<script lang="ts">
  import type { GameState } from '../data/types';
  import type { TableInteraction } from '../stores/tableInteraction';
  import { BUYERS_BOARD, BUYERS_BOARD_HEIGHT, BUYERS_BOARD_WIDTH, assignSlots, type Rect } from '../data/boardLayout';
  import { BUYERS_BOARD_ART_URL } from '../data/cardArt';
  import { roundNews } from '../gameUtils';
  import CardDisplay from './CardDisplay.svelte';
  import { fly } from 'svelte/transition';

  let { gameState, market = null }: {
    gameState: GameState;
    market?: TableInteraction['market'];
  } = $props();

  function place(rect: Rect): string {
    return `left:calc(${rect.x} * var(--u));top:calc(${rect.y} * var(--u));`
      + `width:calc(${rect.width} * var(--u));height:calc(${rect.height} * var(--u));`;
  }

  let previousDisplay: (number | null)[] = [];
  let displaySlots = $derived.by(() => {
    previousDisplay = assignSlots(previousDisplay, gameState.sellCardDisplay.map(c => c.instanceId), BUYERS_BOARD.display.length);
    return previousDisplay;
  });
  let displayById = $derived(new Map(gameState.sellCardDisplay.map(c => [c.instanceId, c])));

  function roundTitle(round: number): string {
    const name = round === gameState.round ? `Round ${round} of ${gameState.maxRounds} (now)` : `Round ${round}`;
    return [name, ...roundNews(round)].join('\n');
  }

  const r = BUYERS_BOARD.roundRadius;
</script>

<div class="buyers-board" style:aspect-ratio="{BUYERS_BOARD_WIDTH} / {BUYERS_BOARD_HEIGHT}" style:--art-width={BUYERS_BOARD_WIDTH}>
  <img class="board-art" src={BUYERS_BOARD_ART_URL} alt="Buyer's board" draggable="false" />

  {#each BUYERS_BOARD.rounds as center, i}
    {@const round = i + 1}
    <div
      class="round"
      class:past={round < gameState.round}
      class:current={round === gameState.round}
      style={place({ x: center.x - r, y: center.y - r, width: 2 * r, height: 2 * r })}
      title={roundTitle(round)}
    >
      {#if round === gameState.round}<span class="now">Now</span>{/if}
    </div>
  {/each}

  <button class="sell-deck" class:claimable={!!market?.onDrawUnseen} style={place(BUYERS_BOARD.sellDeck)}
    disabled={!market?.onDrawUnseen}
    onclick={() => market?.onDrawUnseen?.()}
    title={market?.onDrawUnseen ? 'Claim the top card of the sell deck, unseen' : `Sell deck: ${gameState.sellCardDeck.length} cards`}>
    <span class="pile-count">{gameState.sellCardDeck.length}</span>
    <span class="pile-label">{market?.onDrawUnseen ? 'draw unseen' : 'sell deck'}</span>
  </button>

  {#each BUYERS_BOARD.display as slot, i}
    {@const id = displaySlots[i]}
    {@const sellCard = id !== null ? displayById.get(id) : undefined}
    <div class="display-slot" class:claimable={!!sellCard && !!market} style="{place(slot)}--card-height:calc({slot.height} * var(--u));">
      {#if sellCard}
        {#key sellCard.instanceId}
          <div class="display-card" in:fly={{ y: -20, duration: 300 }}>
            <CardDisplay card={sellCard.card} onclick={market ? () => market!.onClaim(sellCard.instanceId) : undefined} />
          </div>
        {/key}
      {/if}
    </div>
  {/each}
</div>

<style>
  .buyers-board {
    --u: calc(100cqw / var(--art-width));
    position: relative;
    width: 100%;
    container-type: inline-size;
    user-select: none;
  }

  .board-art {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    border-radius: calc(16 * var(--u));
    box-shadow: 0 calc(10 * var(--u)) calc(40 * var(--u)) rgba(0, 0, 0, 0.6);
    pointer-events: none;
  }

  .round {
    position: absolute;
    border-radius: 50%;
  }

  .round.past {
    background: rgba(10, 8, 5, 0.62);
  }

  .round.current {
    box-shadow: 0 0 0 max(calc(14 * var(--u)), 3px) #e8c766, 0 0 calc(60 * var(--u)) calc(16 * var(--u)) rgba(232, 199, 102, 0.9);
  }

  .now {
    position: absolute;
    left: 50%;
    top: 100%;
    transform: translate(-50%, calc(22 * var(--u)));
    padding: calc(2 * var(--u)) calc(16 * var(--u));
    border-radius: 999px;
    background: #e8c766;
    color: #2c1e12;
    font-family: 'Cinzel', serif;
    font-weight: 700;
    font-size: max(calc(34 * var(--u)), 9px);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    white-space: nowrap;
  }

  .sell-deck {
    position: absolute;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: calc(6 * var(--u));
    background: none;
    border: none;
    padding: 0;
    min-height: 0;
    border-radius: calc(16 * var(--u));
    color: #f5ede0;
    font-family: 'Cinzel', serif;
  }

  .sell-deck:disabled {
    opacity: 1;
    cursor: default;
  }

  .sell-deck.claimable {
    cursor: pointer;
    box-shadow: 0 0 0 calc(8 * var(--u)) #e8c766, 0 0 calc(40 * var(--u)) rgba(232, 199, 102, 0.8);
  }

  .sell-deck.claimable:hover {
    background: rgba(255, 255, 255, 0.15);
  }

  .pile-count {
    font-size: max(calc(110 * var(--u)), 16px);
    font-weight: 700;
    line-height: 1;
    text-shadow: 0 0 calc(18 * var(--u)) #000, 0 0 calc(6 * var(--u)) #000;
  }

  .pile-label {
    font-size: max(calc(34 * var(--u)), 9px);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    text-shadow: 0 0 calc(10 * var(--u)) #000, 0 0 calc(4 * var(--u)) #000;
  }

  .display-slot {
    position: absolute;
    border-radius: calc(16 * var(--u));
    transition: transform 0.18s ease-out;
  }

  .display-card {
    width: 100%;
    height: 100%;
  }

  .display-slot :global(.card) {
    border-radius: calc(16 * var(--u));
    box-shadow: 0 calc(4 * var(--u)) calc(16 * var(--u)) rgba(0, 0, 0, 0.55);
  }

  .display-slot :global(.card.clickable:hover),
  .display-slot :global(.card.clickable:active) {
    transform: none;
  }

  .display-slot.claimable :global(.card) {
    box-shadow: 0 0 0 calc(6 * var(--u)) rgba(232, 199, 102, 0.8), 0 calc(4 * var(--u)) calc(16 * var(--u)) rgba(0, 0, 0, 0.55);
  }

  .display-slot.claimable:hover {
    transform: scale(1.05);
    z-index: 1;
  }

  .display-slot.claimable:hover :global(.card) {
    box-shadow: 0 0 0 calc(9 * var(--u)) #e8c766, 0 0 calc(50 * var(--u)) rgba(232, 199, 102, 0.9);
  }
</style>
