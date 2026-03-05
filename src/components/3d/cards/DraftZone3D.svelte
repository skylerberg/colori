<script lang="ts">
  import { T } from '@threlte/core';
  import { HTML } from '@threlte/extras';
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
  <!-- Pick indicator -->
  <HTML position={[0, 0.9, 1.5]} transform sprite center>
    <div style="
      background: rgba(42, 107, 207, 0.85);
      color: #fff;
      padding: 4px 12px;
      border-radius: 12px;
      font-size: 13px;
      font-weight: 700;
      white-space: nowrap;
      backdrop-filter: blur(4px);
      letter-spacing: 0.5px;
    ">
      Pick {pickNumber}
    </div>
  </HTML>

  <!-- Player's hand (fan spread) -->
  <CardHand3D
    cards={currentHand}
    interactive={true}
    onCardClick={handleDraftPick}
  />

  <!-- Drafted cards laid out on table with slight overlap -->
  {#each draftedCards as ci, i}
    {@const isJustPicked = lastPickedCard === ci.card}
    <Card3D
      card={ci.card}
      position={[-1.5 + i * 0.5, 0.08, 0.6]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      highlighted={isJustPicked}
    />
  {/each}

  <!-- Workshop cards -->
  {#each workshopCards as ci, i}
    <Card3D
      card={ci.card}
      position={[-1.5 + i * 0.5, 0.08, -0.8]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
    />
  {/each}
{/if}
