<script lang="ts">
  import type { CardInstance, Color, DucatPurchase, MaterialType, PlayerState, SellCard } from '../data/types';
  import type { TableInteraction } from '../stores/tableInteraction';
  import {
    BOARD_ART_WIDTH, PLAYER_BOARD_HEIGHT, PLAYER_CARD, PLAYER_SELL_CARD, TUCK,
    DRAFT_SLOT_X, WORKSHOP_SLOT_X, BUYER_SLOTS,
    COMPLETED_SLOT, DECK, DUCAT_BAG, MATERIALS_CIRCLE, BOATS, WILD_PRIMARIES, MATERIALS,
    DUCAT_PURCHASE_ICONS, DUCAT_PURCHASE_ICON_RADIUS, DUCAT_STRIP, WHEEL, WHEEL_REGIONS, assignSlots,
  } from '../data/boardLayout';
  import { playerBoardArtUrl, SELL_CARD_BACK_URL } from '../data/cardArt';
  import { saleShortfall } from '../engine/wasmEngine';
  import { getAnyCardData, getSellCardData } from '../data/cards';
  import { formatAbility } from '../data/abilities';
  import { buyerCapacity, orderByDraftOrder } from '../gameUtils';
  import CardDisplay from './CardDisplay.svelte';
  import CardModal from './CardModal.svelte';
  import { fly } from 'svelte/transition';

  let { player, seat, round, withTuckedCards = true, draftOrder = [], revealedDrafted = Infinity, claiming = false, interaction = null }: {
    player: PlayerState;
    seat: number;
    round: number;
    // Leave out the drafted and workshop cards, and the room they stick out into.
    withTuckedCards?: boolean;
    // The order this seat drafted its cards in, so they fill the slots left to right.
    draftOrder?: number[];
    // How many of its drafted cards, in that order, are face up; the rest are
    // still face down, waiting for this pick round to finish.
    revealedDrafted?: number;
    // This seat is claiming a buyer, so its next free slot is waiting for one.
    claiming?: boolean;
    interaction?: TableInteraction | null;
  } = $props();

  const MATERIAL_TYPES: MaterialType[] = ['Paintings', 'Ceramics', 'Textiles'];
  const PURCHASES: { type: DucatPurchase; label: string }[] = [
    { type: 'workshop', label: 'Workshop ×1' },
    { type: 'mixColors', label: 'Mix ×1' },
    { type: 'sell', label: 'Sell' },
  ];

  // Positions are given in pixels of the board art; the board scales with its width.
  function place(x: number, y: number, width?: number, height?: number): string {
    let style = `left:calc(${x} * var(--u));top:calc(${y} * var(--u));`;
    if (width !== undefined) style += `width:calc(${width} * var(--u));`;
    if (height !== undefined) style += `height:calc(${height} * var(--u));`;
    return style;
  }
  // The art sits below the drafted cards that stick out over its top edge.
  let overhang = $derived(withTuckedCards ? TUCK : 0);
  const onArt = (y: number) => y + overhang;

  let previousDrafted: (number | null)[] = [];
  let draftedSlots = $derived.by(() => {
    const ids = orderByDraftOrder(player.draftedCards, draftOrder).map(c => c.instanceId);
    previousDrafted = assignSlots(previousDrafted, ids, DRAFT_SLOT_X.length);
    return previousDrafted;
  });

  let previousWorkshop: (number | null)[] = [];
  let workshopSlots = $derived.by(() => {
    const ids = [...player.workshopCards, ...player.workshoppedCards].map(c => c.instanceId);
    previousWorkshop = assignSlots(previousWorkshop, ids, WORKSHOP_SLOT_X.length);
    return previousWorkshop;
  });

  let previousBuyers: (number | null)[] = [];
  let buyerSlots = $derived.by(() => {
    previousBuyers = assignSlots(previousBuyers, player.buyers.map(b => b.instanceId), BUYER_SLOTS.length);
    return previousBuyers;
  });

  let cardsById = $derived(new Map(
    [...player.draftedCards, ...player.workshopCards, ...player.workshoppedCards].map(c => [c.instanceId, c]),
  ));
  let buyersById = $derived(new Map(player.buyers.map(b => [b.instanceId, b])));
  let usedIds = $derived(new Set(player.workshoppedCards.map(c => c.instanceId)));
  let openBuyerSlots = $derived(buyerCapacity(round));
  let nextBuyerSlot = $derived(claiming ? buyerSlots.findIndex((id, i) => id === null && i < openBuyerSlots) : -1);

  let wheel = $derived(interaction?.wheel?.wheel ?? player.colorWheel);
  let wildPrimaries = $derived(interaction?.wheel?.wildPrimaries ?? player.wildPrimaries);
  let hoveredColor: Color | null = $state(null);
  // The colors a mix into the hovered color would use up.
  let previewedInputs: Color[] = $derived(hoveredColor ? interaction?.wheel?.spends[hoveredColor] ?? [] : []);

  // `concat` keeps the holes of an opponent's deck that online play leaves
  // unrevealed, where `flat` would drop them and the count with them.
  let deckCards = $derived(([] as CardInstance[]).concat(...player.deck));

  let showDeck = $state(false);
  let showCompleted = $state(false);

  function trashHint(card: string): string {
    const data = getAnyCardData(card);
    return `Trash ${data.name}: ${formatAbility(data)}`;
  }

  function buyerHint(card: SellCard, shortfall: string[]): string {
    const { name, points } = getSellCardData(card);
    if (shortfall.length === 0) return `${name}, ${points} points: ready to sell`;
    const counts = new Map<string, number>();
    for (const item of shortfall) counts.set(item, (counts.get(item) ?? 0) + 1);
    const needs = [...counts].map(([item, n]) => (n > 1 ? `${n} ${item}` : item)).join(', ');
    return `${name}, ${points} points: needs ${needs}`;
  }

  function colorState(color: Color): string {
    const w = interaction?.wheel;
    if (!w) return '';
    if (w.selected.includes(color)) return 'selected';
    if (w.pickable.includes(color)) return 'pickable';
    return 'muted';
  }

  function pickColor(color: Color) {
    const w = interaction?.wheel;
    if (w && (w.pickable.includes(color) || w.selected.includes(color))) w.onClick(color);
  }
