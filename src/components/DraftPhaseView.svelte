<script lang="ts">
  import type { GameState, Choice } from '../data/types';
  import CardList from './CardList.svelte';

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
      <h2 class="phase-title">Draft - Round {gameState.round}, Pick {draftState.pickNumber + 1} of 4</h2>

      <div class="draft-cards">
        <h3 class="section-label">{playerIndex !== undefined ? 'Pick a card' : `${gameState.playerNames[viewingPlayerIndex]}'s Turn - Pick a card`}</h3>
        <CardList
          cards={currentHand}
          selectable={selectable !== false}
          onCardClick={handlePick}
        />
      </div>
    </div>
{/if}

<style>
  .draft-phase {
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

  .section-label {
    font-family: 'Cinzel', serif;
    font-size: 0.85rem;
    color: #c9a84c;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    margin-bottom: 4px;
    text-align: left;
  }

  .draft-cards {
    padding: 4px;
  }
</style>
