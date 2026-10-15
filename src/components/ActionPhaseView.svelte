<script lang="ts">
  import type { GameState, Choice, Ability } from '../data/types';
  import { canSell } from '../engine/wasmEngine';
  import { canMixAny } from '../data/colors';
  import { orderByDraftOrder } from '../gameUtils';
  import CardList from './CardList.svelte';
  import AbilityPrompt from './AbilityPrompt.svelte';

  let {
    gameState,
    playerIndex,
    onAction,
    onUndo,
    undoAvailable,
    draftCardOrder,
  }: {
    gameState: GameState;
    playerIndex: number;
    onAction: (choice: Choice) => void;
    onUndo: () => void;
    undoAvailable: boolean;
    draftCardOrder?: number[][];
  } = $props();

  let turn = $derived(
    gameState.phase.type === 'action' ? gameState.phase.actionState.turns[playerIndex] : null
  );

  let currentPlayer = $derived(turn ? gameState.players[playerIndex] : null);

  let topAbility: Ability | null = $derived(
    turn && turn.abilityStack.length > 0
      ? turn.abilityStack[turn.abilityStack.length - 1]
      : null
  );
  let hasPendingChoice = $derived(topAbility !== null);

  let workshopDisplayCards = $derived(
    currentPlayer ? [...currentPlayer.workshopCards, ...currentPlayer.workshoppedCards] : []
  );
  let workshoppedIds = $derived(
    currentPlayer ? currentPlayer.workshoppedCards.map(c => c.instanceId) : []
  );
  let hasAbilitiesQueued = $derived((turn?.abilityStack.length ?? 0) > 0);

  // Cards moved in from the workshop are not in the draft order, so
  // `orderByDraftOrder` appends them after the ones that were drafted.
  let draftedDisplayCards = $derived.by(() => {
    if (!currentPlayer) return [];
    return draftCardOrder
      ? orderByDraftOrder(currentPlayer.draftedCards, draftCardOrder[playerIndex])
      : currentPlayer.draftedCards;
  });

  let workshopPendingChoice = $derived(
    topAbility?.type === 'workshop' || topAbility?.type === 'moveToDraftPool'
      ? topAbility : null
  );

  let selectedWorkshopIds: number[] = $state([]);

  $effect(() => {
    topAbility;
    selectedWorkshopIds = [];
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

  function handleMoveToDraftPool(instanceId: number) {
    if (!topAbility || topAbility.type !== 'moveToDraftPool' || !currentPlayer) return;
    const ci =
      currentPlayer.workshopCards.find(c => c.instanceId === instanceId)
      ?? currentPlayer.workshoppedCards.find(c => c.instanceId === instanceId);
    if (!ci) return;
    onAction({ type: 'moveToDraftPool', card: ci.card });
  }

  // The move is mandatory when there is anything to move, so this only ever
  // resolves an ability that has nothing to act on.
  function handleNothingToMove() {
    onAction({ type: 'moveToDraftPool', card: null });
  }

  function handleDestroyDrafted(cardInstanceId: number) {
    if (!currentPlayer) return;
    if (hasPendingChoice) return;
    const ci = currentPlayer.draftedCards.find(c => c.instanceId === cardInstanceId);
    if (!ci) return;
    onAction({ type: 'destroyDraftedCard', card: ci.card });
  }

  function handleEndTurn() {
    onAction({ type: 'endTurn' });
  }

  // Mirrors the engine: a purchase is offered only when it can act, and only
  // between destroys. See `action_phase::can_buy_with_ducat`.
  let canBuy = $derived.by(() => {
    const none = { workshop: false, mixColors: false, sell: false };
    if (!currentPlayer || hasPendingChoice || currentPlayer.ducats < 1) return none;
    return {
      workshop: currentPlayer.workshopCards.length > 0,
      mixColors: canMixAny(currentPlayer.colorWheel, currentPlayer.wildPrimaries),
      sell: currentPlayer.buyers.some(b => canSell(currentPlayer!, b.instanceId)),
    };
  });

  let anyPurchase = $derived(canBuy.workshop || canBuy.mixColors || canBuy.sell);

</script>

{#if turn && currentPlayer}
  <div class="action-phase">
    <h2 class="phase-title">Action Phase - {gameState.playerNames[playerIndex]}'s Turn</h2>
    <div class="queue-status">
      {#if hasAbilitiesQueued}
        <span class="queue-info">Abilities queued: {turn.abilityStack.length}</span>
      {/if}
      {#if hasPendingChoice}
        <span class="pending-info">Awaiting your choice...</span>
      {/if}
    </div>

    {#if hasPendingChoice && !workshopPendingChoice}
      <AbilityPrompt {gameState} {playerIndex} {onAction} />
    {/if}

    <div class="sections">
      <div class="section">
        <h3>Drafted Cards <span class="hint">(click to destroy and activate ability)</span></h3>
        <CardList
          cards={draftedDisplayCards}
          selectable={!hasPendingChoice}
          onCardClick={handleDestroyDrafted}
        />
      </div>

      <div class="section" class:active-choice={workshopPendingChoice}>
        {#if topAbility?.type === 'workshop'}
          <h3>Workshop — Select cards ({topAbility.count} available)</h3>
          <CardList
            cards={workshopDisplayCards}
            selectable={true}
            selectedIds={selectedWorkshopIds}
            rotatedIds={workshoppedIds}
            onCardClick={toggleWorkshopCard}
          />
          <div class="workshop-actions">
            <button class="confirm-btn" onclick={confirmWorkshop}>
              Confirm Workshop ({selectedWorkshopIds.length} selected)
            </button>
            {#if selectedWorkshopIds.length === 0}
              <button class="confirm-btn skip-btn" onclick={handleSkipWorkshop}>
                Skip Workshop
              </button>
            {/if}
          </div>
        {:else if topAbility?.type === 'moveToDraftPool'}
          <h3>Workshop — Click a card to move to draft pool</h3>
          <CardList
            cards={workshopDisplayCards}
            selectable={true}
            rotatedSelectable={true}
            rotatedIds={workshoppedIds}
            onCardClick={handleMoveToDraftPool}
          />
          {#if workshopDisplayCards.length === 0}
            <button class="confirm-btn skip-btn" onclick={handleNothingToMove}>
              Nothing to move
            </button>
          {/if}
        {:else}
          <h3>Workshop</h3>
          <CardList cards={workshopDisplayCards} rotatedIds={workshoppedIds} />
        {/if}
      </div>

    </div>

    {#if anyPurchase}
      <div class="ducat-shop">
        <span class="ducat-label">Spend a ducat ({currentPlayer.ducats}):</span>
        {#if canBuy.workshop}
          <button class="ducat-btn" onclick={() => onAction({ type: 'spendDucat', purchase: 'workshop' })}>
            Workshop &times;1
          </button>
        {/if}
        {#if canBuy.mixColors}
          <button class="ducat-btn" onclick={() => onAction({ type: 'spendDucat', purchase: 'mixColors' })}>
            Mix &times;1
          </button>
        {/if}
        {#if canBuy.sell}
          <button class="ducat-btn" onclick={() => onAction({ type: 'spendDucat', purchase: 'sell' })}>
            Sell
          </button>
        {/if}
      </div>
    {/if}

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
  .ducat-shop {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.4rem;
    justify-content: center;
    font-size: 0.8rem;
  }

  .ducat-label {
    color: #c9a84c;
    font-weight: 600;
  }

  .ducat-btn {
    padding: 0.2rem 0.55rem;
    background: rgba(201, 168, 76, 0.18);
    border: 1px solid rgba(201, 168, 76, 0.6);
    border-radius: 4px;
    color: #e8d9a8;
    cursor: pointer;
    font-size: 0.78rem;
  }

  .ducat-btn:hover {
    background: rgba(201, 168, 76, 0.32);
  }

  .action-phase {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0 0.25rem;
  }

  .phase-title {
    font-family: 'Cinzel', serif;
    color: #c9a84c;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    text-align: center;
    margin: 0;
  }

  .queue-status {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
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
    gap: 0.5rem;
  }

  .section {
    border: 1px solid rgba(201, 168, 76, 0.4);
    border-radius: 8px;
    padding: 8px;
    background: rgba(20, 15, 10, 0.6);
    text-align: left;
  }

  .section h3 {
    font-family: 'Cinzel', serif;
    font-size: 0.75rem;
    color: #c9a84c;
    margin-bottom: 6px;
  }

  .hint {
    font-size: 0.65rem;
    color: rgba(245, 237, 224, 0.4);
    font-weight: 400;
  }

  .active-choice {
    border-color: #c9a84c;
    border-width: 2px;
    background: rgba(201, 168, 76, 0.1);
  }

  .confirm-btn {
    padding: 10px 16px;
    font-family: 'Cinzel', serif;
    font-weight: 600;
    font-size: 0.85rem;
    background: rgba(42, 107, 207, 0.8);
    color: #f5ede0;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    margin-top: 6px;
    margin-right: 0;
    min-height: 44px;
    width: 100%;
  }

  .confirm-btn:hover:not(:disabled) {
    background: rgba(30, 86, 168, 0.9);
  }

  .confirm-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .skip-btn {
    background: rgba(100, 100, 100, 0.6);
  }

  .skip-btn:hover {
    background: rgba(80, 80, 80, 0.8);
  }

  .action-footer {
    display: flex;
    gap: 8px;
    padding-top: 4px;
  }

  .undo-btn {
    padding: 10px 16px;
    font-family: 'Cinzel', serif;
    font-size: 0.85rem;
    font-weight: 600;
    letter-spacing: 1px;
    background: rgba(20, 15, 10, 0.6);
    color: #c9a84c;
    border: 1px solid rgba(201, 168, 76, 0.4);
    border-radius: 8px;
    cursor: pointer;
    min-height: 44px;
    flex: 1;
  }

  .undo-btn:hover:not(:disabled) {
    background: rgba(40, 30, 20, 0.8);
  }

  .undo-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .end-turn-btn {
    padding: 10px 16px;
    font-family: 'Cinzel', serif;
    font-size: 0.85rem;
    font-weight: 600;
    letter-spacing: 1px;
    background: #8b2020;
    color: #f5ede0;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    min-height: 44px;
    flex: 1;
  }

  .end-turn-btn:hover:not(:disabled) {
    background: #6b1818;
  }

  .end-turn-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .workshop-actions {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  /* ── Responsive: sm (640px) ── */
  @media (min-width: 640px) {
    .action-phase {
      padding: 0;
    }

    .phase-title {
      font-size: 0.9rem;
      letter-spacing: 0.08em;
    }

    .section {
      padding: 10px 12px;
    }

    .section h3 {
      font-size: 0.8rem;
    }

    .hint {
      font-size: 0.7rem;
    }

    .confirm-btn {
      width: auto;
      padding: 8px 20px;
      margin-right: 6px;
      min-height: unset;
      font-size: inherit;
    }

    .action-footer {
      justify-content: center;
      gap: 12px;
    }

    .undo-btn,
    .end-turn-btn {
      flex: 0 1 auto;
      min-height: unset;
    }

    .undo-btn {
      padding: 8px 20px;
      font-size: 0.95rem;
    }

    .end-turn-btn {
      padding: 8px 28px;
      font-size: 0.95rem;
    }

    .workshop-actions {
      flex-direction: row;
      flex-wrap: wrap;
      align-items: center;
    }

  }

  /* ── Responsive: md (768px) ── */
  @media (min-width: 768px) {
    .phase-title {
      font-size: 1rem;
      letter-spacing: 0.1em;
    }

    .sections {
      gap: 0.75rem;
    }

    .section h3 {
      font-size: 0.85rem;
    }
  }
</style>
