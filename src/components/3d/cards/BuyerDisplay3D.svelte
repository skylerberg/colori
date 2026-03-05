<script lang="ts">
  import { T } from '@threlte/core';
  import { Text } from '@threlte/extras';
  import type { GameState, BuyerInstance, Choice, Color } from '../../../data/types';
  import { getBuyerData } from '../../../data/cards';
  import { colorToHex } from '../../../data/colors';
  import Card3D from './Card3D.svelte';

  let { buyers, onAction, gameState }: {
    buyers: BuyerInstance[];
    onAction?: (choice: Choice) => void;
    gameState: GameState;
  } = $props();

  // Center of table, 2x3 grid layout with improved spacing
  const CENTER_X = 0;
  const CENTER_Z = -0.3;
  const CARD_SPACING_X = 0.65;
  const CARD_SPACING_Z = 0.9;

  // Can interact with buyers during action phase sell ability
  let canInteract = $derived(
    gameState.phase.type === 'action' &&
    gameState.phase.actionState.abilityStack.some(a => a.type === 'sell')
  );

  // Track hovered buyer for detail popup
  let hoveredBuyerIndex = $state<number | null>(null);

  function getBuyerPosition(index: number): [number, number, number] {
    const col = index % 2;
    const row = Math.floor(index / 2);
    const x = CENTER_X + (col - 0.5) * CARD_SPACING_X;
    const z = CENTER_Z + (row - 0.5) * CARD_SPACING_Z;
    return [x, 0.08, z];
  }

  function getBuyerInfo(buyer: BuyerInstance) {
    return getBuyerData(buyer.card);
  }
</script>

{#each buyers as buyer, i}
  {@const pos = getBuyerPosition(i)}
  {@const info = getBuyerInfo(buyer)}
  <Card3D
    buyerCard={buyer.card}
    position={pos}
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

  <!-- Hover detail popup as 3D Text + colored spheres -->
  {#if hoveredBuyerIndex === i && info}
    <T.Group position={[pos[0], pos[1] + 0.6, pos[2]]}>
      <!-- Stars and material label -->
      <Text
        text={`${info.stars} Stars - ${info.requiredMaterial}`}
        position={[0, 0.08, 0]}
        fontSize={0.05}
        color="#ffe8cc"
        anchorX="center"
        anchorY="middle"
        outlineWidth={0.003}
        outlineColor="#1a1410"
        fontWeight="bold"
      />
      <!-- Color cost as small spheres in a row -->
      <T.Group position={[0, 0, 0]}>
        {#each info.colorCost as color, ci}
          {@const offsetX = (ci - (info.colorCost.length - 1) / 2) * 0.06}
          <T.Mesh position={[offsetX, 0, 0]}>
            <T.SphereGeometry args={[0.022, 12, 12]} />
            <T.MeshStandardMaterial
              color={colorToHex(color)}
              roughness={0.4}
              metalness={0.2}
              emissive={colorToHex(color)}
              emissiveIntensity={0.3}
            />
          </T.Mesh>
        {/each}
      </T.Group>
    </T.Group>
  {/if}

  <!-- Invisible hover zone above card -->
  <T.Mesh
    position={[pos[0], pos[1] + 0.01, pos[2]]}
    rotation={[-Math.PI / 2, 0, 0]}
    onpointerenter={() => { hoveredBuyerIndex = i; }}
    onpointerleave={() => { if (hoveredBuyerIndex === i) hoveredBuyerIndex = null; }}
    visible={false}
  >
    <T.PlaneGeometry args={[0.55, 0.77]} />
    <T.MeshBasicMaterial transparent opacity={0} />
  </T.Mesh>
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
