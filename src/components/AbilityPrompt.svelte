<script lang="ts">
  import type { GameState, Color, CardInstance } from '../data/types';
  import { resolveStoreColors, resolveMixColors, skipMix, resolveDestroyCards, resolveChooseGarment, resolveGarmentPayment } from '../engine/actionPhase';
  import { canMix } from '../data/colors';
  import { canPayCost } from '../engine/colorWheel';
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

  // Store colors state
  let selectedStoreIds: number[] = $state([]);

  // Destroy cards state
  let selectedDestroyIds: number[] = $state([]);

  // Mix colors state
  let selectedMixColors: Color[] = $state([]);

  // Garment payment state
  let selectedFabricId: number | null = $state(null);
  let paymentType: 'colorWheel' | 'dyeCard' | null = $state(null);
  let selectedDyeId: number | null = $state(null);

  // Reset local state when pendingChoice changes
  $effect(() => {
    // Read pendingChoice to track it
    const _pc = pendingChoice;
    selectedStoreIds = [];
    selectedDestroyIds = [];
    selectedMixColors = [];
    selectedFabricId = null;
    paymentType = null;
    selectedDyeId = null;
  });

  // -- Store Colors --
  function toggleStoreCard(instanceId: number) {
    if (!pendingChoice || pendingChoice.type !== 'chooseCardsForStore') return;
    const idx = selectedStoreIds.indexOf(instanceId);
    if (idx >= 0) {
      selectedStoreIds = selectedStoreIds.filter(id => id !== instanceId);
    } else if (selectedStoreIds.length < pendingChoice.count) {
      selectedStoreIds = [...selectedStoreIds, instanceId];
    }
  }

  function confirmStore() {
    if (selectedStoreIds.length === 0) return;
    resolveStoreColors(gameState, selectedStoreIds);
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
  let garmentForPayment = $derived(() => {
    if (!pendingChoice || pendingChoice.type !== 'chooseGarmentPayment') return null;
    return gameState.garmentDisplay.find(g => g.instanceId === pendingChoice.garmentInstanceId) ?? null;
  });

  let availableFabrics = $derived(
    currentPlayer && pendingChoice?.type === 'chooseGarmentPayment' && garmentForPayment()
      ? currentPlayer.drawnCards.filter(
          c => c.card.kind === 'fabric' && c.card.fabricType === garmentForPayment()!.card.requiredFabric
        )
      : []
  );

  let matchingDyes = $derived(
    currentPlayer && pendingChoice?.type === 'chooseGarmentPayment' && garmentForPayment()
      ? currentPlayer.drawnCards.filter(
          c => (c.card.kind === 'dye' || c.card.kind === 'basicDye') && c.card.name === garmentForPayment()!.card.matchingDyeName
        )
      : []
  );

  let canPayWithWheel = $derived(
    currentPlayer && pendingChoice?.type === 'chooseGarmentPayment' && garmentForPayment()
      ? canPayCost(currentPlayer.colorWheel, garmentForPayment()!.card.colorCost)
      : false
  );

  function selectFabric(id: number) {
    selectedFabricId = id;
  }

  function selectPaymentType(type: 'colorWheel' | 'dyeCard') {
    paymentType = type;
    if (type === 'colorWheel') {
      selectedDyeId = null;
    }
  }

  function selectDyeCard(id: number) {
    selectedDyeId = id;
    paymentType = 'dyeCard';
  }

  function confirmGarmentPayment() {
    if (selectedFabricId === null || paymentType === null) return;
    if (paymentType === 'dyeCard' && selectedDyeId === null) return;
    resolveGarmentPayment(
      gameState,
      selectedFabricId,
      paymentType,
      paymentType === 'dyeCard' ? selectedDyeId! : undefined
    );
    onResolved();
  }
</script>

{#if pendingChoice && currentPlayer}
  <div class="ability-prompt">
    {#if pendingChoice.type === 'chooseCardsForStore'}
      <div class="prompt-section">
        <h3>Store Colors: Select up to {pendingChoice.count} card(s) to store on your color wheel</h3>
        <CardList
          cards={currentPlayer.drawnCards}
          selectable={true}
          selectedIds={selectedStoreIds}
          onCardClick={toggleStoreCard}
        />
        <button
          class="confirm-btn"
          onclick={confirmStore}
          disabled={selectedStoreIds.length === 0}
        >
          Confirm Store ({selectedStoreIds.length} selected)
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

        {#if garmentForPayment()}
          <div class="payment-step">
            <h4>1. Select a {garmentForPayment()!.card.requiredFabric} fabric card:</h4>
            <CardList
              cards={availableFabrics}
              selectable={true}
              selectedIds={selectedFabricId !== null ? [selectedFabricId] : []}
              onCardClick={selectFabric}
            />
          </div>

          {#if selectedFabricId !== null}
            <div class="payment-step">
              <h4>2. Choose payment method:</h4>
              <div class="payment-options">
                {#if canPayWithWheel}
                  <button
                    class="payment-btn"
                    class:active={paymentType === 'colorWheel'}
                    onclick={() => selectPaymentType('colorWheel')}
                  >
                    Pay with Color Wheel
                  </button>
                {/if}
                {#if matchingDyes.length > 0}
                  <div class="dye-payment">
                    <p>Or pay with matching dye card:</p>
                    <CardList
                      cards={matchingDyes}
                      selectable={true}
                      selectedIds={selectedDyeId !== null ? [selectedDyeId] : []}
                      onCardClick={selectDyeCard}
                    />
                  </div>
                {/if}
              </div>
            </div>

            <button
              class="confirm-btn"
              onclick={confirmGarmentPayment}
              disabled={paymentType === null || (paymentType === 'dyeCard' && selectedDyeId === null)}
            >
              Confirm Payment
            </button>
          {/if}
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

  h4 {
    font-size: 0.85rem;
    color: #555;
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

  .payment-step {
    display: flex;
    flex-direction: column;
    gap: 6px;
    border-top: 1px solid #eee;
    padding-top: 10px;
  }

  .payment-options {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .payment-btn {
    padding: 8px 16px;
    border: 2px solid #ccc;
    border-radius: 6px;
    background: #fff;
    font-weight: 600;
    align-self: flex-start;
  }

  .payment-btn.active {
    border-color: #2a6bcf;
    background: #eef3ff;
    color: #2a6bcf;
  }

  .dye-payment p {
    font-size: 0.8rem;
    color: #666;
    text-align: left;
  }
</style>
