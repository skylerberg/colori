<script lang="ts">
  import { T } from '@threlte/core';
  import { HTML } from '@threlte/extras';
  import * as THREE from 'three';
  import type { PlayerState, Color } from '../../../data/types';
  import { colorToHex, WHEEL_ORDER } from '../../../data/colors';
  import Card3D from '../cards/Card3D.svelte';

  let { player, playerName, position = [0, 0.07, 2.2] }: {
    player: PlayerState;
    playerName: string;
    position?: [number, number, number];
  } = $props();

  // Color wheel geometry: 12 segments as a flat disc
  const WHEEL_RADIUS = 0.35;
  const WHEEL_INNER_RADIUS = 0.1;
  const SEGMENT_COUNT = 12;
  const SEGMENT_ANGLE = (Math.PI * 2) / SEGMENT_COUNT;

  // Create a THREE.Shape for one segment
  function createSegmentShape(index: number): THREE.Shape {
    const startAngle = index * SEGMENT_ANGLE - Math.PI / 2; // Start from top
    const endAngle = startAngle + SEGMENT_ANGLE;
    const shape = new THREE.Shape();
    const steps = 16;

    // Outer arc
    for (let s = 0; s <= steps; s++) {
      const a = startAngle + (endAngle - startAngle) * (s / steps);
      const x = Math.cos(a) * WHEEL_RADIUS;
      const y = Math.sin(a) * WHEEL_RADIUS;
      if (s === 0) shape.moveTo(x, y);
      else shape.lineTo(x, y);
    }

    // Inner arc (reverse direction)
    for (let s = steps; s >= 0; s--) {
      const a = startAngle + (endAngle - startAngle) * (s / steps);
      const x = Math.cos(a) * WHEEL_INNER_RADIUS;
      const y = Math.sin(a) * WHEEL_INNER_RADIUS;
      shape.lineTo(x, y);
    }

    shape.closePath();
    return shape;
  }

  // Pre-create segment geometries
  let segmentGeometries = WHEEL_ORDER.map((_, i) => {
    const shape = createSegmentShape(i);
    const geom = new THREE.ShapeGeometry(shape);
    return geom;
  });

  // Material colors
  const MATERIAL_COLORS: Record<string, string> = {
    Textiles: '#c0392b',
    Ceramics: '#8b6914',
    Paintings: '#2a6bcf',
  };

  let totalScore = $derived(
    player.completedBuyers.length > 0
      ? player.completedBuyers.length
      : 0
  );
</script>

<T.Group position={position}>
  <!-- Color Wheel disc flat on the table -->
  <T.Group position={[-1.2, 0.01, 0]} rotation={[-Math.PI / 2, 0, 0]}>
    {#each WHEEL_ORDER as color, i}
      {@const hasColor = player.colorWheel[color] > 0}
      <T.Mesh geometry={segmentGeometries[i]}>
        <T.MeshStandardMaterial
          color={colorToHex(color)}
          roughness={0.6}
          metalness={0.1}
          opacity={hasColor ? 1.0 : 0.2}
          transparent={!hasColor}
          emissive={hasColor ? colorToHex(color) : '#000000'}
          emissiveIntensity={hasColor ? 0.15 : 0}
        />
      </T.Mesh>
    {/each}
    <!-- Wheel center -->
    <T.Mesh>
      <T.CircleGeometry args={[WHEEL_INNER_RADIUS - 0.005, 24]} />
      <T.MeshStandardMaterial color="#2a1f18" roughness={0.8} />
    </T.Mesh>
  </T.Group>

  <!-- Color count labels floating above wheel -->
  <HTML position={[-1.2, 0.3, 0]} transform sprite center>
    <div style="
      display: flex;
      gap: 2px;
      flex-wrap: wrap;
      max-width: 120px;
      justify-content: center;
    ">
      {#each WHEEL_ORDER as color}
        {#if player.colorWheel[color] > 0}
          <span style="
            background: {colorToHex(color)};
            color: white;
            font-size: 8px;
            font-weight: bold;
            padding: 1px 3px;
            border-radius: 2px;
            text-shadow: 0 1px 2px rgba(0,0,0,0.5);
          ">{color[0]}{player.colorWheel[color]}</span>
        {/if}
      {/each}
    </div>
  </HTML>

  <!-- Material tokens (small colored cubes) -->
  <T.Group position={[-0.4, 0, 0]}>
    {#each Object.entries(player.materials) as [material, count], mi}
      {#each Array(Math.min(count, 5)) as _, ci}
        <T.Mesh
          position={[mi * 0.15, 0.03 + ci * 0.06, 0]}
          castShadow
        >
          <T.BoxGeometry args={[0.08, 0.06, 0.08]} />
          <T.MeshStandardMaterial
            color={MATERIAL_COLORS[material] ?? '#888'}
            roughness={0.5}
            metalness={0.2}
          />
        </T.Mesh>
      {/each}
    {/each}
    <!-- Material labels -->
    <HTML position={[0.15, 0.25, 0]} transform sprite center>
      <div style="
        display: flex;
        gap: 6px;
        font-size: 9px;
        color: #ffe8cc;
        text-shadow: 0 1px 3px rgba(0,0,0,0.7);
        white-space: nowrap;
      ">
        {#each Object.entries(player.materials) as [material, count]}
          {#if count > 0}
            <span style="font-weight: 600;">{material}: {count}</span>
          {/if}
        {/each}
      </div>
    </HTML>
  </T.Group>

  <!-- Ducats display -->
  {#if player.ducats > 0}
    <T.Group position={[0.2, 0, 0]}>
      {#each Array(Math.min(player.ducats, 8)) as _, ci}
        <T.Mesh
          position={[ci * 0.08, 0.02, 0]}
          castShadow
        >
          <T.CylinderGeometry args={[0.04, 0.04, 0.015, 12]} />
          <T.MeshStandardMaterial
            color="#d4a017"
            roughness={0.3}
            metalness={0.6}
          />
        </T.Mesh>
      {/each}
      <HTML position={[0.15, 0.15, 0]} transform sprite center>
        <div style="
          font-size: 10px;
          color: #ffd700;
          font-weight: bold;
          text-shadow: 0 1px 3px rgba(0,0,0,0.7);
        ">{player.ducats} Ducats</div>
      </HTML>
    </T.Group>
  {/if}

  <!-- Completed buyers row -->
  {#each player.completedBuyers as bi, i}
    <Card3D
      buyerCard={bi.card}
      position={[0.8 + i * 0.4, 0.03, 0]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      scale={0.5}
    />
  {/each}
</T.Group>
