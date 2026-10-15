<script lang="ts">
  import type { Color, Choice } from '../data/types';
  import { isPrimary, mixWithWildPrimaries } from '../data/colors';
  import { mixWheelState } from '../stores/mixWheelState.svelte';
  import { onDestroy } from 'svelte';

  let { colorWheel, wildPrimaries, remaining, onAction }: {
    colorWheel: Record<Color, number>;
    wildPrimaries: number;
    remaining: number;
    onAction: (choice: Choice) => void;
  } = $props();

  let plannedMixes: [Color, Color][] = $state([]);
  let selectedMixColors: Color[] = $state([]);

  $effect(() => {
    colorWheel;
    wildPrimaries;
    plannedMixes = [];
    selectedMixColors = [];
  });

  // What the planned mixes leave behind, replayed from the real stock.
  let simulated = $derived.by(() => {
    let stock = { wheel: { ...colorWheel }, wildPrimaries };
    for (const [a, b] of plannedMixes) {
      stock = mixWithWildPrimaries(stock.wheel, stock.wildPrimaries, a, b) ?? stock;
    }
    return stock;
  });

  // Push state to the shared store so GameLayout's wheel becomes interactive
  $effect(() => {
    mixWheelState.simulatedWheel = simulated.wheel;
    mixWheelState.simulatedWildPrimaries = simulated.wildPrimaries;
    mixWheelState.selectedColors = selectedMixColors;
    mixWheelState.onColorClick = handleMixColorClick;
  });

  onDestroy(() => {
    mixWheelState.clear();
  });

  let mixRemaining = $derived(remaining - plannedMixes.length);

  function handleMixColorClick(color: Color) {
    const first = selectedMixColors[0];
    if (first === undefined) {
      selectedMixColors = [color];
    } else if (first === color) {
      selectedMixColors = [];
    } else if (mixWithWildPrimaries(simulated.wheel, simulated.wildPrimaries, first, color)) {
      plannedMixes = [...plannedMixes, [first, color]];
      selectedMixColors = [];

      // Auto-submit if all mixes used
      if (plannedMixes.length === remaining) {
        mixWheelState.clear();
        onAction({ type: 'mixAll', mixes: plannedMixes });
      }
    } else {
      selectedMixColors = [color];
    }
  }

  function handleSkipMix() {
    mixWheelState.clear();
    onAction({ type: 'mixAll', mixes: plannedMixes });
  }

  function handleUndoMix() {
    plannedMixes = plannedMixes.slice(0, -1);
    selectedMixColors = [];
  }

  let wildPrimaryHint = $derived.by(() => {
    if (simulated.wildPrimaries === 0) return '';
    const emptyPrimary = selectedMixColors.some(c => isPrimary(c) && simulated.wheel[c] === 0);
    return emptyPrimary
      ? ' This mix will spend a wild primary for the primary you lack.'
      : ` Wild primaries (${simulated.wildPrimaries}) can stand in for a primary you lack.`;
  });
</script>

<div class="prompt-section">
  <h3>Mix Colors: Select two adjacent colors on the wheel ({mixRemaining} remaining)</h3>
  <p class="hint">Click colors on the wheel to mix. Two adjacent colors with at least 1 stored each.{wildPrimaryHint}</p>
  <div class="mix-actions">
    <button class="skip-btn" onclick={handleSkipMix}>
      {plannedMixes.length > 0 ? 'Submit Mixes' : 'Skip Remaining Mixes'}
    </button>
    {#if plannedMixes.length > 0}
      <button class="skip-btn" onclick={handleUndoMix}>Undo Last Mix</button>
    {/if}
  </div>
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

  .mix-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  @media (min-width: 640px) {
    h3 {
      font-size: 0.9rem;
    }

    .skip-btn {
      padding: 8px 16px;
      font-size: 0.85rem;
      min-height: unset;
    }
  }
</style>
