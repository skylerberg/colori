<script lang="ts">
  import type { AnyCard } from '../data/types';
  import { colorToHex } from '../data/colors';
  import { getCardPips } from '../data/cards';

  let { card, selected = false, onclick }: {
    card: AnyCard;
    selected?: boolean;
    onclick?: () => void;
  } = $props();

  let abilityText = $derived(formatAbility(card));
  let pips = $derived(getCardPips(card));

  function formatAbility(c: AnyCard): string {
    if (c.kind === 'garment') return '';
    const a = c.ability;
    switch (a.type) {
      case 'makeMaterials': return `Materials x${a.count}`;
      case 'drawCards': return `Draw x${a.count}`;
      case 'mixColors': return `Mix x${a.count}`;
      case 'destroyCards': return `Destroy x${a.count}`;
      case 'makeGarment': return 'Make Garment';
    }
  }

  function cardKindLabel(c: AnyCard): string {
    switch (c.kind) {
      case 'dye': return 'Dye';
      case 'basicDye': return 'Basic Dye';
      case 'fabric': return 'Fabric';
      case 'garment': return 'Garment';
    }
  }
</script>

<button
  class="card"
  class:selected
  class:clickable={!!onclick}
  class:dye={card.kind === 'dye'}
  class:basic-dye={card.kind === 'basicDye'}
  class:fabric={card.kind === 'fabric'}
  class:garment={card.kind === 'garment'}
  onclick={onclick}
  disabled={!onclick}
>
  <div class="card-header">
    <span class="card-kind">{cardKindLabel(card)}</span>
    {#if card.kind === 'garment'}
      <span class="stars">{'*'.repeat(card.stars)}</span>
    {/if}
  </div>

  <div class="card-name">{card.name}</div>

  {#if card.kind === 'dye' || card.kind === 'basicDye'}
    <div class="pips">
      {#each pips as pip}
        <span class="pip" style="background-color: {colorToHex(pip)}" title={pip}></span>
      {/each}
    </div>
  {/if}

  {#if card.kind === 'fabric'}
    <div class="fabric-type">{card.fabricType}</div>
  {/if}

  {#if card.kind === 'garment'}
    <div class="garment-info">
      <div class="required-fabric">{card.requiredFabric}</div>
      <div class="color-cost">
        {#each card.colorCost as color}
          <span class="pip" style="background-color: {colorToHex(color)}" title={color}></span>
        {/each}
      </div>
    </div>
  {/if}

  {#if abilityText}
    <div class="ability">{abilityText}</div>
  {/if}
</button>

<style>
  .card {
    width: 120px;
    height: 160px;
    border: 2px solid #666;
    border-radius: 8px;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    background: #fffef7;
    font-size: 0.7rem;
    flex-shrink: 0;
    text-align: left;
    cursor: default;
    overflow: hidden;
  }

  .card.clickable {
    cursor: pointer;
  }

  .card.clickable:hover {
    border-color: #4a90d9;
    background: #f0f4ff;
  }

  .card.selected {
    border-color: #2a6bcf;
    background: #dbe8ff;
    box-shadow: 0 0 6px rgba(42, 107, 207, 0.5);
  }

  .card:disabled {
    opacity: 1;
    cursor: default;
  }

  .card.dye {
    border-left: 4px solid #b33;
  }

  .card.basic-dye {
    border-left: 4px solid #999;
  }

  .card.fabric {
    border-left: 4px solid #8b6914;
  }

  .card.garment {
    border-left: 4px solid #9b59b6;
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .card-kind {
    font-size: 0.6rem;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .stars {
    color: #d4a017;
    font-size: 0.8rem;
    font-weight: bold;
    letter-spacing: 1px;
  }

  .card-name {
    font-weight: 600;
    font-size: 0.7rem;
    line-height: 1.2;
    min-height: 2em;
  }

  .pips {
    display: flex;
    flex-wrap: wrap;
    gap: 3px;
  }

  .pip {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 1px solid rgba(0, 0, 0, 0.3);
    display: inline-block;
  }

  .fabric-type {
    font-size: 0.75rem;
    color: #8b6914;
    font-weight: 600;
    padding: 2px 0;
  }

  .garment-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .required-fabric {
    font-size: 0.65rem;
    color: #8b6914;
    font-style: italic;
  }

  .color-cost {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
  }

  .color-cost .pip {
    width: 10px;
    height: 10px;
  }

  .ability {
    margin-top: auto;
    font-size: 0.65rem;
    color: #555;
    font-style: italic;
    border-top: 1px solid #eee;
    padding-top: 3px;
  }
</style>
