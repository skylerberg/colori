<script lang="ts">
  import type { Choice, PlayerState } from '../data/types';
  import { canSell } from '../engine/wasmEngine';
  import SellCardDisplay from './SellCardDisplay.svelte';

  let { player, onAction }: {
    player: PlayerState;
    onAction: (choice: Choice) => void;
  } = $props();

  function handleSellCardSelect(sellCardInstanceId: number) {
    const sellCardInstance = player.buyers.find(b => b.instanceId === sellCardInstanceId);
    if (!sellCardInstance) return;
    onAction({ type: 'selectSellCard', sellCard: sellCardInstance.card });
  }
</script>

<div class="prompt-section">
  <h3>Choose a Buyer</h3>
  {#if player.boats > 0 || player.wildPrimaries > 0}
    <p class="hint">A boat covers a material you lack, and a wild primary covers a primary you lack. What a buyer asks for is spent first.</p>
  {/if}
  <div class="sell-card-side">
    <SellCardDisplay
      sellCards={player.buyers.filter(g => canSell(player, g.instanceId))}
      title="Your Buyers"
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

  .hint {
    font-size: 0.8rem;
    color: var(--text-tertiary, #9a8775);
    font-style: italic;
    text-align: left;
  }

  .sell-card-side {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
</style>
