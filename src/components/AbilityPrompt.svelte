<script lang="ts">
  import type { GameState, Choice, Ability } from '../data/types';
  import MixColorPrompt from './MixColorPrompt.svelte';
  import SellCardSelectPrompt from './SellCardSelectPrompt.svelte';
  import PrimaryColorPrompt from './PrimaryColorPrompt.svelte';
  import SecondaryColorPrompt from './SecondaryColorPrompt.svelte';
  import TertiarySwapPrompt from './TertiarySwapPrompt.svelte';

  let { gameState, onAction }: {
    gameState: GameState;
    onAction: (choice: Choice) => void;
  } = $props();

  let actionState = $derived(
    gameState.phase.type === 'action' ? gameState.phase.actionState : null
  );

  let topAbility: Ability | null = $derived(
    actionState && actionState.abilityStack.length > 0
      ? actionState.abilityStack[actionState.abilityStack.length - 1]
      : null
  );

  let currentPlayer = $derived(
    actionState ? gameState.players[actionState.currentPlayerIndex] : null
  );
</script>

{#if topAbility && currentPlayer}
  <div class="ability-prompt">
    {#if topAbility.type === 'mixColors'}
      <MixColorPrompt
        colorWheel={currentPlayer.colorWheel}
        remaining={topAbility.count}
        {onAction}
      />
    {:else if topAbility.type === 'sell'}
      <SellCardSelectPrompt {gameState} {onAction} />
    {:else if topAbility.type === 'gainPrimary'}
      <PrimaryColorPrompt {onAction} />
    {:else if topAbility.type === 'gainSecondary'}
      <SecondaryColorPrompt {onAction} />
    {:else if topAbility.type === 'changeTertiary'}
      <TertiarySwapPrompt colorWheel={currentPlayer.colorWheel} {onAction} />
    {/if}
  </div>
{/if}

<style>
  .ability-prompt {
    border: 2px solid var(--accent-gold, #c9a84c);
    border-radius: 8px;
    padding: 0.5rem;
    background: rgba(201, 168, 76, 0.06);
    max-width: 100%;
    overflow-x: auto;
  }

  @media (min-width: 640px) {
    .ability-prompt {
      padding: 0.625rem;
      border-radius: 10px;
    }
  }

  @media (min-width: 1024px) {
    .ability-prompt {
      padding: 0.75rem;
    }
  }
</style>
