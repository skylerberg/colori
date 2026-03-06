<script lang="ts">
  import type { GameState, Choice, Ability } from '../data/types';
  import CardList from './CardList.svelte';
  import AbilityPrompt from './AbilityPrompt.svelte';

  let { gameState, onAction, onUndo, undoAvailable }: {
    gameState: GameState;
    onAction: (choice: Choice) => void;
    onUndo: () => void;
    undoAvailable: boolean;
  } = $props();

  let actionState = $derived(
    gameState.phase.type === 'action' ? gameState.phase.actionState : null
  );

  let currentPlayer = $derived(
    actionState ? gameState.players[actionState.currentPlayerIndex] : null
  );

  let topAbility: Ability | null = $derived(
    actionState && actionState.abilityStack.length > 0
      ? actionState.abilityStack[actionState.abilityStack.length - 1]
      : null
  );
  let hasPendingChoice = $derived(topAbility !== null);
  let hasAbilitiesQueued = $derived((actionState?.abilityStack.length ?? 0) > 0);

  let workshopPendingChoice = $derived(
    topAbility?.type === 'workshop' || topAbility?.type === 'destroyCards'
      ? topAbility : null
  );

  let selectedWorkshopIds: number[] = $state([]);
  let selectedDestroyIds: number[] = $state([]);

  $effect(() => {
    topAbility;
    selectedWorkshopIds = [];
    selectedDestroyIds = [];
  });

  function toggleWorkshopCard(instanceId: number) {
    if (!topAbility || topAbility.type !== 'workshop') return;
    const idx = selectedWorkshopIds.indexOf(instanceId);
    if (idx >= 0) {
      selectedWorkshopIds = selectedWorkshopIds.filter(id => id !== instanceId);
    } else if (selectedWorkshopIds.length < topAbility.count) {
      selectedWorkshopIds = [...selectedWorkshopIds, instanceId];
    }
  }

  function confirmWorkshop() {
    if (!currentPlayer) return;
    const cardTypes = selectedWorkshopIds.map(id => {
      const ci = currentPlayer!.workshopCards.find(c => c.instanceId === id);
      return ci!.card;
    });
    onAction({ type: 'workshop', cardTypes });
  }

  function handleSkipWorkshop() {
    onAction({ type: 'skipWorkshop' });
  }

  function toggleDestroyCard(instanceId: number) {
    if (!topAbility || topAbility.type !== 'destroyCards') return;
    const idx = selectedDestroyIds.indexOf(instanceId);
    if (idx >= 0) {
      selectedDestroyIds = selectedDestroyIds.filter(id => id !== instanceId);
    } else if (selectedDestroyIds.length < 1) {
      selectedDestroyIds = [...selectedDestroyIds, instanceId];
    }
  }

  function confirmDestroy() {
    if (!currentPlayer) return;
    const card = selectedDestroyIds.length > 0
      ? currentPlayer.workshopCards.find(c => c.instanceId === selectedDestroyIds[0])!.card
      : null;
    onAction({ type: 'destroyDrawnCards', card });
  }

  function handleDestroyDrafted(cardInstanceId: number) {
    if (hasPendingChoice || !currentPlayer) return;
    const ci = currentPlayer.draftedCards.find(c => c.instanceId === cardInstanceId);
    if (!ci) return;
    onAction({ type: 'destroyDraftedCard', card: ci.card });
  }

  function handleEndTurn() {
    onAction({ type: 'endTurn' });
  }
</script>

{#if actionState && currentPlayer}
  <div class="action-phase">
    <h2 class="phase-title">Action Phase - {gameState.playerNames[actionState.currentPlayerIndex]}'s Turn</h2>
    <div class="queue-status">
      {#if hasAbilitiesQueued}
        <span class="queue-info">Abilities queued: {actionState.abilityStack.length}</span>
      {/if}
      {#if hasPendingChoice}
        <span class="pending-info">Awaiting your choice...</span>
      {/if}
    </div>

    {#if hasPendingChoice && !workshopPendingChoice}
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

      <div class="section" class:active-choice={workshopPendingChoice}>
        {#if topAbility?.type === 'workshop'}
          <h3>Workshop — Select cards ({topAbility.count} available)</h3>
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
        {:else if topAbility?.type === 'destroyCards'}
          <h3>Workshop — Select a card to destroy</h3>
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
    gap: 0.5rem;
  }

  .phase-title {
    font-family: 'Cinzel', serif;
    color: #c9a84c;
    font-size: 1rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    text-align: center;
    margin: 0;
  }

  .queue-status {
    display: flex;
    gap: 1rem;
    justify-content: center;
    font-size: 0.8rem;
  }

  .queue-info {
    color: #c9a84c;
    font-weight: 600;
  }

  .pending-info {
    color: #8b2020;
    font-weight: 600;
  }

  .sections {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .section {
    border: 1px solid rgba(201, 168, 76, 0.4);
    border-radius: 8px;
    padding: 10px 12px;
    background: rgba(20, 15, 10, 0.6);
    text-align: left;
  }

  .section h3 {
    font-family: 'Cinzel', serif;
    font-size: 0.85rem;
    color: #c9a84c;
    margin-bottom: 6px;
  }

  .hint {
    font-size: 0.7rem;
    color: rgba(245, 237, 224, 0.4);
    font-weight: 400;
  }

  .active-choice {
    border-color: #c9a84c;
    border-width: 2px;
    background: rgba(201, 168, 76, 0.1);
  }

  .confirm-btn {
    padding: 8px 20px;
    font-family: 'Cinzel', serif;
    font-weight: 600;
    background: rgba(42, 107, 207, 0.8);
    color: #f5ede0;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    margin-top: 6px;
    margin-right: 6px;
  }

  .confirm-btn:hover {
    background: rgba(30, 86, 168, 0.9);
  }

  .skip-btn {
    background: rgba(100, 100, 100, 0.6);
  }

  .skip-btn:hover {
    background: rgba(80, 80, 80, 0.8);
  }

  .action-footer {
    display: flex;
    justify-content: center;
    gap: 12px;
    padding-top: 4px;
  }

  .undo-btn {
    padding: 8px 20px;
    font-family: 'Cinzel', serif;
    font-size: 0.95rem;
    font-weight: 600;
    letter-spacing: 1px;
    background: rgba(20, 15, 10, 0.6);
    color: #c9a84c;
    border: 1px solid rgba(201, 168, 76, 0.4);
    border-radius: 8px;
    cursor: pointer;
  }

  .undo-btn:hover:not(:disabled) {
    background: rgba(40, 30, 20, 0.8);
  }

  .undo-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .end-turn-btn {
    padding: 8px 28px;
    font-family: 'Cinzel', serif;
    font-size: 0.95rem;
    font-weight: 600;
    letter-spacing: 1px;
    background: #8b2020;
    color: #f5ede0;
    border: none;
    border-radius: 8px;
    cursor: pointer;
  }

  .end-turn-btn:hover:not(:disabled) {
    background: #6b1818;
  }

  .end-turn-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
