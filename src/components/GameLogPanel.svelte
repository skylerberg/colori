<script lang="ts">
  import type { Ability, CardInstance, Color, GoldCoinPurchase, MaterialType, Buyer } from '../data/types';
  import { type LogChange, type LogEntry, type LogMove, isEmptyChange } from '../logEntries';
  import { colorToHex, mixResult, ALL_COLORS } from '../data/colors';
  import { getAnyCardData, getBuyerData } from '../data/cards';
  import { formatSingleAbility } from '../data/abilities';
  import { roundNews } from '../gameUtils';
  import CardDisplay from './CardDisplay.svelte';
  import CharacterPortrait from './CharacterPortrait.svelte';
  import { tick } from 'svelte';

  let { log, playerNames, onClose }: {
    log: LogEntry[];
    playerNames: string[];
    onClose: () => void;
  } = $props();

  const MATERIALS: MaterialType[] = ['Paintings', 'Ceramics', 'Textiles'];
  const MATERIAL_ICONS: Record<MaterialType, string> = {
    Paintings: '/icons/paintings.webp',
    Ceramics: '/icons/ceramics.webp',
    Textiles: '/icons/textiles.webp',
  };
  const PURCHASES: Record<GoldCoinPurchase, string> = { workshop: 'a workshop pick', mixColors: 'a mix', sell: 'a sale' };

  // -1 for everyone.
  let view = $state(-1);
  let scroller: HTMLDivElement | undefined = $state();

  let shown = $derived(
    view < 0 ? log : log.filter(e => e.kind === 'round' || e.player === view)
  );

  // Open at the latest move, and follow new ones unless scrolled back.
  let pinnedToEnd = true;
  $effect(() => {
    shown;
    if (!pinnedToEnd) return;
    tick().then(() => scroller?.scrollTo({ top: scroller.scrollHeight }));
  });

  function onScroll() {
    if (!scroller) return;
    pinnedToEnd = scroller.scrollTop + scroller.clientHeight >= scroller.scrollHeight - 24;
  }

  function selectView(i: number) {
    view = i;
    pinnedToEnd = true;
  }

  function cardName(card: string): string {
    return getAnyCardData(card).name;
  }

  function topActions(card: string): Ability[] {
    const data = getAnyCardData(card);
    return data.kind === 'buyer' ? [] : data.topActions;
  }

  function signed(n: number): string {
    return n > 0 ? `+${n}` : `−${-n}`;
  }

  function hidesColors(moves: LogMove[]): boolean {
    return moves.every(m => m.type === 'mix' || m.type === 'secondary');
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#snippet card(ci: CardInstance)}
  <span class="chip card-chip">
    <span class="thumb"><CardDisplay card={ci.card} instanceId={ci.instanceId >= 0 ? ci.instanceId : undefined} /></span>
    <span class="chip-name">{cardName(ci.card)}</span>
  </span>
{/snippet}

{#snippet buyer(buyer: Buyer)}
  <span class="chip card-chip">
    <span class="thumb sell"><CardDisplay card={buyer} /></span>
    <span class="chip-name">{getBuyerData(buyer).name}</span>
    <span class="points">{getBuyerData(buyer).points}</span>
  </span>
{/snippet}

{#snippet swatch(color: Color)}
  <span class="swatch" style:background={colorToHex(color)} title={color}></span>
{/snippet}

{#snippet move(m: LogMove)}
  <span class="move">
    {#if m.type === 'claim'}
      <span class="verb">{m.unseen ? 'drew' : 'claimed'}</span> {@render buyer(m.buyer)}
      {#if m.unseen}<span class="aside">unseen, from the buyer deck</span>{/if}
    {:else if m.type === 'picked'}
      <span class="verb">picked</span>
      {#each m.cards as ci (ci.instanceId)}{@render card(ci)}{/each}
    {:else if m.type === 'topAction'}
      <span class="verb">used</span> {@render card(m.card)} <span class="aside">top action</span>
      {#each topActions(m.card.card) as ability}
        <span class="ability">{formatSingleAbility(ability)}</span>
      {/each}
    {:else if m.type === 'trashWithoutUsing'}
      <span class="verb">trashed</span>
      {#each m.cards as ci (ci.instanceId)}{@render card(ci)}{/each}
      <span class="aside">without using {m.cards.length === 1 ? 'it' : 'them'}</span>
    {:else if m.type === 'keep'}
      <span class="verb">kept</span> {@render card(m.card)} <span class="aside">on top of their deck</span>
    {:else if m.type === 'workshop'}
      {#if m.cards.length > 0}
        <span class="verb">workshopped</span>
        {#each m.cards as ci, i (i)}{@render card(ci)}{/each}
      {:else}
        <span class="verb">skipped the workshop</span>
      {/if}
    {:else if m.type === 'mix'}
      {#if m.mixes.length > 0}
        <span class="verb">mixed</span>
        {#each m.mixes as [a, b], i (i)}
          <span class="chip mix-chip">
            {@render swatch(a)}<span class="op">+</span>{@render swatch(b)}<span class="op">→</span>{@render swatch(mixResult(a, b))}
            <span class="chip-name">{mixResult(a, b)}</span>
          </span>
        {/each}
      {:else}
        <span class="verb">skipped mixing</span>
      {/if}
    {:else if m.type === 'sell'}
      <span class="verb">sold to</span> {@render buyer(m.buyer)}
    {:else if m.type === 'lift'}
      {#if m.card}
        <span class="verb">lifted</span> {@render card(m.card)} <span class="aside">to their top cards</span>
      {:else}
        <span class="verb">had no card to lift</span>
      {/if}
    {:else if m.type === 'secondary'}
      <span class="verb">took</span>
      <span class="chip mix-chip">{@render swatch(m.color)}<span class="chip-name">{m.color}</span></span>
    {:else if m.type === 'spendGoldCoin'}
      <span class="verb">spent</span> <img class="icon" src="/icons/gold-coin.webp" alt="a gold coin" />
      <span class="aside">on {PURCHASES[m.purchase]}</span>
    {:else if m.type === 'endTurn'}
      <span class="verb muted">ended their turn</span>
    {/if}
  </span>
{/snippet}

{#snippet change(c: LogChange, showColors: boolean)}
  <div class="change">
    {#if c.points}<span class="delta points-delta" class:minus={c.points < 0}>{signed(c.points)} points</span>{/if}
    {#if showColors}
      {#each ALL_COLORS as color}
        {#if c.colors[color]}
          <span class="delta" class:minus={c.colors[color]! < 0}>{signed(c.colors[color]!)}{@render swatch(color)}</span>
        {/if}
      {/each}
    {/if}
    {#each MATERIALS as material}
      {#if c.materials[material]}
        <span class="delta" class:minus={c.materials[material]! < 0} title={material}>
          {signed(c.materials[material]!)}<img class="icon" src={MATERIAL_ICONS[material]} alt={material} />
        </span>
      {/if}
    {/each}
    {#if c.goldCoins}<span class="delta" class:minus={c.goldCoins < 0} title="Gold coins">{signed(c.goldCoins)}<img class="icon" src="/icons/gold-coin.webp" alt="gold coins" /></span>{/if}
    {#if c.boats}<span class="delta" class:minus={c.boats < 0} title="Boats">{signed(c.boats)}<img class="icon boat" src="/icons/boat.webp" alt="boats" /></span>{/if}
    {#if c.bowls}<span class="delta" class:minus={c.bowls < 0} title="Bowls">{signed(c.bowls)}<span class="wild"></span></span>{/if}
  </div>
{/snippet}

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="log-overlay" onclick={onClose} role="presentation">
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="log-drawer" onclick={(e) => e.stopPropagation()} role="dialog" aria-label="Game log" tabindex="-1">
    <header class="log-header">
      <span class="log-title">Game Log</span>
      <button class="close-btn" onclick={onClose} aria-label="Close the log">&times;</button>
    </header>

    <div class="views" role="tablist">
      <button class="view-tab" class:active={view < 0} role="tab" aria-selected={view < 0} onclick={() => selectView(-1)}>Everyone</button>
      {#each playerNames as name, i}
        <button class="view-tab" class:active={view === i} role="tab" aria-selected={view === i} onclick={() => selectView(i)}>
          <CharacterPortrait seat={i} size={22} />{name}
        </button>
      {/each}
    </div>

    <div class="entries" bind:this={scroller} onscroll={onScroll}>
      {#each shown as entry, i (i)}
        {#if entry.kind === 'round'}
          <div class="round">
            <span class="round-name">Round {entry.round}</span>
            {#each roundNews(entry.round) as news}<span class="round-news">{news}</span>{/each}
          </div>
        {:else if entry.kind === 'note'}
          <div class="note">{entry.text}</div>
        {:else}
          <div class="play" class:quiet={entry.moves.every(m => m.type === 'endTurn')}>
            {#if view < 0}
              <CharacterPortrait seat={entry.player} size={28} />
            {/if}
            <div class="play-body">
              <div class="moves" class:named={view < 0}>
                {#if view < 0}<span class="who">{playerNames[entry.player]}</span>{/if}
                {#each entry.moves as m, j}
                  {#if j > 0}<span class="then">then</span>{/if}
                  {@render move(m)}
                {/each}
              </div>
              {#if !isEmptyChange(entry.change)}
                {@render change(entry.change, !hidesColors(entry.moves))}
              {/if}
            </div>
          </div>
        {/if}
      {:else}
        <div class="note">Nothing yet.</div>
      {/each}
    </div>
  </div>
</div>

<style>
  .log-overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    justify-content: flex-end;
  }

  .log-drawer {
    width: min(560px, 100vw);
    height: 100%;
    display: flex;
    flex-direction: column;
    background: rgba(22, 16, 11, 0.98);
    border-left: 1px solid rgba(201, 168, 76, 0.5);
    box-shadow: -12px 0 40px rgba(0, 0, 0, 0.5);
    color: #f5ede0;
    text-align: left;
  }

  .log-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid rgba(201, 168, 76, 0.3);
  }

  .log-title {
    font-family: 'Cinzel', serif;
    font-size: 1rem;
    font-weight: 700;
    color: #e8c766;
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  .close-btn {
    background: none;
    border: none;
    color: rgba(245, 237, 224, 0.6);
    font-size: 1.6rem;
    line-height: 1;
    min-height: 0;
    padding: 2px 8px;
  }

  .close-btn:hover {
    color: #f5ede0;
    background: none;
  }

  .views {
    display: flex;
    gap: 6px;
    padding: 8px 14px;
    overflow-x: auto;
    border-bottom: 1px solid rgba(201, 168, 76, 0.2);
    scrollbar-width: none;
  }

  .view-tab {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
    min-height: 0;
    padding: 4px 12px 4px 6px;
    border-radius: 999px;
    border: 1px solid rgba(201, 168, 76, 0.3);
    background: rgba(40, 30, 20, 0.6);
    color: rgba(245, 237, 224, 0.7);
    font-family: 'Cinzel', serif;
    font-size: 0.75rem;
    font-weight: 600;
    white-space: nowrap;
  }

  .view-tab:first-child {
    padding-left: 12px;
  }

  .view-tab:hover {
    background: rgba(60, 45, 30, 0.8);
    color: #f5ede0;
  }

  .view-tab.active {
    background: rgba(201, 168, 76, 0.25);
    border-color: #c9a84c;
    color: #f5ede0;
  }

  .entries {
    flex: 1;
    overflow-y: auto;
    padding: 6px 14px 20px;
    overscroll-behavior: contain;
  }

  .round {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
    margin: 14px 0 6px;
    padding-bottom: 4px;
    border-bottom: 1px solid rgba(201, 168, 76, 0.35);
  }

  .round-name {
    font-family: 'Cinzel', serif;
    font-weight: 700;
    font-size: 0.85rem;
    color: #e8c766;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .round-news {
    font-family: 'Cormorant Garamond', serif;
    font-size: 0.9rem;
    font-style: italic;
    color: rgba(245, 237, 224, 0.65);
  }

  .note {
    padding: 4px 0;
    font-family: 'Cormorant Garamond', serif;
    font-style: italic;
    color: rgba(245, 237, 224, 0.55);
  }

  .play {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 7px 0;
    border-bottom: 1px solid rgba(201, 168, 76, 0.08);
  }

  .play.quiet {
    opacity: 0.6;
  }

  .play-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .moves {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 6px;
    font-family: 'Cormorant Garamond', serif;
    font-size: 1rem;
    line-height: 1.3;
  }

  .moves:not(.named) .move:first-of-type .verb {
    display: inline-block;
  }

  .moves:not(.named) .move:first-of-type .verb::first-letter {
    text-transform: uppercase;
  }

  .who {
    font-family: 'Cinzel', serif;
    font-weight: 700;
    font-size: 0.85rem;
    color: #f3dc8a;
  }

  .move {
    display: contents;
  }

  .verb.muted,
  .aside,
  .then {
    color: rgba(245, 237, 224, 0.6);
    font-style: italic;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 8px 2px 3px;
    border-radius: 6px;
    background: rgba(245, 237, 224, 0.07);
    border: 1px solid rgba(201, 168, 76, 0.25);
  }

  .card-chip .thumb {
    --card-height: 34px;
    display: inline-flex;
  }

  .card-chip .thumb :global(.card) {
    border-radius: 3px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
  }

  .chip-name {
    font-family: 'Cinzel', serif;
    font-size: 0.72rem;
    font-weight: 600;
    color: #f5ede0;
    white-space: nowrap;
  }

  .points {
    min-width: 18px;
    height: 18px;
    padding: 0 4px;
    border-radius: 999px;
    background: radial-gradient(circle at 35% 30%, #f3dc8a, #c9a84c 60%, #8a6d22);
    color: #2c1e12;
    font-family: 'Cinzel', serif;
    font-size: 0.7rem;
    font-weight: 700;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .mix-chip {
    padding: 3px 8px 3px 5px;
    gap: 3px;
  }

  .op {
    color: rgba(245, 237, 224, 0.6);
    font-size: 0.85rem;
  }

  .swatch {
    display: inline-block;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 1px solid rgba(0, 0, 0, 0.6);
    box-shadow: 0 0 0 1px rgba(245, 237, 224, 0.25);
    flex-shrink: 0;
  }

  .ability {
    padding: 1px 7px;
    border-radius: 999px;
    border: 1px solid rgba(201, 168, 76, 0.45);
    font-family: 'Cinzel', serif;
    font-size: 0.65rem;
    color: #e8c766;
    white-space: nowrap;
  }

  .icon {
    width: 18px;
    height: 18px;
    object-fit: contain;
    vertical-align: middle;
  }

  .change {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .delta {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 1px 7px 1px 6px;
    border-radius: 999px;
    background: rgba(76, 175, 80, 0.16);
    border: 1px solid rgba(129, 199, 132, 0.45);
    color: #c8e6c9;
    font-family: 'Cinzel', serif;
    font-size: 0.7rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }

  .delta.minus {
    background: rgba(198, 40, 40, 0.16);
    border-color: rgba(239, 154, 154, 0.4);
    color: #ffcdd2;
  }

  .points-delta:not(.minus) {
    background: rgba(201, 168, 76, 0.25);
    border-color: #c9a84c;
    color: #f3dc8a;
  }

  .wild {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: conic-gradient(#e5031a 0 120deg, #fdf70a 120deg 240deg, #0159fe 240deg 360deg);
    border: 1px solid rgba(0, 0, 0, 0.5);
  }
</style>
