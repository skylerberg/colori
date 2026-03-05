<script lang="ts">
  import type { GameState, Choice, CardInstance, Card as CardType } from '../../../data/types';
  import Card3D from './Card3D.svelte';

  let { gameState, onAction }: {
    gameState: GameState;
    onAction?: (choice: Choice) => void;
  } = $props();

  let actionState = $derived(
    gameState.phase.type === 'action' ? gameState.phase.actionState : null
  );

  let currentPlayerIndex = $derived(actionState?.currentPlayerIndex ?? 0);
  let player = $derived(gameState.players[currentPlayerIndex]);
  let draftedCards = $derived(player?.draftedCards ?? []);
  let workshopCards = $derived(player?.workshopCards ?? []);
  let workshoppedCards = $derived(player?.workshoppedCards ?? []);

  // Check what abilities are pending
  let currentAbility = $derived(
    actionState?.abilityStack[actionState.abilityStack.length - 1] ?? null
  );

  let canDestroyDrafted = $derived(
    currentAbility === null // Can destroy drafted cards when no ability is pending
  );

  function handleDraftedCardClick(ci: CardInstance) {
    if (canDestroyDrafted && onAction) {
      onAction({ type: 'destroyDraftedCard', card: ci.card });
    }
  }

  function handleWorkshopCardClick(ci: CardInstance) {
    // Workshop cards can be selected for workshop ability
    if (currentAbility?.type === 'workshop' && onAction) {
      onAction({ type: 'workshop', cardTypes: [ci.card] });
    }
  }
</script>

{#if actionState}
  <!-- Drafted cards on table (interactive for destroy) -->
  {#each draftedCards as ci, i}
    <Card3D
      card={ci.card}
      position={[-1.5 + i * 0.55, 0.08, 0.6]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      interactive={canDestroyDrafted}
      onclick={() => handleDraftedCardClick(ci)}
    />
  {/each}

  <!-- Workshop cards -->
  {#each workshopCards as ci, i}
    <Card3D
      card={ci.card}
      position={[-1.5 + i * 0.55, 0.08, -0.8]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      interactive={currentAbility?.type === 'workshop'}
      onclick={() => handleWorkshopCardClick(ci)}
    />
  {/each}

  <!-- Workshopped cards (already used this turn) -->
  {#each workshoppedCards as ci, i}
    <Card3D
      card={ci.card}
      position={[2.0 + i * 0.15, 0.08, -0.8]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      highlighted={true}
    />
  {/each}
{/if}
