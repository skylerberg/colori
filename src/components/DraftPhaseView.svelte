<script lang="ts">
  import type { GameState } from '../data/types';
  import { playerPick, confirmPass } from '../engine/draftPhase';
  import CardList from './CardList.svelte';
  import GarmentDisplay from './GarmentDisplay.svelte';
  import PassScreen from './PassScreen.svelte';

  let { gameState, onGameUpdated }: {
    gameState: GameState;
    onGameUpdated: () => void;
  } = $props();

  let draftState = $derived(
    gameState.phase.type === 'draft' ? gameState.phase.draftState : null
  );

  let currentPlayer = $derived(
    draftState ? gameState.players[draftState.currentPlayerIndex] : null
  );

  let currentHand = $derived(
    draftState ? draftState.hands[draftState.currentPlayerIndex] : []
  );

  let directionLabel = $derived(
    draftState ? (draftState.direction === 1 ? 'Left >>' : '<< Right') : ''
  );

  function handlePick(cardInstanceId: number) {
    playerPick(gameState, cardInstanceId);
    onGameUpdated();
  }

  function handlePassReady() {
    confirmPass(gameState);
    onGameUpdated();
  }
</script>

{#if draftState}
  {#if draftState.waitingForPass}
    <PassScreen
      playerName={gameState.players[draftState.currentPlayerIndex].name}
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
        <h3>{currentPlayer?.name}'s Turn - Pick a card</h3>
        <CardList
          cards={currentHand}
          selectable={true}
          onCardClick={handlePick}
        />
      </div>

      <div class="section">
        <h3>Drawn Cards</h3>
        <CardList cards={currentPlayer?.drawnCards ?? []} />
      </div>

      <div class="section">
        <GarmentDisplay garments={gameState.garmentDisplay} />
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
</style>
