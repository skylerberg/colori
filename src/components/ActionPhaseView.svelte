<script lang="ts">
  import type { GameState } from '../data/types';
  import { destroyDraftedCard, endPlayerTurn } from '../engine/actionPhase';
  import CardList from './CardList.svelte';
  import AbilityPrompt from './AbilityPrompt.svelte';
  import OpponentBoardPanel from './OpponentBoardPanel.svelte';

  let { gameState, onGameUpdated, onLog }: {
    gameState: GameState;
    onGameUpdated: () => void;
    onLog: (entry: string) => void;
  } = $props();

  let actionState = $derived(
    gameState.phase.type === 'action' ? gameState.phase.actionState : null
  );

  let currentPlayer = $derived(
    actionState ? gameState.players[actionState.currentPlayerIndex] : null
  );

  let hasPendingChoice = $derived(actionState?.pendingChoice !== null);
  let hasAbilitiesQueued = $derived((actionState?.abilityQueue.length ?? 0) > 0);

  function handleDestroyDrafted(cardInstanceId: number) {
    if (hasPendingChoice) return;
    const card = currentPlayer?.draftedCards.find(c => c.instanceId === cardInstanceId);
    onLog(`${currentPlayer?.name} destroyed ${card && 'name' in card.card ? card.card.name : 'a card'} from drafted cards`);
    destroyDraftedCard(gameState, cardInstanceId);
    onGameUpdated();
  }

  function handleEndTurn() {
    onLog(`${currentPlayer?.name} ended their turn`);
    endPlayerTurn(gameState);
    onGameUpdated();
  }

  function handleAbilityResolved() {
    onGameUpdated();
  }
</script>

{#if actionState && currentPlayer}
  <div class="action-phase">
    <div class="action-header">
      <h2>Action Phase - {currentPlayer.name}'s Turn</h2>
      <div class="queue-status">
        {#if hasAbilitiesQueued}
          <span class="queue-info">Abilities queued: {actionState.abilityQueue.length}</span>
        {/if}
        {#if hasPendingChoice}
          <span class="pending-info">Awaiting your choice...</span>
        {/if}
      </div>
    </div>

    {#if hasPendingChoice}
      <AbilityPrompt {gameState} onResolved={handleAbilityResolved} {onLog} />
    {/if}

    <div class="sections">
      <div class="section">
        <h3>Drafted Cards <span class="hint">(click to destroy and activate ability)</span></h3>
        <CardList
          cards={currentPlayer.draftedCards}
          selectable={!hasPendingChoice}
          onCardClick={handleDestroyDrafted}
        />
      </div>

      <div class="section">
        <h3>Drawn Cards</h3>
        <CardList cards={currentPlayer.drawnCards} />
      </div>

    </div>

    <div class="opponents-section">
      <h3>Other Players</h3>
      <div class="opponents-list">
        {#each gameState.players as player, i}
          {#if i !== actionState.currentPlayerIndex}
            <OpponentBoardPanel {player} />
          {/if}
        {/each}
      </div>
    </div>

    <div class="action-footer">
      <button
        class="end-turn-btn"
        onclick={handleEndTurn}
        disabled={hasPendingChoice}
      >
        End Turn
      </button>
    </div>
  </div>
{/if}

<style>
  .action-phase {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .action-header {
    text-align: center;
  }

  h2 {
    color: #4a3728;
    font-size: 1.3rem;
    margin-bottom: 4px;
  }

  .queue-status {
    display: flex;
    gap: 1rem;
    justify-content: center;
    font-size: 0.8rem;
  }

  .queue-info {
    color: #d4a017;
    font-weight: 600;
  }

  .pending-info {
    color: #e63946;
    font-weight: 600;
  }

  .sections {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .section {
    border: 1px solid #ddd;
    border-radius: 8px;
    padding: 10px 12px;
    background: #fff;
    text-align: left;
  }

  .section h3 {
    font-size: 0.85rem;
    color: #4a3728;
    margin-bottom: 6px;
  }

  .hint {
    font-size: 0.7rem;
    color: #999;
    font-weight: 400;
  }

  .action-footer {
    display: flex;
    justify-content: center;
    padding-top: 8px;
  }

  .end-turn-btn {
    padding: 10px 32px;
    font-size: 1.05rem;
    font-weight: 700;
    background: #c0392b;
    color: #fff;
    border: none;
    border-radius: 8px;
  }

  .end-turn-btn:hover:not(:disabled) {
    background: #a93226;
  }

  .end-turn-btn:disabled {
    background: #ccc;
    cursor: not-allowed;
  }

  .opponents-section {
    border-top: 2px solid #e0e0e0;
    padding-top: 1rem;
  }

  .opponents-section h3 {
    font-size: 0.85rem;
    color: #888;
    margin-bottom: 8px;
  }

  .opponents-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
</style>
