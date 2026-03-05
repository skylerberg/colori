<script lang="ts">
  import { T } from '@threlte/core';
  import { HTML } from '@threlte/extras';
  import * as THREE from 'three';
  import type { PlayerState, Color } from '../../../data/types';
  import { colorToHex, WHEEL_ORDER } from '../../../data/colors';
  import Card3D from '../cards/Card3D.svelte';

  let { player, playerName, position = [0, 0.07, -1.7], isAI = false }: {
    player: PlayerState;
    playerName: string;
    position?: [number, number, number];
    isAI?: boolean;
  } = $props();

  let deckSize = $derived(player.deck.length + player.discard.length);
  let completedBuyers = $derived(player.completedBuyers);
  let workshopCards = $derived(player.workshopCards);
  let draftedCards = $derived(player.draftedCards);

  // Score = total stars from completed buyers (simplified: count of completed buyers)
  let score = $derived(completedBuyers.length);

  // Mini color wheel constants
  const MINI_WHEEL_RADIUS = 0.15;
  const MINI_INNER_RADIUS = 0.04;
  const SEGMENT_COUNT = 12;
  const SEGMENT_ANGLE = (Math.PI * 2) / SEGMENT_COUNT;

  function createMiniSegmentShape(index: number): THREE.Shape {
    const startAngle = index * SEGMENT_ANGLE - Math.PI / 2;
    const endAngle = startAngle + SEGMENT_ANGLE;
    const shape = new THREE.Shape();
    const steps = 8;

    for (let s = 0; s <= steps; s++) {
      const a = startAngle + (endAngle - startAngle) * (s / steps);
      const x = Math.cos(a) * MINI_WHEEL_RADIUS;
      const y = Math.sin(a) * MINI_WHEEL_RADIUS;
      if (s === 0) shape.moveTo(x, y);
      else shape.lineTo(x, y);
    }

    for (let s = steps; s >= 0; s--) {
      const a = startAngle + (endAngle - startAngle) * (s / steps);
      const x = Math.cos(a) * MINI_INNER_RADIUS;
      const y = Math.sin(a) * MINI_INNER_RADIUS;
      shape.lineTo(x, y);
    }

    shape.closePath();
    return shape;
  }

  let miniSegmentGeometries = WHEEL_ORDER.map((_, i) => {
    const shape = createMiniSegmentShape(i);
    return new THREE.ShapeGeometry(shape);
  });
</script>

<T.Group position={position}>
  <!-- Name label + score floating above -->
  <HTML position={[0, 0.55, 0]} transform sprite center>
    <div style="
      background: rgba(26, 20, 16, 0.85);
      color: #ffe8cc;
      padding: 3px 10px;
      border-radius: 4px;
      font-size: 11px;
      font-weight: 600;
      white-space: nowrap;
      backdrop-filter: blur(4px);
      display: flex;
      gap: 8px;
      align-items: center;
    ">
      <span>{playerName}{isAI ? ' (AI)' : ''}</span>
      {#if score > 0}
        <span style="
          color: #ffd700;
          font-size: 10px;
        ">Score: {score}</span>
      {/if}
    </div>
  </HTML>

  <!-- Mini color wheel (left side) -->
  <T.Group position={[-0.7, 0.01, 0]} rotation={[-Math.PI / 2, 0, 0]}>
    {#each WHEEL_ORDER as color, i}
      {@const hasColor = player.colorWheel[color] > 0}
      <T.Mesh geometry={miniSegmentGeometries[i]}>
        <T.MeshStandardMaterial
          color={colorToHex(color)}
          roughness={0.6}
          metalness={0.1}
          opacity={hasColor ? 1.0 : 0.15}
          transparent={!hasColor}
          emissive={hasColor ? colorToHex(color) : '#000000'}
          emissiveIntensity={hasColor ? 0.15 : 0}
        />
      </T.Mesh>
    {/each}
    <T.Mesh>
      <T.CircleGeometry args={[MINI_INNER_RADIUS - 0.002, 16]} />
      <T.MeshStandardMaterial color="#2a1f18" roughness={0.8} />
    </T.Mesh>
  </T.Group>

  <!-- Face-down deck stack -->
  {#if deckSize > 0}
    <T.Mesh position={[-0.35, 0.04, 0]} castShadow>
      <T.BoxGeometry args={[0.35, Math.min(deckSize * 0.003, 0.1), 0.49]} />
      <T.MeshStandardMaterial color="#2a1f18" roughness={0.8} />
    </T.Mesh>
  {/if}

  <!-- Workshop cards (public info) -->
  {#each workshopCards.slice(0, 5) as ci, i}
    <Card3D
      card={ci.card}
      position={[-0.05 + i * 0.32, 0.07, -0.35]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      scale={0.5}
    />
  {/each}

  <!-- Drafted cards stacked with offset -->
  {#each draftedCards.slice(0, 4) as ci, i}
    <Card3D
      card={ci.card}
      position={[-0.05 + i * 0.32, 0.07, 0]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      scale={0.55}
    />
  {/each}

  <!-- Completed buyers -->
  {#each completedBuyers.slice(0, 4) as bi, i}
    <Card3D
      buyerCard={bi.card}
      position={[-0.05 + i * 0.32, 0.07, 0.38]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      scale={0.45}
    />
  {/each}
</T.Group>
