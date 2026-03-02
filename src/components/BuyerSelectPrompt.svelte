<script lang="ts">
  import type { GameState, Choice } from '../data/types';
  import { canSell } from '../engine/wasmEngine';
  import BuyerDisplay from './BuyerDisplay.svelte';

  let { gameState, onAction }: {
    gameState: GameState;
    onAction: (choice: Choice) => void;
  } = $props();

  let selectedBuyerId: number | undefined = $state(undefined);

  // Reset when gameState changes
  $effect(() => {
    const _gs = gameState;
    selectedBuyerId = undefined;
  });

  function toggleBuyerSelect(buyerInstanceId: number) {
    selectedBuyerId = selectedBuyerId === buyerInstanceId ? undefined : buyerInstanceId;
  }

  function confirmBuyer() {
    if (selectedBuyerId === undefined) return;
    const buyerInstance = gameState.buyerDisplay.find(b => b.instanceId === selectedBuyerId);
    if (!buyerInstance) return;
    onAction({ type: 'selectBuyer', buyer: buyerInstance.card });
  }
</script>

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

<style>
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
</style>
