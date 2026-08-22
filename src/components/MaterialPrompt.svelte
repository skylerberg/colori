<script lang="ts">
  import type { Choice } from '../data/types';
  import { ALL_MATERIAL_TYPES } from '../data/types';

  let { onAction }: {
    onAction: (choice: Choice) => void;
  } = $props();
</script>

<div class="prompt-section">
  <h3>Choose a material to gain</h3>
  <div class="material-buttons">
    {#each ALL_MATERIAL_TYPES as material}
      <button
        class="material-btn"
        onclick={() => onAction({ type: 'gainMaterial', material })}
      >
        {material}
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

  .material-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .material-btn {
    padding: 12px 20px;
    font-weight: 600;
    font-size: 1rem;
    color: var(--text-primary, #2c1e12);
    background-color: var(--parchment, #f2e5c8);
    border: 2px solid var(--accent-gold, #c9a84c);
    border-radius: 8px;
    cursor: pointer;
    min-height: 44px;
    min-width: 44px;
    flex: 1 1 0;
  }

  .material-btn:hover {
    opacity: 0.85;
  }

  @media (min-width: 640px) {
    .material-btn {
      flex: 0 1 auto;
      padding: 10px 18px;
      font-size: 0.9rem;
    }
  }
</style>
