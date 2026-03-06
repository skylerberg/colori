<script lang="ts">
  import type { GameState, Choice } from '../data/types';
  import CardList from './CardList.svelte';
  import OpponentBoardPanel from './OpponentBoardPanel.svelte';

  let { gameState, onAction, playerIndex, selectable }: {
    gameState: GameState;
    onAction: (choice: Choice) => void;
    playerIndex?: number;
    selectable?: boolean;
  } = $props();

  let draftState = $derived(
    gameState.phase.type === 'draft' ? gameState.phase.draftState : null
  );

  let viewingPlayerIndex = $derived(
    playerIndex !== undefined ? playerIndex : (draftState?.currentPlayerIndex ?? 0)
  );

  let currentHand = $derived(
    draftState ? draftState.hands[viewingPlayerIndex] : []
  );

  function handlePick(cardInstanceId: number) {
    const clickedCard = currentHand.find(c => c.instanceId === cardInstanceId);
    if (!clickedCard) return;
    onAction({ type: 'draftPick', card: clickedCard.card });
  }

</script>

{#if draftState}
    <div class="draft-phase">
      <div class="draft-header">
        <h2>Draft Phase - Round {gameState.round}</h2>
        <div class="draft-info">
          <span class="pick-number">Pick {draftState.pickNumber + 1} of 4</span>
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

<style>
  .draft-phase {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .draft-header {
    text-align: center;
  }

  h2 {
    font-family: var(--font-display, 'Cinzel', serif);
    color: var(--text-primary, #2c1e12);
    font-size: 1.1rem;
    margin-bottom: 2px;
  }

  .draft-info {
    display: flex;
    gap: 1rem;
    justify-content: center;
    font-size: 0.8rem;
    color: var(--text-secondary, #6b5744);
  }

  .pick-number {
    font-family: var(--font-display, 'Cinzel', serif);
    font-weight: 600;
    color: var(--accent-gold, #c9a84c);
  }

  .current-player-section {
    border: 2px solid var(--accent-gold, #c9a84c);
    border-radius: 10px;
    padding: 8px;
    background: rgba(201, 168, 76, 0.06);
  }

  .current-player-section h3 {
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 0.85rem;
    color: var(--accent-gold, #c9a84c);
    margin-bottom: 4px;
    text-align: left;
    text-transform: uppercase;
    letter-spacing: 1px;
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