</script>

{#snippet counter(value: number, x: number, y: number, label: string, kind = '', showZero = false)}
  {#if value > 0 || showZero}
    <div class="counter {kind}" class:zero={value === 0} style={place(x, onArt(y))} title="{label}: {value}" aria-label="{label}: {value}">
      {#key value}<span class="counter-value">{value}</span>{/key}
    </div>
  {/if}
{/snippet}

<div class="player-board" style:aspect-ratio="{BOARD_ART_WIDTH} / {PLAYER_BOARD_HEIGHT + 2 * overhang}" data-seat={seat}>
  <!-- Under the art: cards tucked beneath the top and bottom edges. -->
  {#if withTuckedCards}
  {#each DRAFT_SLOT_X as x, i}
    {@const id = draftedSlots[i]}
    {@const ci = id !== null ? cardsById.get(id) : undefined}
    <div class="tucked drafted" class:filled={!!ci} class:trashable={!!ci && !!interaction?.trashDrafted}
      style="{place(x, 0, PLAYER_CARD.width, PLAYER_CARD.height)}--card-height:calc({PLAYER_CARD.height} * var(--u));">
      {#if ci}
        {#key ci.instanceId}
          <div class="tucked-card" in:fly={{ y: -24, duration: 250 }}>
            <CardDisplay
              card={ci.card}
              instanceId={ci.instanceId}
              faceDown={i >= revealedDrafted}
              hint={interaction?.trashDrafted ? trashHint(ci.card) : undefined}
              onclick={interaction?.trashDrafted ? () => interaction!.trashDrafted!(ci.instanceId) : undefined}
            />
          </div>
        {/key}
      {/if}
    </div>
  {/each}

  {#each WORKSHOP_SLOT_X as x, i}
    {@const id = workshopSlots[i]}
    {@const ci = id !== null ? cardsById.get(id) : undefined}
    {@const used = ci ? usedIds.has(ci.instanceId) : false}
    {@const selectable = !!ci && !!interaction?.workshop?.selectable.includes(ci.instanceId)}
    {@const selected = !!ci && !!interaction?.workshop?.selected.includes(ci.instanceId)}
    <div class="tucked workshop" class:filled={!!ci} class:used class:selectable class:selected
      style="{place(x, PLAYER_BOARD_HEIGHT, PLAYER_CARD.width, PLAYER_CARD.height)}--card-height:calc({PLAYER_CARD.height} * var(--u));">
      {#if ci}
        {#key ci.instanceId}
          <div class="tucked-card" in:fly={{ y: 24, duration: 250 }}>
            <CardDisplay
              card={ci.card}
              instanceId={ci.instanceId}
              onclick={selectable ? () => interaction!.workshop!.onClick(ci.instanceId) : undefined}
            />
          </div>
        {/key}
        {#if used}<span class="mark used-mark" title="Workshopped this round">✓</span>{/if}
        {#if selected}<span class="mark selected-mark">✓</span>{/if}
      {/if}
    </div>
  {/each}
  {/if}

  <img class="board-art" src={playerBoardArtUrl(seat)} alt="Player board" draggable="false"
    style={place(0, overhang, BOARD_ART_WIDTH, PLAYER_BOARD_HEIGHT)} />

  <!-- On the art. -->
  <svg class="wheel" viewBox="0 0 {BOARD_ART_WIDTH} {PLAYER_BOARD_HEIGHT}" style={place(0, overhang, BOARD_ART_WIDTH, PLAYER_BOARD_HEIGHT)}
    class:interactive={!!interaction?.wheel}>
    {#each WHEEL_REGIONS as region}
      {@const regionState = colorState(region.color)}
      {@const clickable = regionState === 'pickable' || regionState === 'selected'}
      {#if clickable}
        <path
          d={region.path}
          class="region {regionState}"
          class:mix-input={previewedInputs.includes(region.color)}
          role="button"
          tabindex="0"
          aria-label="{region.color}: {wheel[region.color]}"
          onpointerenter={() => hoveredColor = region.color}
          onpointerleave={() => { if (hoveredColor === region.color) hoveredColor = null; }}
          onclick={() => pickColor(region.color)}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); pickColor(region.color); } }}
        ><title>{region.color}: {wheel[region.color]}</title></path>
      {:else}
        <path d={region.path} class="region {regionState}" class:mix-input={previewedInputs.includes(region.color)}>
          <title>{region.color}: {wheel[region.color]}</title>
        </path>
      {/if}
    {/each}
  </svg>

  {#each WHEEL_REGIONS as region}
    {@render counter(wheel[region.color], region.counter.x, region.counter.y, region.color)}
  {/each}
  {@render counter(wildPrimaries, WILD_PRIMARIES.x, WILD_PRIMARIES.y, 'Wild primaries (½ point each)', '', true)}

  {#each MATERIAL_TYPES as material}
    {@render counter(player.materials[material], MATERIALS[material].x, MATERIALS[material].y, material)}
  {/each}
  {@render counter(player.boats, BOATS.x, BOATS.y, 'Boats (½ point each)', '', true)}
  {@render counter(player.ducats, DUCAT_BAG.x, DUCAT_BAG.y, 'Ducats', 'ducats', true)}

  <button class="deck" style={place(DECK.x, onArt(DECK.y), DECK.width, DECK.height)} class:empty={deckCards.length === 0}
    onclick={() => showDeck = true} title="Deck: {deckCards.length} cards. Click to see them.">
    <span class="deck-count">{deckCards.length}</span>
    <span class="deck-label">{deckCards.length === 1 ? 'card' : 'cards'}</span>
  </button>

  {#each BUYER_SLOTS as slot, i}
    {@const id = buyerSlots[i]}
    {@const buyer = id !== null ? buyersById.get(id) : undefined}
    {@const sellable = !!buyer && !!interaction?.buyers?.selectable.includes(buyer.instanceId)}
    <div class="buyer-slot" style={place(slot.x, onArt(slot.y), PLAYER_SELL_CARD.width, PLAYER_SELL_CARD.height)}
      style:--card-height="calc({PLAYER_SELL_CARD.height} * var(--u))"
      class:awaiting={i === nextBuyerSlot}
      class:sellable
      class:unsellable={!!buyer && !!interaction?.buyers && !sellable}>
      {#if buyer}
        {@const shortfall = saleShortfall(player, buyer.card)}
        {#key buyer.instanceId}
          <div in:fly={{ x: -30, duration: 300 }}>
            <CardDisplay
              card={buyer.card}
              hint={sellable ? `Sell to this ${getSellCardData(buyer.card).name}` : buyerHint(buyer.card, shortfall)}
              onclick={sellable ? () => interaction!.buyers!.onClick(buyer.instanceId) : undefined}
            />
          </div>
        {/key}
        {#if shortfall.length === 0}
          <span class="ready" title="Ready to sell: everything this buyer asks for is on the board">✓</span>
        {/if}
      {/if}
    </div>
  {/each}

  <button class="completed" style={place(COMPLETED_SLOT.x, onArt(COMPLETED_SLOT.y), PLAYER_SELL_CARD.width, PLAYER_SELL_CARD.height)}
    disabled={player.completedSellCards.length === 0}
    onclick={() => showCompleted = true}
    title="Completed buyers: {player.completedSellCards.length}">
    {#if player.completedSellCards.length > 0}
      {#key player.completedSellCards.length}
        <img class="completed-back" src={SELL_CARD_BACK_URL} alt="" in:fly={{ y: -30, duration: 300 }} />
      {/key}
      <span class="completed-count">{player.completedSellCards.length}</span>
    {/if}
  </button>

  <!-- Regions the tutorial points at. -->
  {#if withTuckedCards}
    {@render tourAnchor('drafted', DRAFT_SLOT_X[0], 0, DRAFT_SLOT_X[3] + PLAYER_CARD.width - DRAFT_SLOT_X[0], TUCK + 80)}
    {@render tourAnchor('workshop', WORKSHOP_SLOT_X[0], PLAYER_BOARD_HEIGHT + TUCK - 60, WORKSHOP_SLOT_X[4] + PLAYER_CARD.width - WORKSHOP_SLOT_X[0], TUCK + 60)}
  {/if}
  {@render tourAnchor('wheel', WHEEL.cx - WHEEL.outer, onArt(WHEEL.cy - WHEEL.outer), 2 * WHEEL.outer, 2 * WHEEL.outer)}
  {@render tourAnchor('materials', MATERIALS_CIRCLE.x, onArt(MATERIALS_CIRCLE.y), MATERIALS_CIRCLE.width, MATERIALS_CIRCLE.height)}
  {@render tourAnchor('ducats', DUCAT_BAG.x - 100, onArt(DUCAT_STRIP.y - 130), DUCAT_STRIP.x + DUCAT_STRIP.width - DUCAT_BAG.x + 100, 210)}
  {@render tourAnchor('buyers', BUYER_SLOTS[0].x, onArt(BUYER_SLOTS[0].y), COMPLETED_SLOT.x + PLAYER_SELL_CARD.width - BUYER_SLOTS[0].x, COMPLETED_SLOT.y + PLAYER_SELL_CARD.height - BUYER_SLOTS[0].y)}
  {@render tourAnchor('deck', DECK.x, onArt(DECK.y), DECK.width, DECK.height)}

  {#if interaction?.ducats}
    {#each PURCHASES as purchase}
      {@const icon = DUCAT_PURCHASE_ICONS[purchase.type]}
      {@const available = interaction.ducats.available.includes(purchase.type)}
      {@const r = DUCAT_PURCHASE_ICON_RADIUS}
      <button class="purchase" class:available style={place(icon.x - r, onArt(icon.y - r), 2 * r, 2 * r)}
        disabled={!available}
        onclick={() => interaction!.ducats!.onSpend(purchase.type)}
        title={available ? `Spend a ducat: ${purchase.label}` : `Can't buy ${purchase.label} now`}
        aria-label="Spend a ducat: {purchase.label}"></button>
    {/each}
  {/if}
</div>

{#snippet tourAnchor(name: string, x: number, y: number, width: number, height: number)}
  <div class="tour-anchor" data-tour={name} style={place(x, y, width, height)}></div>
{/snippet}

{#if showDeck}
  <CardModal title="Deck ({deckCards.length})" cards={deckCards} onClose={() => showDeck = false} />
{/if}
{#if showCompleted}
  <CardModal title="Completed buyers ({player.completedSellCards.length})" cards={player.completedSellCards} onClose={() => showCompleted = false} />
{/if}

<style>
  .player-board {
    --u: 0.05cqw;
    position: relative;
    width: 100%;
    container-type: inline-size;
    user-select: none;
  }

  .board-art {
    position: absolute;
    z-index: 2;
    display: block;
    border-radius: calc(14 * var(--u));
    box-shadow: 0 calc(10 * var(--u)) calc(40 * var(--u)) rgba(0, 0, 0, 0.6);
  }

  .tucked {
    position: absolute;
    z-index: 1;
    transition: transform 0.18s ease-out;
  }

  .tucked-card,
  .buyer-slot > div {
    width: 100%;
    height: 100%;
  }

  .tucked :global(.card),
  .buyer-slot :global(.card),
  .completed :global(.card) {
    border-radius: calc(16 * var(--u));
    box-shadow: 0 calc(4 * var(--u)) calc(16 * var(--u)) rgba(0, 0, 0, 0.55);
  }

  /* The slot slides the card instead of the card lifting itself. */
  .tucked :global(.card.clickable:hover),
  .tucked :global(.card.clickable:active),
  .buyer-slot :global(.card.clickable:hover),
  .buyer-slot :global(.card.clickable:active) {
    transform: none;
  }

  .tucked:not(.filled)::before {
    content: '';
    position: absolute;
    left: 6%;
    right: 6%;
    border: calc(4 * var(--u)) dashed rgba(245, 237, 224, 0.18);
    border-radius: calc(16 * var(--u));
  }

  .drafted:not(.filled)::before {
    top: 12%;
    bottom: 50%;
    border-bottom: none;
  }

  .workshop:not(.filled)::before {
    top: 50%;
    bottom: 12%;
    border-top: none;
  }

  .drafted.trashable:hover {
    transform: translateY(calc(-36 * var(--u)));
  }

  .drafted.trashable :global(.card) {
    box-shadow: 0 0 0 calc(5 * var(--u)) rgba(201, 168, 76, 0.75), 0 calc(4 * var(--u)) calc(16 * var(--u)) rgba(0, 0, 0, 0.55);
  }

  .drafted.trashable:hover :global(.card) {
    box-shadow: 0 0 0 calc(7 * var(--u)) #e8c766, 0 0 calc(40 * var(--u)) rgba(232, 199, 102, 0.8);
  }

  .workshop.used :global(.card) {
    filter: grayscale(0.7) brightness(0.55);
  }

  .workshop.selectable :global(.card) {
    box-shadow: 0 0 0 calc(5 * var(--u)) rgba(201, 168, 76, 0.75), 0 calc(4 * var(--u)) calc(16 * var(--u)) rgba(0, 0, 0, 0.55);
  }

  .workshop.selectable:hover {
    transform: translateY(calc(24 * var(--u)));
  }

  .workshop.selected {
    transform: translateY(calc(80 * var(--u)));
  }

  .workshop.selected :global(.card) {
    box-shadow: 0 0 0 calc(8 * var(--u)) #e8c766, 0 0 calc(40 * var(--u)) rgba(232, 199, 102, 0.8);
  }

  .mark {
    position: absolute;
    right: 8%;
    bottom: 6%;
    width: max(calc(64 * var(--u)), 18px);
    height: max(calc(64 * var(--u)), 18px);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: max(calc(40 * var(--u)), 12px);
    font-weight: 700;
    pointer-events: none;
  }

  .used-mark {
    background: rgba(20, 15, 10, 0.85);
    color: rgba(245, 237, 224, 0.75);
    border: calc(3 * var(--u)) solid rgba(245, 237, 224, 0.4);
  }

  .selected-mark {
    background: #e8c766;
    color: #2c1e12;
  }

  /* ── Wheel ── */
  .wheel {
    position: absolute;
    z-index: 3;
    pointer-events: none;
  }

  .region {
    fill: transparent;
    stroke: none;
    outline: none;
    transition: fill 0.15s;
  }

  .wheel.interactive .region {
    pointer-events: auto;
  }

  .region.muted {
    fill: rgba(10, 8, 5, 0.55);
  }

  .region.pickable {
    cursor: pointer;
    stroke: rgba(255, 255, 255, 0.9);
    stroke-width: 5;
    animation: pickable-pulse 1.6s ease-in-out infinite;
  }

  .region.pickable:hover,
  .region.pickable:focus-visible {
    fill: rgba(255, 255, 255, 0.28);
    stroke-width: 8;
    outline: none;
  }

  .region.selected {
    cursor: pointer;
    fill: rgba(232, 199, 102, 0.45);
    stroke: #e8c766;
    stroke-width: 12;
  }

  .region.mix-input {
    stroke: #fff;
    stroke-width: 8;
    stroke-dasharray: 18 12;
    fill: rgba(255, 255, 255, 0.18);
  }

  @keyframes pickable-pulse {
    50% { stroke-opacity: 0.35; }
  }

  /* ── Counters ── */
  .counter {
    position: absolute;
    z-index: 4;
    transform: translate(-50%, -50%);
    min-width: max(calc(62 * var(--u)), 20px);
    height: max(calc(62 * var(--u)), 20px);
    padding: 0 calc(10 * var(--u));
    border-radius: 999px;
    background: rgba(20, 15, 10, 0.9);
    border: max(calc(4 * var(--u)), 1.5px) solid #c9a84c;
    color: #fff;
    font-family: 'Cinzel', serif;
    font-weight: 700;
    font-size: max(calc(38 * var(--u)), 12px);
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 calc(3 * var(--u)) calc(10 * var(--u)) rgba(0, 0, 0, 0.5);
    pointer-events: none;
  }

  .counter.zero {
    opacity: 0.6;
    border-color: rgba(201, 168, 76, 0.5);
  }

  .counter.ducats {
    min-width: max(calc(96 * var(--u)), 26px);
    height: max(calc(96 * var(--u)), 26px);
    font-size: max(calc(56 * var(--u)), 15px);
    background: radial-gradient(circle at 35% 30%, #f3dc8a, #c9a84c 55%, #8a6d22);
    color: #2c1e12;
    border-color: #f5ede0;
  }

  .counter-value {
    display: inline-block;
    animation: counter-pop 0.35s ease-out;
  }

  @keyframes counter-pop {
    0% { transform: scale(1.6); }
    100% { transform: scale(1); }
  }

  /* ── Deck ── */
  .deck {
    position: absolute;
    z-index: 3;
    background: none;
    border: none;
    padding: 0;
    min-height: 0;
    border-radius: calc(16 * var(--u));
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: calc(4 * var(--u));
    color: #f5ede0;
    font-family: 'Cinzel', serif;
    cursor: pointer;
  }

  .deck:hover {
    background: rgba(255, 255, 255, 0.08);
    box-shadow: 0 0 0 calc(5 * var(--u)) rgba(201, 168, 76, 0.6);
  }

  .deck.empty {
    background: rgba(10, 8, 5, 0.6);
  }

  .deck-count {
    font-size: max(calc(110 * var(--u)), 18px);
    font-weight: 700;
    line-height: 1;
    text-shadow: 0 0 calc(18 * var(--u)) #000, 0 0 calc(6 * var(--u)) #000;
  }

  .deck-label {
    font-size: max(calc(36 * var(--u)), 10px);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    text-shadow: 0 0 calc(10 * var(--u)) #000, 0 0 calc(4 * var(--u)) #000;
  }

  /* ── Buyers ── */
  .buyer-slot {
    position: absolute;
    z-index: 3;
    border-radius: calc(16 * var(--u));
    transition: transform 0.18s ease-out;
  }

  .buyer-slot.awaiting {
    box-shadow: 0 0 0 max(calc(12 * var(--u)), 3px) #e8c766, 0 0 calc(60 * var(--u)) rgba(232, 199, 102, 0.9);
    animation: awaiting-pulse 1.4s ease-in-out infinite;
  }


  .buyer-slot.sellable :global(.card) {
    box-shadow: 0 0 0 calc(7 * var(--u)) #e8c766, 0 0 calc(40 * var(--u)) rgba(232, 199, 102, 0.8);
  }

  .buyer-slot.sellable:hover {
    transform: scale(1.04);
  }

  .ready {
    position: absolute;
    left: 0;
    top: 0;
    transform: translate(-30%, -30%);
    width: max(calc(64 * var(--u)), 18px);
    height: max(calc(64 * var(--u)), 18px);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #2f7d32;
    border: max(calc(4 * var(--u)), 1.5px) solid #c8e6c9;
    color: #fff;
    font-size: max(calc(38 * var(--u)), 11px);
    font-weight: 700;
    box-shadow: 0 calc(3 * var(--u)) calc(10 * var(--u)) rgba(0, 0, 0, 0.5);
    pointer-events: none;
  }

  .buyer-slot.unsellable :global(.card) {
    filter: grayscale(0.6) brightness(0.5);
  }

  @keyframes awaiting-pulse {
    50% { box-shadow: 0 0 0 max(calc(12 * var(--u)), 3px) rgba(232, 199, 102, 0.45), 0 0 calc(20 * var(--u)) rgba(232, 199, 102, 0.3); }
  }


  .completed {
    position: absolute;
    z-index: 3;
    background: none;
    border: none;
    padding: 0;
    min-height: 0;
    border-radius: calc(16 * var(--u));
    cursor: pointer;
  }

  .completed:disabled {
    cursor: default;
    opacity: 1;
  }

  .completed-back {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border-radius: calc(16 * var(--u));
    box-shadow: 0 calc(4 * var(--u)) calc(16 * var(--u)) rgba(0, 0, 0, 0.55);
  }

  .completed:not(:disabled):hover .completed-back {
    box-shadow: 0 0 0 calc(5 * var(--u)) rgba(201, 168, 76, 0.7), 0 calc(4 * var(--u)) calc(16 * var(--u)) rgba(0, 0, 0, 0.55);
  }

  .completed-count {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    min-width: max(calc(110 * var(--u)), 28px);
    height: max(calc(110 * var(--u)), 28px);
    padding: 0 calc(14 * var(--u));
    border-radius: 999px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(20, 15, 10, 0.88);
    border: max(calc(5 * var(--u)), 2px) solid #c9a84c;
    color: #fff;
    font-family: 'Cinzel', serif;
    font-weight: 700;
    font-size: max(calc(64 * var(--u)), 16px);
    box-shadow: 0 calc(4 * var(--u)) calc(14 * var(--u)) rgba(0, 0, 0, 0.6);
  }

  .tour-anchor {
    position: absolute;
    pointer-events: none;
  }

  /* ── Spending ducats ── */
  .purchase {
    position: absolute;
    z-index: 4;
    padding: 0;
    min-height: 0;
    border-radius: 50%;
    border: none;
    background: rgba(10, 8, 5, 0.55);
  }

  .purchase.available {
    background: transparent;
    box-shadow: 0 0 0 calc(5 * var(--u)) #e8c766, 0 0 calc(24 * var(--u)) rgba(232, 199, 102, 0.9);
    cursor: pointer;
  }

  .purchase.available:hover {
    background: rgba(255, 255, 255, 0.25);
  }

  .purchase:disabled {
    opacity: 1;
    cursor: default;
  }
</style>
