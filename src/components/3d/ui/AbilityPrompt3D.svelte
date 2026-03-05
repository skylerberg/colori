<script lang="ts">
  import { Text } from '@threlte/extras';
  import type { GameState, Choice, Ability } from '../../../data/types';
  import PrimaryColorPrompt3D from './PrimaryColorPrompt3D.svelte';
  import SecondaryColorPrompt3D from './SecondaryColorPrompt3D.svelte';
  import TertiarySwapPrompt3D from './TertiarySwapPrompt3D.svelte';

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
  {#if topAbility.type === 'gainPrimary'}
    <PrimaryColorPrompt3D {onAction} />
  {:else if topAbility.type === 'gainSecondary'}
    <SecondaryColorPrompt3D {onAction} />
  {:else if topAbility.type === 'changeTertiary'}
    <TertiarySwapPrompt3D colorWheel={currentPlayer.colorWheel} {onAction} />
  {:else if topAbility.type === 'sell'}
    <!-- Sell is handled by clicking buyers directly -->
    <Text
      text="Select a buyer to sell to"
      position={[0, 1.3, 2.0]}
      fontSize={0.06}
      color="#4a3a2a"
      anchorX="center"
      anchorY="middle"
      outlineWidth={0.003}
      outlineColor="#ffffff"
      fontWeight="bold"
    />
  {:else if topAbility.type === 'workshop'}
    <!-- Workshop is handled by clicking workshop cards -->
    <Text
      text={`Workshop ${topAbility.count} cards`}
      position={[0, 1.3, 2.0]}
      fontSize={0.06}
      color="#4a3a2a"
      anchorX="center"
      anchorY="middle"
      outlineWidth={0.003}
      outlineColor="#ffffff"
      fontWeight="bold"
    />
  {:else if topAbility.type === 'destroyCards'}
    <!-- Destroy cards is handled by clicking drafted cards -->
    <Text
      text="Destroy a card"
      position={[0, 1.3, 2.0]}
      fontSize={0.06}
      color="#4a3a2a"
      anchorX="center"
      anchorY="middle"
      outlineWidth={0.003}
      outlineColor="#ffffff"
      fontWeight="bold"
    />
  {/if}
{/if}
