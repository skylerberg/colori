<script lang="ts">
  import type { GameState, Choice, Ability } from '../data/types';
  import AbilityPrompt from './AbilityPrompt.svelte';
  import OpponentBoardPanel from './OpponentBoardPanel.svelte';

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
    <div class="action-header">
      <h2>Action Phase - {gameState.playerNames[actionState.currentPlayerIndex]}'s Turn</h2>
      <div class="queue-status">
        {#if hasAbilitiesQueued}
          <span class="queue-info">Abilities queued: {actionState.abilityStack.length}</span>
        {/if}
        {#if hasPendingChoice}
          <span class="pending-info">Awaiting your choice...</span>
        {/if}
      </div>
    </div>

    {#if hasPendingChoice && !workshopPendingChoice}
      <AbilityPrompt {gameState} {onAction} />
    {/if}

    <div class="opponents-section">
      <h3>Other Players</h3>
      <div class="opponents-list">
        {#each gameState.players as player, i}
          {#if i !== actionState.currentPlayerIndex}
            <OpponentBoardPanel {player} playerName={gameState.playerNames[i]} />
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
    gap: 0.5rem;
  }

  .action-header {
    text-align: center;
  }

  h2 {
    font-family: var(--font-display, 'Cinzel', serif);
    color: var(--text-primary, #2c1e12);
    font-size: 1.1rem;
    margin-bottom: 2px;
  }

  .queue-status {
    display: flex;
    gap: 1rem;
    justify-content: center;
    font-size: 0.8rem;
  }

  .queue-info {
    color: var(--accent-gold, #c9a84c);
    font-weight: 600;
  }

  .pending-info {
    color: var(--accent-crimson, #8b2020);
    font-weight: 600;
  }

  .action-footer {
    display: flex;
    justify-content: center;
    gap: 12px;
    padding-top: 4px;
  }

  .undo-btn {
    padding: 8px 20px;
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 0.95rem;
    font-weight: 600;
    letter-spacing: 1px;
    background: var(--bg-panel, #ebe3d3);
    color: var(--text-secondary, #6b5744);
    border: 2px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 8px;
  }

  .undo-btn:hover:not(:disabled) {
    background: #e0d6c3;
  }

  .undo-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .end-turn-btn {
    padding: 8px 28px;
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 0.95rem;
    font-weight: 600;
    letter-spacing: 1px;
    background: var(--accent-crimson, #8b2020);
    color: var(--text-on-dark, #f5ede0);
    border: none;
    border-radius: 8px;
  }

  .end-turn-btn:hover:not(:disabled) {
    background: #6b1818;
  }

  .end-turn-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .opponents-section {
    border-top: 2px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    padding-top: 0.5rem;
  }

  .opponents-section h3 {
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 0.8rem;
    color: var(--text-tertiary, #9a8775);
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 1px;
  }

  .opponents-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
</style>
