<script lang="ts">
  import type { Color, Choice } from '../data/types';
  import { canMix, mixResult } from '../data/colors';
  import { mixWheelState } from '../stores/mixWheelState.svelte';
  import { onDestroy } from 'svelte';

  let { colorWheel, remaining, onAction }: {
    colorWheel: Record<Color, number>;
    remaining: number;
    onAction: (choice: Choice) => void;
  } = $props();

  let plannedMixes: [Color, Color][] = $state([]);
  let selectedMixColors: Color[] = $state([]);
  let simulatedWheel: Record<Color, number> = $state({} as Record<Color, number>);

  // Sync simulatedWheel when colorWheel changes
  $effect(() => {
    simulatedWheel = { ...colorWheel };
    plannedMixes = [];
    selectedMixColors = [];
  });

  // Push state to the shared store so GameLayout's wheel becomes interactive
  $effect(() => {
    mixWheelState.simulatedWheel = simulatedWheel;
    mixWheelState.selectedColors = selectedMixColors;
    mixWheelState.onColorClick = handleMixColorClick;
  });

  onDestroy(() => {
    mixWheelState.clear();
  });

  let mixRemaining = $derived(remaining - plannedMixes.length);

  function handleMixColorClick(color: Color) {
    if (selectedMixColors.length === 0) {
      selectedMixColors = [color];
    } else if (selectedMixColors.length === 1) {
      const first = selectedMixColors[0];
      if (first === color) {
        selectedMixColors = [];
      } else if (canMix(first, color) && simulatedWheel[first] > 0 && simulatedWheel[color] > 0) {
        // Apply mix locally
        const result = mixResult(first, color);
        const newWheel = { ...simulatedWheel };
        newWheel[first]--;
        newWheel[color]--;
        newWheel[result]++;
        simulatedWheel = newWheel;
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
  }

  function handleSkipMix() {
    mixWheelState.clear();
    onAction({ type: 'mixAll', mixes: plannedMixes });
  }

  function handleUndoMix() {
    if (plannedMixes.length === 0) return;
    // Rebuild simulated wheel from scratch
    const newWheel = { ...colorWheel };
    const newMixes = plannedMixes.slice(0, -1);
    for (const [a, b] of newMixes) {
      const result = mixResult(a, b);
      newWheel[a]--;
      newWheel[b]--;
      newWheel[result]++;
    }
    simulatedWheel = newWheel;
    plannedMixes = newMixes;
    selectedMixColors = [];
  }
</script>

<div class="prompt-section">
  <h3>Mix Colors: Select two adjacent colors on the wheel ({mixRemaining} remaining)</h3>
  <p class="hint">Click colors on the wheel to mix. Two adjacent colors with at least 1 stored each.</p>
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
    padding: 8px 16px;
    font-size: 0.85rem;
    background: var(--bg-panel, #ebe3d3);
    border: 1px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 6px;
    align-self: flex-start;
  }

  .skip-btn:hover {
    background: #e0d6c3;
  }

  .mix-actions {
    display: flex;
    gap: 8px;
  }
</style>
