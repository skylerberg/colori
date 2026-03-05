<script lang="ts">
  import { T } from '@threlte/core';
  import type { GameState, BuyerInstance, Choice } from '../../../data/types';
  import Card3D from './Card3D.svelte';

  let { buyers, onAction, gameState }: {
    buyers: BuyerInstance[];
    onAction?: (choice: Choice) => void;
    gameState: GameState;
  } = $props();

  // Center of table, 2x3 grid layout
  const CENTER_X = 0;
  const CENTER_Z = -0.3;
  const CARD_SPACING_X = 0.6;
  const CARD_SPACING_Z = 0.85;

  // Can interact with buyers during action phase sell ability
  let canInteract = $derived(
    gameState.phase.type === 'action' &&
    gameState.phase.actionState.abilityStack.some(a => a.type === 'sell')
  );

  function getBuyerPosition(index: number): [number, number, number] {
    const col = index % 2;
    const row = Math.floor(index / 2);
    const x = CENTER_X + (col - 0.5) * CARD_SPACING_X;
    const z = CENTER_Z + (row - 0.5) * CARD_SPACING_Z;
    return [x, 0.08, z];
  }
</script>

{#each buyers as buyer, i}
  <Card3D
    buyerCard={buyer.card}
    position={getBuyerPosition(i)}
    rotation={[-Math.PI / 2, 0, 0]}
    faceUp={true}
    interactive={canInteract}
    scale={1.1}
    onclick={() => {
      if (canInteract && onAction) {
        onAction({ type: 'selectBuyer', buyer: buyer.card });
      }
    }}
  />
{/each}

<!-- Buyer deck stack (face down) -->
<T.Mesh position={[-1.8, 0.1, CENTER_Z]} castShadow>
  <T.BoxGeometry args={[0.5, 0.1, 0.7]} />
  <T.MeshStandardMaterial color="#2a1f18" roughness={0.8} />
</T.Mesh>

<!-- Draft deck stack (face down) -->
<T.Mesh position={[1.8, 0.1, CENTER_Z]} castShadow>
  <T.BoxGeometry args={[0.5, 0.1, 0.7]} />
  <T.MeshStandardMaterial color="#2a1f18" roughness={0.8} />
</T.Mesh>
