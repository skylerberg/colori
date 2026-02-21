<script lang="ts">
  import type { GameState, Color, CardInstance } from '../data/types';
  import { resolveMakeMaterials, resolveMixColors, skipMix, resolveDestroyCards, resolveSelectGarment, canMakeGarment } from '../engine/actionPhase';
  import { canMix, mixResult } from '../data/colors';
  import CardList from './CardList.svelte';
  import ColorWheelDisplay from './ColorWheelDisplay.svelte';
  import GarmentDisplay from './GarmentDisplay.svelte';

  let { gameState, onResolved, onLog }: {
    gameState: GameState;
    onResolved: () => void;
    onLog: (entry: string) => void;
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

  // Garment selection state
  let selectedGarmentId: number | undefined = $state(undefined);

  // Reset local state when pendingChoice changes
  $effect(() => {
    // Read pendingChoice to track it
    const _pc = pendingChoice;
    selectedMaterialIds = [];
    selectedDestroyIds = [];
    selectedMixColors = [];
    selectedGarmentId = undefined;
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
    const cardNames = selectedMaterialIds.map(id => {
      const c = currentPlayer?.drawnCards.find(c => c.instanceId === id);
      return c && 'name' in c.card ? c.card.name : 'a card';
    });
    onLog(`${currentPlayer?.name} stored materials from ${cardNames.join(', ')}`);
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
    const cardNames = selectedDestroyIds.map(id => {
      const c = currentPlayer?.drawnCards.find(c => c.instanceId === id);
      return c && 'name' in c.card ? c.card.name : 'a card';
    });
    onLog(`${currentPlayer?.name} destroyed ${cardNames.join(', ')} from drawn cards`);
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
        const result = mixResult(first, color);
        onLog(`${currentPlayer.name} mixed ${first} + ${color} to make ${result}`);
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
    onLog(`${currentPlayer?.name} skipped remaining mixes`);
    skipMix(gameState);
    onResolved();
  }

  // -- Choose Garment --
  function toggleGarmentSelect(garmentInstanceId: number) {
    selectedGarmentId = selectedGarmentId === garmentInstanceId ? undefined : garmentInstanceId;
  }

  function confirmGarment() {
    if (selectedGarmentId === undefined) return;
    const garment = gameState.garmentDisplay.find(g => g.instanceId === selectedGarmentId);
    onLog(`${currentPlayer?.name} completed a ${garment?.card.stars ?? '?'}-star garment`);
    resolveSelectGarment(gameState, selectedGarmentId);
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
        >
          Confirm Destroy ({selectedDestroyIds.length} selected)
        </button>
      </div>

    {:else if pendingChoice.type === 'chooseMix'}
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
