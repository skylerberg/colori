<script lang="ts">
  import type { GameState } from '../data/types';
  import type { ColoriChoice } from '../data/types';
  import CardList from './CardList.svelte';

  let { gameState, onAction }: {
    gameState: GameState;
    onAction: (choice: ColoriChoice) => void;
  } = $props();

  let cleanupState = $derived(
    gameState.phase.type === 'cleanup' ? gameState.phase.cleanupState : null
  );

  let currentPlayer = $derived(
    cleanupState ? gameState.players[cleanupState.currentPlayerIndex] : null
  );

  // All cards start unselected (discarded by default)
  let selectedIds: number[] = $state([]);

  $effect(() => {
    if (currentPlayer) {
      selectedIds = [];
    }
  });

  function toggleCard(instanceId: number) {
    const idx = selectedIds.indexOf(instanceId);
    if (idx >= 0) {
      selectedIds = selectedIds.filter(id => id !== instanceId);
    } else {
      selectedIds = [...selectedIds, instanceId];
    }
  }

  function confirmKeep() {
    onAction({ type: 'keepWorkshopCards', cardInstanceIds: selectedIds });
  }

  function keepAll() {
    if (currentPlayer) {
      selectedIds = currentPlayer.workshopCards.map(c => c.instanceId);
    }
  }

  function discardAll() {
    selectedIds = [];
  }
</script>

{#if cleanupState && currentPlayer}
  <div class="cleanup-phase">
    <div class="cleanup-header">
      <h2>Cleanup Phase - {gameState.playerNames[cleanupState.currentPlayerIndex]}</h2>
      <p class="hint">Select workshop cards to keep for next round (unselected cards will be discarded)</p>
    </div>

    <div class="section" class:active-choice={true}>
      <h3>Workshop Cards ({selectedIds.length} of {currentPlayer.workshopCards.length} kept)</h3>
      <CardList
        cards={currentPlayer.workshopCards}
        selectable={true}
        selectedIds={selectedIds}
        onCardClick={toggleCard}
      />
      <div class="button-row">
        <button class="select-btn" onclick={keepAll}>Select All</button>
        <button class="select-btn deselect" onclick={discardAll}>Deselect All</button>
      </div>
    </div>

    <div class="cleanup-footer">
      <button class="confirm-btn" onclick={confirmKeep}>
        Confirm ({selectedIds.length} kept, {currentPlayer.workshopCards.length - selectedIds.length} discarded)
      </button>
    </div>
  </div>
{/if}

<style>
  .cleanup-phase {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .cleanup-header {
    text-align: center;
  }

  h2 {
    color: #4a3728;
    font-size: 1.3rem;
    margin-bottom: 4px;
  }

  .hint {
    font-size: 0.8rem;
    color: #999;
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

  .active-choice {
    border-color: #d4a017;
    border-width: 2px;
    background: #fffef0;
  }

  .button-row {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }

  .select-btn {
    padding: 6px 14px;
    font-size: 0.8rem;
    font-weight: 600;
    background: #2a6bcf;
    color: #fff;
    border: none;
    border-radius: 6px;
    cursor: pointer;
  }

  .select-btn:hover {
    background: #1e56a8;
  }

  .select-btn.deselect {
    background: #888;
  }

  .select-btn.deselect:hover {
    background: #666;
  }

  .cleanup-footer {
    display: flex;
    justify-content: center;
    padding-top: 8px;
  }

  .confirm-btn {
    padding: 10px 32px;
    font-size: 1.05rem;
    font-weight: 700;
    background: #2a6bcf;
    color: #fff;
    border: none;
    border-radius: 8px;
    cursor: pointer;
  }

  .confirm-btn:hover {
    background: #1e56a8;
  }
</style>
