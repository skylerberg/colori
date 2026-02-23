<script lang="ts">
  import type { GameState, Color } from '../data/types';
  import type { ColoriChoice } from '../ai/coloriGame';
  import { canSell } from '../engine/actionPhase';
  import { canMix, colorToHex, textColorForBackground } from '../data/colors';
  import { PRIMARIES, SECONDARIES, TERTIARIES } from '../data/cards';
  import ColorWheelDisplay from './ColorWheelDisplay.svelte';
  import BuyerDisplay from './BuyerDisplay.svelte';

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
  let selectedBuyerId: number | undefined = $state(undefined);

  $effect(() => {
    const _pc = pendingChoice;
    selectedMixColors = [];
    selectedBuyerId = undefined;
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

  function toggleBuyerSelect(buyerInstanceId: number) {
    selectedBuyerId = selectedBuyerId === buyerInstanceId ? undefined : buyerInstanceId;
  }

  function confirmBuyer() {
    if (selectedBuyerId === undefined) return;
    onAction({ type: 'selectBuyer', buyerInstanceId: selectedBuyerId });
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

    {:else if pendingChoice.type === 'chooseBuyer'}
      <div class="prompt-section">
        <h3>Choose a Buyer</h3>
        <BuyerDisplay
          buyers={gameState.buyerDisplay.filter(g => canSell(gameState, g.instanceId))}
          selectable={true}
          selectedId={selectedBuyerId}
          onSelect={toggleBuyerSelect}
        />
        <button
          class="confirm-btn"
          disabled={selectedBuyerId === undefined}
          onclick={confirmBuyer}
        >
          Confirm Buyer
        </button>
      </div>

    {:else if pendingChoice.type === 'choosePrimaryColor'}
      <div class="prompt-section">
        <h3>Choose a primary color to gain</h3>
        <div class="color-buttons">
          {#each PRIMARIES as color}
            <button
              class="color-btn"
              style="background-color: {colorToHex(color)}; color: {textColorForBackground(colorToHex(color))}"
              onclick={() => onAction({ type: 'gainPrimary', color })}
            >
              {color}
            </button>
          {/each}
        </div>
      </div>

    {:else if pendingChoice.type === 'chooseSecondaryColor'}
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

    {:else if pendingChoice.type === 'chooseTertiaryToLose'}
      <div class="prompt-section">
        <h3>Choose a tertiary color to lose</h3>
        <div class="color-buttons">
          {#each TERTIARIES.filter(c => currentPlayer.colorWheel[c] > 0) as color}
            <button
              class="color-btn"
              style="background-color: {colorToHex(color)}; color: {textColorForBackground(colorToHex(color))}"
              onclick={() => onAction({ type: 'chooseTertiaryToLose', color })}
            >
              {color}
            </button>
          {/each}
        </div>
      </div>

    {:else if pendingChoice.type === 'chooseTertiaryToGain'}
      <div class="prompt-section">
        <h3>Choose a tertiary color to gain</h3>
        <div class="color-buttons">
          {#each TERTIARIES.filter(c => c !== pendingChoice.lostColor) as color}
            <button
              class="color-btn"
              style="background-color: {colorToHex(color)}; color: {textColorForBackground(colorToHex(color))}"
              onclick={() => onAction({ type: 'chooseTertiaryToGain', color })}
            >
              {color}
            </button>
          {/each}
        </div>
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
