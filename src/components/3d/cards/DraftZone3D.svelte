<script lang="ts">
  import { T } from '@threlte/core';
  import { Text } from '@threlte/extras';
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
  let pickNumber = $derived(draftState?.pickNumber ?? 1);

  // Track the last picked card for animation
  let lastPickedCard = $state<string | null>(null);

  function handleDraftPick(ci: CardInstance) {
    lastPickedCard = ci.card;
    // Reset after animation completes
    setTimeout(() => { lastPickedCard = null; }, 600);
    if (onAction) {
      onAction({ type: 'draftPick', card: ci.card });
    }
  }
</script>

{#if draftState}
  <!-- Pick indicator floating above tableau -->
  <Text
    text={`Pick ${pickNumber}`}
    position={[0, 0.5, 0]}
    fontSize={0.08}
    color="#ffffff"
    anchorX="center"
    anchorY="middle"
    outlineWidth={0.004}
    outlineColor="#2a6bcf"
    fontWeight="bold"
  />

  <!-- Player's hand (fan spread, in front of tableau toward camera) -->
  <CardHand3D
    cards={currentHand}
    interactive={true}
    onCardClick={handleDraftPick}
  />

  <!-- Workshop cards (positive z = player-facing side of tableau) -->
  {#each workshopCards as ci, i}
    {@const totalCards = workshopCards.length}
    {@const startX = -(totalCards - 1) * 0.22 / 2}
    <Card3D
      card={ci.card}
      position={[startX + i * 0.22, 0.04, 0.85]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      scale={0.35}
    />
  {/each}

  <!-- Drafted cards (negative z = toward table center, behind tableau) -->
  {#each draftedCards as ci, i}
    {@const isJustPicked = lastPickedCard === ci.card}
    {@const totalCards = draftedCards.length}
    {@const startX = -(totalCards - 1) * 0.22 / 2}
    <Card3D
      card={ci.card}
      position={[startX + i * 0.22, 0.04, -0.85]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      scale={0.35}
      highlighted={isJustPicked}
    />
  {/each}
{/if}
