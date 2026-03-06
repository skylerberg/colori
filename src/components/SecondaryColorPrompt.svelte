<script lang="ts">
  import type { Choice } from '../data/types';
  import { colorToHex, textColorForBackground } from '../data/colors';
  import { SECONDARIES } from '../data/cards';

  let { onAction }: {
    onAction: (choice: Choice) => void;
  } = $props();
</script>

<div class="prompt-section">
  <h3>Choose a secondary color to gain</h3>
  <div class="color-buttons">
    {#each SECONDARIES as color}
      <button
        class="color-btn"
        style="background-color: {colorToHex(color)}; color: {textColorForBackground(colorToHex(color))}"
        onclick={() => onAction({ type: 'gainSecondary', color })}
      >
        {color}
      </button>
    {/each}
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

  .color-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .color-btn {
    padding: 10px 18px;
    font-weight: 600;
    font-size: 0.9rem;
    border: 2px solid rgba(0, 0, 0, 0.2);
    border-radius: 8px;
    cursor: pointer;
  }

  .color-btn:hover {
    opacity: 0.85;
  }
</style>
