<script lang="ts">
  import type { GameState } from '../data/types';
  import type { ColoriChoice } from '../data/types';
  import { getCardData } from '../data/cards';
  import CardList from './CardList.svelte';
  import AbilityPrompt from './AbilityPrompt.svelte';
  import OpponentBoardPanel from './OpponentBoardPanel.svelte';

  let { gameState, onAction, onUndo, undoAvailable }: {
    gameState: GameState;
    onAction: (choice: ColoriChoice) => void;
    onUndo: () => void;
    undoAvailable: boolean;
  } = $props();

  let actionState = $derived(
    gameState.phase.type === 'action' ? gameState.phase.actionState : null
  );

  let currentPlayer = $derived(
    actionState ? gameState.players[actionState.currentPlayerIndex] : null
  );

  let pendingChoice = $derived(actionState?.pendingChoice ?? null);
  let hasPendingChoice = $derived(actionState?.pendingChoice !== null);
  let hasAbilitiesQueued = $derived((actionState?.abilityStack.length ?? 0) > 0);

  let drawnCardChoice = $derived(
    pendingChoice?.type === 'chooseCardsForWorkshop' || pendingChoice?.type === 'chooseCardsToDestroy'
      ? pendingChoice : null
  );

  let selectedWorkshopIds: number[] = $state([]);
  let selectedDestroyIds: number[] = $state([]);

  $effect(() => {
    const _pc = pendingChoice;
    selectedWorkshopIds = [];
    selectedDestroyIds = [];
  });

  function toggleWorkshopCard(instanceId: number) {
    if (!pendingChoice || pendingChoice.type !== 'chooseCardsForWorkshop') return;
    const card = currentPlayer?.workshopCards.find(c => c.instanceId === instanceId);
    if (!card) return;
    const isAction = getCardData(card.card).kind === 'action';

    if (isAction) {
      // Action cards: toggle, deselect all non-action
      if (selectedWorkshopIds.includes(instanceId)) {
        selectedWorkshopIds = [];
      } else {
        selectedWorkshopIds = [instanceId];
      }
    } else {
      // Non-action cards: deselect any action cards
      const currentNonAction = selectedWorkshopIds.filter(id => {
        const c = currentPlayer?.workshopCards.find(c => c.instanceId === id);
        return c && getCardData(c.card).kind !== 'action';
      });
      const idx = currentNonAction.indexOf(instanceId);
      if (idx >= 0) {
        selectedWorkshopIds = currentNonAction.filter(id => id !== instanceId);
      } else if (currentNonAction.length < pendingChoice.count) {
        selectedWorkshopIds = [...currentNonAction, instanceId];
      }
    }
  }

  function confirmWorkshop() {
    onAction({ type: 'workshop', cardInstanceIds: selectedWorkshopIds });
  }

  function handleSkipWorkshop() {
    onAction({ type: 'skipWorkshop' });
  }

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
    onAction({ type: 'destroyDrawnCards', cardInstanceIds: selectedDestroyIds });
  }

  function handleDestroyDrafted(cardInstanceId: number) {
    if (hasPendingChoice) return;
    onAction({ type: 'destroyDraftedCard', cardInstanceId });
  }

  function handleEndTurn() {
    onAction({ type: 'endTurn' });
  }
</script>

{#if actionState && currentPlayer}
  <div class="action-phase">
    <div class="action-header">
      <h2>Action Phase - {currentPlayer.name}'s Turn</h2>
      <div class="queue-status">
        {#if hasAbilitiesQueued}
          <span class="queue-info">Abilities queued: {actionState.abilityStack.length}</span>
        {/if}
        {#if hasPendingChoice}
          <span class="pending-info">Awaiting your choice...</span>
        {/if}
      </div>
    </div>

    {#if hasPendingChoice && !drawnCardChoice}
      <AbilityPrompt {gameState} {onAction} />
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

      <div class="section" class:active-choice={drawnCardChoice}>
        {#if pendingChoice?.type === 'chooseCardsForWorkshop'}
          <h3>Workshop — Select cards ({pendingChoice.count} available)</h3>
          <CardList
            cards={currentPlayer.workshopCards}
            selectable={true}
            selectedIds={selectedWorkshopIds}
            onCardClick={toggleWorkshopCard}
          />
          <button class="confirm-btn" onclick={confirmWorkshop}>
            Confirm Workshop ({selectedWorkshopIds.length} selected)
          </button>
          <button class="confirm-btn skip-btn" onclick={handleSkipWorkshop}>
            Skip Workshop
          </button>
        {:else if pendingChoice?.type === 'chooseCardsToDestroy'}
          <h3>Workshop — Select up to {pendingChoice.count} card(s) to destroy</h3>
          <CardList
            cards={currentPlayer.workshopCards}
            selectable={true}
            selectedIds={selectedDestroyIds}
            onCardClick={toggleDestroyCard}
          />
          <button class="confirm-btn" onclick={confirmDestroy}>
            Confirm Destroy ({selectedDestroyIds.length} selected)
          </button>
        {:else}
          <h3>Workshop</h3>
          <CardList cards={currentPlayer.workshopCards} />
        {/if}
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
      <button class="undo-btn" onclick={onUndo} disabled={!undoAvailable}>
        Undo
      </button>
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

  .active-choice {
    border-color: #d4a017;
    border-width: 2px;
    background: #fffef0;
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

  .confirm-btn:hover {
    background: #1e56a8;
  }

  .skip-btn {
    background: #888;
  }

  .skip-btn:hover {
    background: #666;
  }

  .action-footer {
    display: flex;
    justify-content: center;
    gap: 12px;
    padding-top: 8px;
  }

  .undo-btn {
    padding: 10px 24px;
    font-size: 1.05rem;
    font-weight: 700;
    background: #888;
    color: #fff;
    border: none;
    border-radius: 8px;
  }

  .undo-btn:hover:not(:disabled) {
    background: #666;
  }

  .undo-btn:disabled {
    background: #ccc;
    cursor: not-allowed;
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
