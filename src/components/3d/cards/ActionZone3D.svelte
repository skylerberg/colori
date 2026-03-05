<script lang="ts">
  import { T } from '@threlte/core';
  import { HTML } from '@threlte/extras';
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
</script>

{#if actionState}
  <!-- Ability status indicator -->
  {#if abilityLabel}
    <HTML position={[0, 0.9, 1.5]} transform sprite center>
      <div style="
        background: rgba(42, 30, 20, 0.88);
        color: #ffe8cc;
        padding: 4px 12px;
        border-radius: 8px;
        font-size: 12px;
        font-weight: 600;
        white-space: nowrap;
        backdrop-filter: blur(4px);
        border: 1px solid rgba(184, 134, 11, 0.3);
      ">
        {abilityLabel}
      </div>
    </HTML>
  {/if}

  <!-- Drafted cards on table (interactive for destroy) -->
  {#each draftedCards as ci, i}
    {@const isBeingDestroyed = destroyedCard === ci.card}
    <Card3D
      card={ci.card}
      position={[-1.5 + i * 0.5, 0.08, 0.6]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      interactive={canDestroyDrafted}
      highlighted={isBeingDestroyed}
      onclick={() => handleDraftedCardClick(ci)}
    />
  {/each}

  <!-- Workshop cards -->
  {#each workshopCards as ci, i}
    <Card3D
      card={ci.card}
      position={[-1.5 + i * 0.5, 0.08, -0.8]}
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

  <!-- End turn button area -->
  {#if !currentAbility}
    <HTML position={[2.0, 0.3, 0.6]} transform sprite center>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        style="
          background: rgba(42, 107, 20, 0.85);
          color: #fff;
          padding: 6px 16px;
          border-radius: 8px;
          font-size: 13px;
          font-weight: 700;
          cursor: pointer;
          white-space: nowrap;
          backdrop-filter: blur(4px);
          border: 1px solid rgba(100, 200, 50, 0.4);
          user-select: none;
        "
        onclick={() => onAction?.({ type: 'endTurn' })}
      >
        End Turn
      </div>
    </HTML>
  {/if}
{/if}
