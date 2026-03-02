<script lang="ts">
  import type { GameState, Choice, Ability } from '../data/types';
  import MixColorPrompt from './MixColorPrompt.svelte';
  import BuyerSelectPrompt from './BuyerSelectPrompt.svelte';
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
      <BuyerSelectPrompt {gameState} {onAction} />
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
    border: 2px solid #d4a017;
    border-radius: 10px;
    padding: 16px;
    background: #fffef0;
  }
</style>
