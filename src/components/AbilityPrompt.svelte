<script lang="ts">
  import type { GameState, Color } from '../data/types';
  import type { ColoriChoice } from '../ai/coloriGame';
  import { canMakeGarment } from '../engine/actionPhase';
  import { canMix } from '../data/colors';
  import ColorWheelDisplay from './ColorWheelDisplay.svelte';
  import GarmentDisplay from './GarmentDisplay.svelte';

  let { gameState, onAction }: {
    gameState: GameState;
    onAction: (choice: ColoriChoice) => void;
  } = $props();

  let actionState = $derived(
    gameState.phase.type === 'action' ? gameState.phase.actionState : null
  );

  let pendingChoice = $derived(actionState?.pendingChoice ?? null);

  let currentPlayer = $derived(
    actionState ? gameState.players[actionState.currentPlayerIndex] : null
  );

  let selectedMixColors: Color[] = $state([]);
  let selectedGarmentId: number | undefined = $state(undefined);

  $effect(() => {
    const _pc = pendingChoice;
    selectedMixColors = [];
    selectedGarmentId = undefined;
  });

  function handleMixColorClick(color: Color) {
    if (selectedMixColors.length === 0) {
      selectedMixColors = [color];
    } else if (selectedMixColors.length === 1) {
      const first = selectedMixColors[0];
      if (first === color) {
        selectedMixColors = [];
      } else if (canMix(first, color) && currentPlayer && currentPlayer.colorWheel[first] > 0 && currentPlayer.colorWheel[color] > 0) {
        onAction({ type: 'mix', colorA: first, colorB: color });
        selectedMixColors = [];
      } else {
        selectedMixColors = [color];
      }
    }
  }

  function handleSkipMix() {
    onAction({ type: 'skipMix' });
  }

  function toggleGarmentSelect(garmentInstanceId: number) {
    selectedGarmentId = selectedGarmentId === garmentInstanceId ? undefined : garmentInstanceId;
  }

  function confirmGarment() {
    if (selectedGarmentId === undefined) return;
    onAction({ type: 'selectGarment', garmentInstanceId: selectedGarmentId });
  }
</script>

{#if pendingChoice && currentPlayer}
  <div class="ability-prompt">
    {#if pendingChoice.type === 'chooseMix'}
      <div class="prompt-section">
        <h3>Mix Colors: Select two adjacent colors to mix ({pendingChoice.remaining} remaining)</h3>
        <p class="hint">Mix two primary colors, or a primary and an adjacent secondary. They must each have at least 1 stored.</p>
        <ColorWheelDisplay
          wheel={currentPlayer.colorWheel}
          interactive={true}
          onColorClick={handleMixColorClick}
          selectedColors={selectedMixColors}
        />
        <button class="skip-btn" onclick={handleSkipMix}>Skip Remaining Mixes</button>
      </div>

    {:else if pendingChoice.type === 'chooseGarment'}
      <div class="prompt-section">
        <h3>Choose a Garment to make</h3>
        <GarmentDisplay
          garments={gameState.garmentDisplay.filter(g => canMakeGarment(gameState, g.instanceId))}
          selectable={true}
          selectedId={selectedGarmentId}
          onSelect={toggleGarmentSelect}
        />
        <button
          class="confirm-btn"
          disabled={selectedGarmentId === undefined}
          onclick={confirmGarment}
        >
          Confirm Garment
        </button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .ability-prompt {
    border: 2px solid #d4a017;
    border-radius: 10px;
    padding: 16px;
    background: #fffef0;
  }

  .prompt-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h3 {
    font-size: 0.95rem;
    color: #4a3728;
    text-align: left;
  }

  .hint {
    font-size: 0.8rem;
    color: #888;
    font-style: italic;
    text-align: left;
  }

  .confirm-btn {
    padding: 8px 20px;
    font-weight: 600;
    background: #2a6bcf;
    color: #fff;
    border: none;
    border-radius: 6px;
    align-self: flex-start;
  }

  .confirm-btn:hover:not(:disabled) {
    background: #1e56a8;
  }

  .confirm-btn:disabled {
    background: #aaa;
    cursor: not-allowed;
  }

  .skip-btn {
    padding: 8px 16px;
    font-size: 0.85rem;
    background: #eee;
    border: 1px solid #ccc;
    border-radius: 6px;
    align-self: flex-start;
  }

  .skip-btn:hover {
    background: #ddd;
  }


</style>
