<script lang="ts">
  import type { GameState, Choice } from '../data/types';
  import { canSell } from '../engine/wasmEngine';
  import SellCardDisplay from './SellCardDisplay.svelte';

  let { gameState, onAction }: {
    gameState: GameState;
    onAction: (choice: Choice) => void;
  } = $props();

  function handleSellCardSelect(sellCardInstanceId: number) {
    const sellCardInstance = gameState.sellCardDisplay.find(b => b.instanceId === sellCardInstanceId);
    if (!sellCardInstance) return;
    onAction({ type: 'selectSellCard', sellCard: sellCardInstance.card });
  }
</script>

<div class="prompt-section">
  <h3>Choose a Sell Card</h3>
  <div class="sell-card-side">
    <SellCardDisplay
      sellCards={gameState.sellCardDisplay.filter(g => canSell(gameState, g.instanceId))}
      selectable={true}
      onSelect={handleSellCardSelect}
    />
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

  .sell-card-side {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
</style>
