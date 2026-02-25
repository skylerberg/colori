<script lang="ts">
  import type { GameState, Color } from '../data/types';
  import type { ColoriChoice } from '../data/types';
  import { canSell } from '../engine/wasmEngine';
  import { canMix, mixResult, colorToHex, textColorForBackground, ALL_COLORS } from '../data/colors';
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

  // Mix UI state
  let plannedMixes: [Color, Color][] = $state([]);
  let selectedMixColors: Color[] = $state([]);

  let simulatedWheel: Record<Color, number> = $state(
    Object.fromEntries(ALL_COLORS.map(c => [c, 0])) as Record<Color, number>
  );

  // Sync simulatedWheel when pendingChoice or currentPlayer changes
  $effect(() => {
    if (pendingChoice?.type === 'chooseMix' && currentPlayer) {
      simulatedWheel = { ...currentPlayer.colorWheel };
      plannedMixes = [];
      selectedMixColors = [];
    }
  });

  let mixRemaining = $derived(
    pendingChoice?.type === 'chooseMix'
      ? pendingChoice.remaining - plannedMixes.length
      : 0
  );

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
        if (pendingChoice?.type === 'chooseMix' && plannedMixes.length === pendingChoice.remaining) {
          onAction({ type: 'mixAll', mixes: plannedMixes });
        }
      } else {
        selectedMixColors = [color];
      }
    }
  }

  function handleSkipMix() {
    onAction({ type: 'mixAll', mixes: plannedMixes });
  }

  function handleUndoMix() {
    if (plannedMixes.length === 0 || !currentPlayer) return;
    // Rebuild simulated wheel from scratch
    const newWheel = { ...currentPlayer.colorWheel };
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

  // Tertiary swap UI state
  let selectedLoseColor: Color | null = $state(null);

  // Reset tertiary state when pendingChoice changes
  $effect(() => {
    if (pendingChoice?.type === 'chooseTertiaryToLose') {
      selectedLoseColor = null;
    }
  });

  let selectedBuyerId: number | undefined = $state(undefined);

  $effect(() => {
    const _pc = pendingChoice;
    selectedBuyerId = undefined;
  });

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
        <h3>Mix Colors: Select two adjacent colors to mix ({mixRemaining} remaining)</h3>
        <p class="hint">Mix two primary colors, or a primary and an adjacent secondary. They must each have at least 1 stored.</p>
        <ColorWheelDisplay
          wheel={simulatedWheel}
          interactive={true}
          onColorClick={handleMixColorClick}
          selectedColors={selectedMixColors}
        />
        <div class="mix-actions">
          <button class="skip-btn" onclick={handleSkipMix}>
            {plannedMixes.length > 0 ? 'Submit Mixes' : 'Skip Remaining Mixes'}
          </button>
          {#if plannedMixes.length > 0}
            <button class="skip-btn" onclick={handleUndoMix}>Undo Last Mix</button>
          {/if}
        </div>
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
        <h3>Swap Tertiary</h3>
        {#if selectedLoseColor === null}
          <p class="hint">Choose a tertiary color to lose</p>
          <div class="color-buttons">
            {#each TERTIARIES.filter(c => currentPlayer.colorWheel[c] > 0) as color}
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

  .mix-actions {
    display: flex;
    gap: 8px;
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
