<script lang="ts">
  import type { GameState, Color, CardInstance } from '../data/types';
  import { resolveMakeMaterials, resolveMixColors, skipMix, resolveDestroyCards, resolveChooseGarment, resolveGarmentPayment } from '../engine/actionPhase';
  import { canMix } from '../data/colors';
  import CardList from './CardList.svelte';
  import ColorWheelDisplay from './ColorWheelDisplay.svelte';
  import GarmentDisplay from './GarmentDisplay.svelte';

  let { gameState, onResolved }: {
    gameState: GameState;
    onResolved: () => void;
  } = $props();

  let actionState = $derived(
    gameState.phase.type === 'action' ? gameState.phase.actionState : null
  );

  let pendingChoice = $derived(actionState?.pendingChoice ?? null);

  let currentPlayer = $derived(
    actionState ? gameState.players[actionState.currentPlayerIndex] : null
  );

  // Make materials state
  let selectedMaterialIds: number[] = $state([]);

  // Destroy cards state
  let selectedDestroyIds: number[] = $state([]);

  // Mix colors state
  let selectedMixColors: Color[] = $state([]);

  // Reset local state when pendingChoice changes
  $effect(() => {
    // Read pendingChoice to track it
    const _pc = pendingChoice;
    selectedMaterialIds = [];
    selectedDestroyIds = [];
    selectedMixColors = [];
  });

  // -- Make Materials --
  function toggleMaterialCard(instanceId: number) {
    if (!pendingChoice || pendingChoice.type !== 'chooseCardsForMaterials') return;
    const idx = selectedMaterialIds.indexOf(instanceId);
    if (idx >= 0) {
      selectedMaterialIds = selectedMaterialIds.filter(id => id !== instanceId);
    } else if (selectedMaterialIds.length < pendingChoice.count) {
      selectedMaterialIds = [...selectedMaterialIds, instanceId];
    }
  }

  function confirmMaterials() {
    if (selectedMaterialIds.length === 0) return;
    resolveMakeMaterials(gameState, selectedMaterialIds);
    onResolved();
  }

  // -- Destroy Cards --
  function toggleDestroyCard(instanceId: number) {
    if (!pendingChoice || pendingChoice.type !== 'chooseCardsToDestroy') return;
    const idx = selectedDestroyIds.indexOf(instanceId);
    if (idx >= 0) {
      selectedDestroyIds = selectedDestroyIds.filter(id => id !== instanceId);
    } else if (selectedDestroyIds.length < pendingChoice.count) {
      selectedDestroyIds = [...selectedDestroyIds, instanceId];
    }
  }

  function confirmDestroy() {
    if (selectedDestroyIds.length === 0) return;
    resolveDestroyCards(gameState, selectedDestroyIds);
    onResolved();
  }

  // -- Mix Colors --
  function handleMixColorClick(color: Color) {
    if (selectedMixColors.length === 0) {
      selectedMixColors = [color];
    } else if (selectedMixColors.length === 1) {
      const first = selectedMixColors[0];
      if (first === color) {
        // Deselect
        selectedMixColors = [];
      } else if (canMix(first, color) && currentPlayer && currentPlayer.colorWheel[first] > 0 && currentPlayer.colorWheel[color] > 0) {
        // Perform mix
        resolveMixColors(gameState, first, color);
        selectedMixColors = [];
        onResolved();
      } else {
        // Pick a new first color
        selectedMixColors = [color];
      }
    }
  }

  function handleSkipMix() {
    skipMix(gameState);
    onResolved();
  }

  // -- Choose Garment --
  function handleGarmentSelect(garmentInstanceId: number) {
    resolveChooseGarment(gameState, garmentInstanceId);
    onResolved();
  }

  // -- Garment Payment --
  function confirmGarmentPayment() {
    resolveGarmentPayment(gameState);
    onResolved();
  }
</script>

{#if pendingChoice && currentPlayer}
  <div class="ability-prompt">
    {#if pendingChoice.type === 'chooseCardsForMaterials'}
      <div class="prompt-section">
        <h3>Make Materials: Select up to {pendingChoice.count} card(s) to store</h3>
        <CardList
          cards={currentPlayer.drawnCards}
          selectable={true}
          selectedIds={selectedMaterialIds}
          onCardClick={toggleMaterialCard}
        />
        <button
          class="confirm-btn"
          onclick={confirmMaterials}
          disabled={selectedMaterialIds.length === 0}
        >
          Confirm Materials ({selectedMaterialIds.length} selected)
        </button>
      </div>

    {:else if pendingChoice.type === 'chooseCardsToDestroy'}
      <div class="prompt-section">
        <h3>Destroy Cards: Select up to {pendingChoice.count} card(s) from your drawn cards to destroy</h3>
        <CardList
          cards={currentPlayer.drawnCards}
          selectable={true}
          selectedIds={selectedDestroyIds}
          onCardClick={toggleDestroyCard}
        />
        <button
          class="confirm-btn"
          onclick={confirmDestroy}
          disabled={selectedDestroyIds.length === 0}
        >
          Confirm Destroy ({selectedDestroyIds.length} selected)
        </button>
      </div>

    {:else if pendingChoice.type === 'chooseMix'}
      <div class="prompt-section">
        <h3>Mix Colors: Select two adjacent colors to mix ({pendingChoice.remaining} remaining)</h3>
        <p class="hint">Click two colors that are 2 apart on the wheel. They must each have at least 1 stored.</p>
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
          garments={gameState.garmentDisplay}
          selectable={true}
          onSelect={handleGarmentSelect}
        />
      </div>

    {:else if pendingChoice.type === 'chooseGarmentPayment'}
      <div class="prompt-section">
        <h3>Pay for Garment</h3>
        <button
          class="confirm-btn"
          onclick={confirmGarmentPayment}
        >
          Confirm Payment
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
