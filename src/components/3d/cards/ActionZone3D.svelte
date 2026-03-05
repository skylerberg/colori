<script lang="ts">
  import { T } from '@threlte/core';
  import { Text } from '@threlte/extras';
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

  // Visual feedback: track recently destroyed/sold cards
  let destroyedCard = $state<string | null>(null);
  let soldCard = $state<string | null>(null);

  function handleDraftedCardClick(ci: CardInstance) {
    if (canDestroyDrafted && onAction) {
      destroyedCard = ci.card;
      setTimeout(() => { destroyedCard = null; }, 400);
      onAction({ type: 'destroyDraftedCard', card: ci.card });
    }
  }

  function handleWorkshopCardClick(ci: CardInstance) {
    // Workshop cards can be selected for workshop ability
    if (currentAbility?.type === 'workshop' && onAction) {
      onAction({ type: 'workshop', cardTypes: [ci.card] });
    }
  }

  // Current ability label for display
  let abilityLabel = $derived.by(() => {
    if (!currentAbility) return 'Destroy a drafted card or end turn';
    switch (currentAbility.type) {
      case 'sell': return 'Select a buyer to sell to';
      case 'workshop': return `Workshop ${currentAbility.count} cards`;
      case 'drawCards': return `Drawing ${currentAbility.count} cards`;
      case 'mixColors': return `Mix ${currentAbility.count} colors`;
      case 'destroyCards': return 'Destroy cards';
      case 'gainDucats': return `Gain ${currentAbility.count} ducats`;
      case 'gainSecondary': return 'Gain a secondary color';
      case 'gainPrimary': return 'Gain a primary color';
      case 'changeTertiary': return 'Change a tertiary color';
      default: return '';
    }
  });

  // End turn button hover state
  let endTurnHovered = $state(false);
</script>

{#if actionState}
  <!-- Ability status indicator floating above tableau -->
  {#if abilityLabel}
    <Text
      text={abilityLabel}
      position={[0, 0.5, 0]}
      fontSize={0.06}
      color="#ffe8cc"
      anchorX="center"
      anchorY="middle"
      outlineWidth={0.003}
      outlineColor="#2a1e14"
      fontWeight="bold"
    />
  {/if}

  <!-- Workshop cards (positive z = player-facing side) -->
  {#each workshopCards as ci, i}
    {@const totalCards = workshopCards.length}
    {@const startX = -(totalCards - 1) * 0.22 / 2}
    <Card3D
      card={ci.card}
      position={[startX + i * 0.22, 0.04, 0.85]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      scale={0.35}
      interactive={currentAbility?.type === 'workshop'}
      onclick={() => handleWorkshopCardClick(ci)}
    />
  {/each}

  <!-- Drafted cards (negative z = toward table center, behind tableau) -->
  {#each draftedCards as ci, i}
    {@const isBeingDestroyed = destroyedCard === ci.card}
    {@const totalCards = draftedCards.length}
    {@const startX = -(totalCards - 1) * 0.22 / 2}
    <Card3D
      card={ci.card}
      position={[startX + i * 0.22, 0.04, -0.85]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      scale={0.35}
      interactive={canDestroyDrafted}
      highlighted={isBeingDestroyed}
      onclick={() => handleDraftedCardClick(ci)}
    />
  {/each}

  <!-- Workshopped cards (already used, stacked to the right) -->
  {#each workshoppedCards as ci, i}
    <Card3D
      card={ci.card}
      position={[1.1 + i * 0.12, 0.04, 0]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      scale={0.3}
      highlighted={true}
    />
  {/each}

  <!-- End turn button -->
  {#if !currentAbility}
    <T.Group position={[1.0, 0.15, 0.5]}>
      <T.Mesh
        onclick={() => onAction?.({ type: 'endTurn' })}
        onpointerenter={() => { endTurnHovered = true; document.body.style.cursor = 'pointer'; }}
        onpointerleave={() => { endTurnHovered = false; document.body.style.cursor = 'auto'; }}
      >
        <T.BoxGeometry args={[0.4, 0.1, 0.15]} />
        <T.MeshStandardMaterial
          color={endTurnHovered ? '#3cb525' : '#2a6b14'}
          roughness={0.5}
          metalness={0.2}
          emissive={endTurnHovered ? '#3cb525' : '#000000'}
          emissiveIntensity={endTurnHovered ? 0.3 : 0}
        />
      </T.Mesh>
      <Text
        text="End Turn"
        position={[0, 0.06, 0]}
        fontSize={0.05}
        color="#ffffff"
        anchorX="center"
        anchorY="middle"
        fontWeight="bold"
      />
    </T.Group>
  {/if}
{/if}
