<script lang="ts">
  import type { Color, Choice } from '../data/types';
  import { colorToHex, textColorForBackground } from '../data/colors';
  import { TERTIARIES } from '../data/cards';

  let { colorWheel, onAction }: {
    colorWheel: Record<Color, number>;
    onAction: (choice: Choice) => void;
  } = $props();

  let selectedLoseColor: Color | null = $state(null);

  // Reset when colorWheel changes (new pending choice)
  $effect(() => {
    const _cw = colorWheel;
    selectedLoseColor = null;
  });
</script>

<div class="prompt-section">
  <h3>Swap Tertiary</h3>
  {#if selectedLoseColor === null}
    <p class="hint">Choose a tertiary color to lose</p>
    <div class="color-buttons">
      {#each TERTIARIES.filter(c => colorWheel[c] > 0) as color}
        <button
          class="color-btn"
          style="background-color: {colorToHex(color)}; color: {textColorForBackground(colorToHex(color))}"
          onclick={() => selectedLoseColor = color}
        >
          {color}
        </button>
      {/each}
    </div>
  {:else}
    <p class="hint">Losing {selectedLoseColor} — choose a tertiary color to gain</p>
    <div class="color-buttons">
      {#each TERTIARIES.filter(c => c !== selectedLoseColor) as color}
        <button
          class="color-btn"
          style="background-color: {colorToHex(color)}; color: {textColorForBackground(colorToHex(color))}"
          onclick={() => onAction({ type: 'swapTertiary', loseColor: selectedLoseColor!, gainColor: color })}
        >
          {color}
        </button>
      {/each}
    </div>
    <button class="skip-btn" onclick={() => selectedLoseColor = null}>Back</button>
  {/if}
</div>

<style>
  .prompt-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h3 {
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 0.9rem;
    color: var(--text-primary, #2c1e12);
    text-align: left;
  }

  .hint {
    font-size: 0.8rem;
    color: var(--text-tertiary, #9a8775);
    font-style: italic;
    text-align: left;
  }

  .skip-btn {
    padding: 10px 16px;
    font-size: 0.9rem;
    background: var(--bg-panel, #ebe3d3);
    border: 1px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 6px;
    align-self: flex-start;
    min-height: 44px;
  }

  .skip-btn:hover {
    background: #e0d6c3;
  }

  .color-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .color-btn {
    padding: 12px 16px;
    font-weight: 600;
    font-size: 0.9rem;
    border: 2px solid rgba(0, 0, 0, 0.2);
    border-radius: 8px;
    cursor: pointer;
    min-height: 44px;
    min-width: 44px;
    flex: 1 1 calc(33% - 8px);
  }

  .color-btn:hover {
    opacity: 0.85;
  }

  @media (min-width: 640px) {
    .skip-btn {
      padding: 8px 16px;
      font-size: 0.85rem;
      min-height: unset;
    }

    .color-btn {
      flex: 0 1 auto;
      padding: 10px 18px;
    }
  }
</style>
