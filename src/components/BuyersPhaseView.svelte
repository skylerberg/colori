<script lang="ts">
  import type { GameState, Choice } from '../data/types';
  import SellCardDisplay from './SellCardDisplay.svelte';
  import CardList from './CardList.svelte';

  let { gameState, onAction }: {
    gameState: GameState;
    onAction: (choice: Choice) => void;
  } = $props();

  let buyersState = $derived(
    gameState.phase.type === 'buyers' ? gameState.phase.buyersState : null
  );
  let currentPlayer = $derived(
    buyersState ? gameState.players[buyersState.currentPlayerIndex] : null
  );
  let deckEmpty = $derived(gameState.sellCardDeck.length === 0);

  function claim(instanceId: number) {
    const instance = gameState.sellCardDisplay.find(c => c.instanceId === instanceId);
    if (!instance) return;
    onAction({ type: 'takeBuyer', sellCard: instance.card });
  }
</script>

{#if buyersState && currentPlayer}
  <div class="buyers-phase">
    <h2 class="phase-title">
      Buyers Phase - {gameState.playerNames[buyersState.currentPlayerIndex]}'s Pick
    </h2>
    <p class="hint">
      Claim a sell card into your buyers. Only cards in your own buyers can be sold to,
      and a claimed card is replaced in the display straight away.
    </p>

    <SellCardDisplay sellCards={gameState.sellCardDisplay} selectable={true} onSelect={claim} />

    <div class="deck-option">
      <button
        class="draw-btn"
        onclick={() => onAction({ type: 'drawBuyer' })}
        disabled={deckEmpty}
      >
        Take the top of the deck, unseen
      </button>
      <span class="deck-count">{gameState.sellCardDeck.length} left in the deck</span>
    </div>

    <div class="section">
      <h3>Your Buyers</h3>
      {#if currentPlayer.buyers.length > 0}
        <CardList cards={currentPlayer.buyers} />
      {:else}
        <div class="empty-text">None yet</div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .buyers-phase {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .phase-title {
    margin: 0;
    font-size: 1.1rem;
    color: #e8d9a8;
  }

  .hint {
    margin: 0;
    font-size: 0.85rem;
    color: rgba(232, 217, 168, 0.7);
  }

  .deck-option {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .draw-btn {
    padding: 8px 14px;
    background: rgba(201, 168, 76, 0.2);
    border: 1px solid rgba(201, 168, 76, 0.6);
    border-radius: 6px;
    color: #e8d9a8;
    cursor: pointer;
    font-size: 0.9rem;
  }

  .draw-btn:hover:not(:disabled) {
    background: rgba(201, 168, 76, 0.35);
  }

  .draw-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .deck-count {
    font-size: 0.8rem;
    color: rgba(232, 217, 168, 0.6);
  }

  .section h3 {
    margin: 0 0 6px;
    font-size: 0.95rem;
    color: #e8d9a8;
  }

  .empty-text {
    font-size: 0.85rem;
    color: rgba(232, 217, 168, 0.5);
  }
</style>
