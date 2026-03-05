<script lang="ts">
  import { T } from '@threlte/core';
  import { HTML } from '@threlte/extras';
  import type { GameState, BuyerInstance, Choice, Color } from '../../../data/types';
  import { getBuyerData } from '../../../data/cards';
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

  function colorToHex(color: Color): string {
    const map: Record<Color, string> = {
      Red: '#e63946',
      Yellow: '#f4d35e',
      Blue: '#457b9d',
      Orange: '#e76f51',
      Green: '#2d6a4f',
      Purple: '#7b2d8b',
      Vermilion: '#cc3314',
      Amber: '#d4a017',
      Chartreuse: '#7cb518',
      Teal: '#2a9d8f',
      Indigo: '#3f37c9',
      Magenta: '#c9184a',
    };
    return map[color] ?? '#888';
  }

  function getBuyerInfo(buyer: BuyerInstance) {
    const data = getBuyerData(buyer.card);
    return data;
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

  <!-- Hover detail popup -->
  {#if hoveredBuyerIndex === i && info}
    <HTML
      position={[pos[0], pos[1] + 0.6, pos[2]]}
      transform
      sprite
      center
    >
      <div style="
        background: rgba(26, 20, 16, 0.92);
        color: #ffe8cc;
        padding: 6px 10px;
        border-radius: 6px;
        font-size: 11px;
        white-space: nowrap;
        backdrop-filter: blur(4px);
        border: 1px solid rgba(184, 134, 11, 0.4);
        pointer-events: none;
      ">
        <div style="font-weight: 600; margin-bottom: 3px;">
          {info.stars} Stars - {info.requiredMaterial}
        </div>
        <div style="display: flex; gap: 3px; align-items: center;">
          {#each info.colorCost as color}
            <span style="
              display: inline-block;
              width: 10px;
              height: 10px;
              border-radius: 50%;
              background: {colorToHex(color)};
              border: 1px solid rgba(255,255,255,0.3);
            "></span>
          {/each}
        </div>
      </div>
    </HTML>
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
