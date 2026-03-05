<script lang="ts">
  import type { GameState, Choice, CardInstance } from '../../../data/types';
  import CardHand3D from './CardHand3D.svelte';
  import Card3D from './Card3D.svelte';

  let { gameState, onAction }: {
    gameState: GameState;
    onAction?: (choice: Choice) => void;
  } = $props();

  let draftState = $derived(
    gameState.phase.type === 'draft' ? gameState.phase.draftState : null
  );

  let currentPlayerIndex = $derived(draftState?.currentPlayerIndex ?? 0);
  let currentHand = $derived(draftState?.hands[currentPlayerIndex] ?? []);
  let draftedCards = $derived(gameState.players[currentPlayerIndex]?.draftedCards ?? []);
  let workshopCards = $derived(gameState.players[currentPlayerIndex]?.workshopCards ?? []);

  function handleDraftPick(ci: CardInstance) {
    if (onAction) {
      onAction({ type: 'draftPick', card: ci.card });
    }
  }
</script>

{#if draftState}
  <!-- Player's hand (fan spread) -->
  <CardHand3D
    cards={currentHand}
    interactive={true}
    onCardClick={handleDraftPick}
  />

  <!-- Drafted cards laid out on table -->
  {#each draftedCards as ci, i}
    <Card3D
      card={ci.card}
      position={[-1.5 + i * 0.55, 0.08, 0.6]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
    />
  {/each}

  <!-- Workshop cards -->
  {#each workshopCards as ci, i}
    <Card3D
      card={ci.card}
      position={[-1.5 + i * 0.55, 0.08, -0.8]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
    />
  {/each}
{/if}
