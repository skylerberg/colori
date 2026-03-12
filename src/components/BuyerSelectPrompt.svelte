<script lang="ts">
  import type { GameState, Choice, Color, GlassCard } from '../data/types';
  import { canSell } from '../engine/wasmEngine';
  import { getGlassCardData } from '../data/glassCards';
  import { colorToHex, textColorForBackground } from '../data/colors';
  import { PRIMARIES } from '../data/cards';
  import BuyerDisplay from './BuyerDisplay.svelte';

  let { gameState, onAction }: {
    gameState: GameState;
    onAction: (choice: Choice) => void;
  } = $props();

  let selectedBuyerId: number | undefined = $state(undefined);
  let selectedGlassId: number | undefined = $state(undefined);
  let selectedPayColor: Color | undefined = $state(undefined);

  let currentPlayer = $derived(
    gameState.phase.type === 'action'
      ? gameState.players[gameState.phase.actionState.currentPlayerIndex]
      : null
  );

  let glassEnabled = $derived(
    gameState.expansions?.glass &&
    gameState.glassDisplay.length > 0 &&
    currentPlayer !== null &&
    PRIMARIES.some(c => currentPlayer!.colorWheel[c] >= 4)
  );

  let payableColors = $derived(
    currentPlayer ? PRIMARIES.filter(c => currentPlayer!.colorWheel[c] >= 4) : []
  );

  // Reset when gameState changes
  $effect(() => {
    const _gs = gameState;
    selectedBuyerId = undefined;
    selectedGlassId = undefined;
    selectedPayColor = undefined;
  });

  function toggleBuyerSelect(buyerInstanceId: number) {
    selectedGlassId = undefined;
    selectedPayColor = undefined;
    selectedBuyerId = selectedBuyerId === buyerInstanceId ? undefined : buyerInstanceId;
  }

  function toggleGlassSelect(glassInstanceId: number) {
    selectedBuyerId = undefined;
    if (selectedGlassId === glassInstanceId) {
      selectedGlassId = undefined;
      selectedPayColor = undefined;
    } else {
      selectedGlassId = glassInstanceId;
      selectedPayColor = payableColors.length === 1 ? payableColors[0] : undefined;
    }
  }

  function confirmBuyer() {
    if (selectedBuyerId === undefined) return;
    const buyerInstance = gameState.buyerDisplay.find(b => b.instanceId === selectedBuyerId);
    if (!buyerInstance) return;
    onAction({ type: 'selectBuyer', buyer: buyerInstance.card });
  }

  function confirmGlass() {
    if (selectedGlassId === undefined || selectedPayColor === undefined) return;
    const glassInstance = gameState.glassDisplay.find(g => g.instanceId === selectedGlassId);
    if (!glassInstance) return;
    onAction({ type: 'selectGlass', glass: glassInstance.card, payColor: selectedPayColor });
  }
</script>

<div class="prompt-section">
  <h3>Choose a Buyer{#if glassEnabled} or Glass Card{/if}</h3>
  <div class="side-by-side">
    <div class="buyer-side">
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

    {#if glassEnabled}
      <div class="glass-side">
        <div class="glass-section-title">Glass Cards <span class="glass-cost">(cost: 4 primary)</span></div>
        <div class="glass-options">
          {#each gameState.glassDisplay as glass (glass.instanceId)}
            {@const data = getGlassCardData(glass.card)}
            <button
              class="glass-option"
              class:selected={selectedGlassId === glass.instanceId}
              onclick={() => toggleGlassSelect(glass.instanceId)}
            >
              <span class="glass-name">{data.name}</span>
              <span class="glass-desc">{data.description}</span>
            </button>
          {/each}
        </div>

        {#if selectedGlassId !== undefined}
          <div class="pay-color-section">
            <span class="pay-label">Pay with:</span>
            <div class="color-buttons">
              {#each payableColors as color}
                <button
                  class="color-btn"
                  class:selected={selectedPayColor === color}
                  style="background-color: {colorToHex(color)}; color: {textColorForBackground(colorToHex(color))}"
                  onclick={() => selectedPayColor = color}
                >
                  {color}
                </button>
              {/each}
            </div>
          </div>
        {/if}

        <button
          class="confirm-btn"
          disabled={selectedGlassId === undefined || selectedPayColor === undefined}
          onclick={confirmGlass}
        >
          Confirm Glass
        </button>
      </div>
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

  .side-by-side {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .buyer-side {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .glass-side {
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .glass-section-title {
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 0.85rem;
    font-weight: 600;
    color: #c9a84c;
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  .glass-cost {
    font-size: 0.7rem;
    font-weight: 400;
    text-transform: none;
    letter-spacing: 0;
    color: rgba(201, 168, 76, 0.7);
  }

  .glass-options {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .glass-option {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: 10px 12px;
    background: rgba(100, 160, 200, 0.15);
    border: 1px solid rgba(100, 160, 200, 0.4);
    border-radius: 6px;
    cursor: pointer;
    text-align: left;
    min-height: 44px;
  }

  .glass-option:hover {
    background: rgba(100, 160, 200, 0.25);
  }

  .glass-option.selected {
    border-color: #c9a84c;
    background: rgba(201, 168, 76, 0.15);
  }

  .glass-name {
    font-family: 'Cormorant Garamond', serif;
    font-size: 0.85rem;
    font-weight: 600;
    color: rgba(245, 237, 224, 0.9);
  }

  .glass-desc {
    font-family: 'Cormorant Garamond', serif;
    font-size: 0.75rem;
    color: rgba(245, 237, 224, 0.5);
  }

  .pay-color-section {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .pay-label {
    font-family: 'Cormorant Garamond', serif;
    font-size: 0.85rem;
    color: rgba(245, 237, 224, 0.7);
  }

  .color-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .color-btn {
    padding: 10px 14px;
    font-weight: 600;
    font-size: 0.85rem;
    border: 2px solid rgba(0, 0, 0, 0.2);
    border-radius: 6px;
    cursor: pointer;
    min-height: 44px;
    min-width: 44px;
  }

  .color-btn.selected {
    border-color: #c9a84c;
    box-shadow: 0 0 8px rgba(201, 168, 76, 0.5);
  }

  .color-btn:hover {
    opacity: 0.85;
  }

  .confirm-btn {
    padding: 10px 20px;
    font-family: var(--font-display, 'Cinzel', serif);
    font-weight: 600;
    letter-spacing: 1px;
    background: var(--bg-deep, #2c1e12);
    color: var(--text-on-dark, #f5ede0);
    border: none;
    border-radius: 6px;
    align-self: stretch;
    cursor: pointer;
    min-height: 44px;
    font-size: 0.9rem;
  }

  .confirm-btn:hover:not(:disabled) {
    background: #3a2a1e;
  }

  .confirm-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  @media (min-width: 640px) {
    .side-by-side {
      flex-direction: row;
      gap: 16px;
    }

    .glass-side {
      min-width: 200px;
    }

    .glass-option {
      padding: 6px 10px;
      min-height: unset;
    }

    .color-btn {
      padding: 4px 12px;
      font-size: 0.8rem;
      min-height: unset;
      min-width: unset;
    }

    .confirm-btn {
      padding: 8px 20px;
      align-self: flex-start;
      min-height: unset;
      font-size: inherit;
    }
  }
</style>
