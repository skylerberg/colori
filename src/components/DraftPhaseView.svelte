<script lang="ts">
  import type { GameState, Choice } from '../data/types';
  import { getAnyCardData } from '../data/cards';
  import { formatAbility } from '../data/abilities';
  import CardList from './CardList.svelte';

  let { gameState, onAction, playerIndex, selectable = true }: {
    gameState: GameState;
    onAction: (choice: Choice) => void;
    playerIndex: number;
    selectable?: boolean;
  } = $props();

  let draftState = $derived(gameState.phase.type === 'draft' ? gameState.phase.draftState : null);
  let hand = $derived(draftState ? draftState.hands[playerIndex] : []);

  function handlePick(cardInstanceId: number) {
    const picked = hand.find(c => c.instanceId === cardInstanceId);
    if (picked) onAction({ type: 'draftPick', card: picked.card });
  }

  let lastCard = $derived(hand.length === 1);
</script>

{#if draftState}
  <div class="draft-phase">
    <div class="panel-heading">
      {selectable ? `Draft · pick ${draftState.pickNumber + 1} of 3` : 'Draft'}
    </div>
    {#if selectable}
      <p class="panel-text">
        {#if lastCard}
          The last card passed to you is yours.
        {:else}
          Keep one card; the rest pass on to the next player. Its banner is what destroying it does.
        {/if}
      </p>
    {/if}
    <div class="hand" class:waiting={!selectable}>
      <CardList cards={hand} selectable={selectable} onCardClick={handlePick} />
    </div>
    {#if selectable && hand.length > 0}
      <ul class="hand-abilities">
        {#each hand as ci (ci.instanceId)}
          {@const data = getAnyCardData(ci.card)}
          <li><strong>{data.name}</strong> — destroy: {formatAbility(data)}</li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .draft-phase {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .hand {
    container-type: inline-size;
  }

  .hand :global(.card-list) {
    /* Four cards across the panel, no larger than they need to be. */
    --card-height: min(240px, calc((100cqw - 3 * 8px) / 4 * 87 / 57));
    gap: 8px;
    padding: 8px 0 4px;
    min-height: 0;
    overflow: visible;
  }

  .hand.waiting {
    opacity: 0.5;
  }

  .hand-abilities {
    list-style: none;
    font-family: var(--font-body);
    font-size: 0.95rem;
    color: rgba(245, 237, 224, 0.7);
  }

  .hand-abilities strong {
    color: #f3dc8a;
    font-weight: 600;
  }
</style>
