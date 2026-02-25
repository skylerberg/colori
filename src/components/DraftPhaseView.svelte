<script lang="ts">
  import type { GameState } from '../data/types';
  import type { ColoriChoice } from '../data/types';
  import { confirmPass } from '../engine/wasmEngine';
  import CardList from './CardList.svelte';
  import OpponentBoardPanel from './OpponentBoardPanel.svelte';
  import PassScreen from './PassScreen.svelte';

  let { gameState, onAction, onGameUpdated, playerIndex, selectable }: {
    gameState: GameState;
    onAction: (choice: ColoriChoice) => void;
    onGameUpdated?: () => void;
    playerIndex?: number;
    selectable?: boolean;
  } = $props();

  let draftState = $derived(
    gameState.phase.type === 'draft' ? gameState.phase.draftState : null
  );

  let viewingPlayerIndex = $derived(
    playerIndex !== undefined ? playerIndex : (draftState?.currentPlayerIndex ?? 0)
  );

  let currentPlayer = $derived(
    draftState ? gameState.players[viewingPlayerIndex] : null
  );

  let currentHand = $derived(
    draftState ? draftState.hands[viewingPlayerIndex] : []
  );

  let directionLabel = $derived(
    draftState ? (draftState.direction === 1 ? 'Left >>' : '<< Right') : ''
  );

  function handlePick(cardInstanceId: number) {
    onAction({ type: 'draftPick', cardInstanceId });
  }

  function handlePassReady() {
    confirmPass(gameState);
    onGameUpdated?.();
  }
</script>

{#if draftState}
  {#if draftState.waitingForPass && playerIndex === undefined}
    <PassScreen
      playerName={gameState.playerNames[draftState.currentPlayerIndex]}
      onReady={handlePassReady}
    />
  {:else}
    <div class="draft-phase">
      <div class="draft-header">
        <h2>Draft Phase - Round {gameState.round}</h2>
        <div class="draft-info">
          <span class="pick-number">Pick {draftState.pickNumber + 1} of 4</span>
          <span class="direction">Passing: {directionLabel}</span>
        </div>
      </div>

      <div class="current-player-section">
        <h3>{playerIndex !== undefined ? 'Pick a card' : `${gameState.playerNames[viewingPlayerIndex]}'s Turn - Pick a card`}</h3>
        <CardList
          cards={currentHand}
          selectable={selectable !== false}
          onCardClick={handlePick}
        />
      </div>

      <div class="section">
        <h3>Drafted Cards</h3>
        <CardList cards={currentPlayer?.draftedCards ?? []} />
      </div>

      <div class="section">
        <h3>Workshop</h3>
        <CardList cards={currentPlayer?.workshopCards ?? []} />
      </div>

      <div class="opponents-section">
        <h3>Other Players</h3>
        <div class="opponents-list">
          {#each gameState.players as player, i}
            {#if i !== viewingPlayerIndex}
              <OpponentBoardPanel {player} playerName={gameState.playerNames[i]} />
            {/if}
          {/each}
        </div>
      </div>
    </div>
  {/if}
{/if}

<style>
  .draft-phase {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .draft-header {
    text-align: center;
  }

  h2 {
    color: #4a3728;
    font-size: 1.3rem;
    margin-bottom: 4px;
  }

  .draft-info {
    display: flex;
    gap: 1.5rem;
    justify-content: center;
    font-size: 0.85rem;
    color: #666;
  }

  .pick-number {
    font-weight: 600;
    color: #2a6bcf;
  }

  .direction {
    font-style: italic;
  }

  .current-player-section {
    border: 2px solid #2a6bcf;
    border-radius: 10px;
    padding: 12px;
    background: #f8faff;
  }

  .current-player-section h3 {
    font-size: 1rem;
    color: #2a6bcf;
    margin-bottom: 8px;
    text-align: left;
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
