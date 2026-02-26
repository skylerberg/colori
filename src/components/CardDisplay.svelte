<script lang="ts">
  import type { AnyCardData } from '../data/types';
  import { colorToHex, blendColors, textColorForBackground } from '../data/colors';
  import { getAnyCardData, getCardPips } from '../data/cards';
  import { formatSingleAbility, formatAbility } from '../data/abilities';

  let { card, selected = false, onclick }: {
    card: string;
    selected?: boolean;
    onclick?: () => void;
  } = $props();

  let data = $derived(getAnyCardData(card));
  let abilityText = $derived(data ? formatAbility(data) : '');
  let workshopText = $derived(
    data && data.kind === 'action'
      ? data.workshopAbilities.map(formatSingleAbility).join(', ')
      : ''
  );
  let pips = $derived(getCardPips(card));
  let blendedHex = $derived(blendColors(pips));
  let swatchTextColor = $derived(textColorForBackground(blendedHex));

  function cardKindLabel(c: AnyCardData): string {
    switch (c.kind) {
      case 'dye': return 'Dye';
      case 'basicDye': return 'Basic Dye';
      case 'material': return 'Material';
      case 'buyer': return 'Buyer';
      case 'action': return 'Reagent';
    }
  }
</script>

{#if data}
<button
  class="card"
  class:selected
  class:clickable={!!onclick}
  class:dye={data.kind === 'dye'}
  class:basic-dye={data.kind === 'basicDye'}
  class:material={data.kind === 'material'}
  class:buyer={data.kind === 'buyer'}
  class:action={data.kind === 'action'}
  onclick={onclick}
  disabled={!onclick}
>
  <div class="card-header">
    <span class="card-kind">{cardKindLabel(data)}</span>
    {#if data.kind === 'buyer'}
      <span class="stars">{'*'.repeat(data.stars)}</span>
    {/if}
  </div>

  {#if 'name' in data}
    <div class="card-name">{data.name}</div>
  {/if}

  {#if data.kind === 'dye' || data.kind === 'basicDye'}
    <div class="pips">
      {#each pips as pip}
        <span class="pip" style="background-color: {colorToHex(pip)}; color: {textColorForBackground(colorToHex(pip))}" title={pip}>{pip[0]}</span>
      {/each}
    </div>
    <div class="color-swatch" style="background-color: {blendedHex}"></div>
  {/if}

  {#if data.kind === 'material'}
    <div class="material-type">
      {#if data.materialTypes[0] === data.materialTypes[1]}
        2x {data.materialTypes[0]}
      {:else}
        {data.materialTypes.join(' + ')}
      {/if}
    </div>
    {#if data.colorPip}
      <div class="pips">
        <span class="pip" style="background-color: {colorToHex(data.colorPip)}; color: {textColorForBackground(colorToHex(data.colorPip))}" title={data.colorPip}>{data.colorPip[0]}</span>
      </div>
    {/if}
  {/if}

  {#if data.kind === 'buyer'}
    <div class="buyer-info">
      <div class="required-material">{data.requiredMaterial}</div>
      <div class="color-cost">
        {#each data.colorCost as color}
          <span class="pip" style="background-color: {colorToHex(color)}; color: {textColorForBackground(colorToHex(color))}" title={color}>{color[0]}</span>
        {/each}
      </div>
    </div>
  {/if}

  {#if workshopText}
    <div class="workshop-abilities">{workshopText}</div>
  {/if}

  {#if abilityText}
    <div class="ability">{abilityText}</div>
  {/if}
</button>
{/if}

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

  .card.material {
    border-left: 4px solid #8b6914;
  }

  .card.action {
    border-left: 4px solid #2ecc71;
  }

  .card.buyer {
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
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 1px solid rgba(0, 0, 0, 0.3);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.65rem;
    font-weight: bold;
  }

  .material-type {
    font-size: 0.75rem;
    color: #8b6914;
    font-weight: 600;
    padding: 2px 0;
  }

  .workshop-abilities {
    font-size: 0.75rem;
    color: #2ecc71;
    font-weight: 600;
    padding: 2px 0;
  }

  .buyer-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .required-material {
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
    width: 20px;
    height: 20px;
  }

  .color-swatch {
    height: 18px;
    border-radius: 3px;
    margin-top: auto;
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
