<script lang="ts">
  import { T } from '@threlte/core';
  import * as THREE from 'three';
  import { colorToHex } from '../../../data/colors';
  import type { Color } from '../../../data/types';
  import { createWoodTexture, getCachedTexture } from '../shaders/proceduralTextures';

  const pigmentColors: Color[] = ['Red', 'Vermilion', 'Orange', 'Amber', 'Yellow', 'Chartreuse', 'Green', 'Teal', 'Blue', 'Indigo', 'Purple', 'Magenta'];

  const shelfYPositions = [1.5, 2.2, 2.9, 3.6];

  // Jar shape variations: [topRadius, bottomRadius, height, segments]
  const jarShapes: [number, number, number, number][] = [
    [0.09, 0.10, 0.20, 8],   // standard
    [0.08, 0.12, 0.18, 8],   // wide bottom
    [0.10, 0.10, 0.24, 8],   // tall
    [0.07, 0.09, 0.16, 8],   // small
    [0.11, 0.11, 0.22, 8],   // wide
    [0.08, 0.10, 0.19, 6],   // hexagonal
    [0.09, 0.12, 0.17, 8],   // squat
    [0.10, 0.09, 0.23, 8],   // slightly inverted
    [0.08, 0.11, 0.20, 8],   // tapered
    [0.09, 0.09, 0.21, 10],  // round
    [0.11, 0.10, 0.18, 8],   // subtle flare
    [0.07, 0.10, 0.22, 8],   // narrow top
  ];

  const shelfWoodTexture = getCachedTexture('shelf-wood', () =>
    createWoodTexture(256, 256, {
      baseColor: [62, 44, 26],
      darkColor: [38, 24, 12],
      ringScale: 10,
      seed: 88,
    })
  );
</script>

<!-- Shelf frame uprights -->
{#each [-2.3, 2.3] as x}
  <T.Mesh position={[x, 2.5, -6.85]} castShadow>
    <T.BoxGeometry args={[0.08, 4, 0.08]} />
    <T.MeshStandardMaterial map={shelfWoodTexture} roughness={0.8} />
  </T.Mesh>
{/each}

<!-- Shelf planks -->
{#each shelfYPositions as y}
  <T.Mesh position={[0, y, -6.82]} castShadow>
    <T.BoxGeometry args={[4.8, 0.06, 0.5]} />
    <T.MeshStandardMaterial map={shelfWoodTexture} roughness={0.8} />
  </T.Mesh>
  <!-- Front lip on each shelf -->
  <T.Mesh position={[0, y + 0.03, -6.55]}>
    <T.BoxGeometry args={[4.8, 0.03, 0.02]} />
    <T.MeshStandardMaterial map={shelfWoodTexture} roughness={0.8} />
  </T.Mesh>
{/each}

<!-- Pigment jars on shelves with varied shapes -->
{#each pigmentColors as color, i}
  {@const shelfIndex = Math.floor(i / 4)}
  {@const posInShelf = (i % 4) - 1.5}
  {@const shape = jarShapes[i]}
  {@const y = shelfYPositions[shelfIndex] + 0.03 + shape[2] / 2}
  <!-- Jar body -->
  <T.Mesh position={[posInShelf * 1.0, y, -6.7]} castShadow>
    <T.CylinderGeometry args={[shape[0], shape[1], shape[2], shape[3]]} />
    <T.MeshStandardMaterial color={colorToHex(color)} roughness={0.6} metalness={0.05} />
  </T.Mesh>
  <!-- Jar lid -->
  <T.Mesh position={[posInShelf * 1.0, y + shape[2] / 2 + 0.01, -6.7]}>
    <T.CylinderGeometry args={[shape[0] * 0.85, shape[0] * 1.05, 0.025, shape[3]]} />
    <T.MeshStandardMaterial color="#5a4a3a" roughness={0.7} />
  </T.Mesh>
  <!-- Pigment powder visible at top (colored disc inside jar) -->
  <T.Mesh position={[posInShelf * 1.0, y + shape[2] / 2 - 0.02, -6.7]} rotation.x={-Math.PI / 2}>
    <T.CircleGeometry args={[shape[0] * 0.8, shape[3]]} />
    <T.MeshStandardMaterial color={colorToHex(color)} roughness={0.95} />
  </T.Mesh>
{/each}

<!-- Dye bottles (improved glass material) on lower shelf -->
{#each [[-1.8, 'Red'], [-1.2, 'Blue'], [-0.6, 'Yellow'], [0.6, 'Green'], [1.2, 'Purple']] as [x, color]}
  {@const y = shelfYPositions[0] + 0.03 + 0.15}
  <!-- Bottle body -->
  <T.Mesh position={[x as number, y, -6.65]}>
    <T.CylinderGeometry args={[0.06, 0.08, 0.25, 10]} />
    <T.MeshPhysicalMaterial
      color={colorToHex(color as Color)}
      roughness={0.05}
      metalness={0.0}
      transmission={0.6}
      thickness={0.5}
      transparent
      opacity={0.8}
    />
  </T.Mesh>
  <!-- Bottle neck -->
  <T.Mesh position={[x as number, y + 0.15, -6.65]}>
    <T.CylinderGeometry args={[0.025, 0.04, 0.06, 8]} />
    <T.MeshPhysicalMaterial
      color={colorToHex(color as Color)}
      roughness={0.05}
      metalness={0.0}
      transmission={0.5}
      thickness={0.3}
      transparent
      opacity={0.7}
    />
  </T.Mesh>
  <!-- Cork stopper -->
  <T.Mesh position={[x as number, y + 0.19, -6.65]}>
    <T.CylinderGeometry args={[0.028, 0.025, 0.03, 6]} />
    <T.MeshStandardMaterial color="#a08060" roughness={0.9} />
  </T.Mesh>
  <!-- Liquid level inside bottle -->
  <T.Mesh position={[x as number, y - 0.04, -6.65]}>
    <T.CylinderGeometry args={[0.055, 0.07, 0.12, 10]} />
    <T.MeshStandardMaterial
      color={colorToHex(color as Color)}
      roughness={0.3}
      transparent
      opacity={0.5}
    />
  </T.Mesh>
{/each}

<!-- Fabric rolls on bottom shelf -->
{#each [[-1.5, '#8b2020', 0.12], [0, '#2a4a7a', 0.14], [1.5, '#c4a040', 0.10]] as [x, color, radius]}
  <T.Mesh position={[x as number, shelfYPositions[0] + 0.03 + (radius as number), -6.55]} rotation.z={Math.PI / 2}>
    <T.CylinderGeometry args={[radius as number, radius as number, 0.4, 12]} />
    <T.MeshStandardMaterial color={color as string} roughness={0.85} />
  </T.Mesh>
  <!-- Fabric end circle -->
  <T.Mesh position={[(x as number) + 0.2, shelfYPositions[0] + 0.03 + (radius as number), -6.55]} rotation.z={Math.PI / 2}>
    <T.CircleGeometry args={[radius as number, 12]} />
    <T.MeshStandardMaterial color={color as string} roughness={0.9} side={THREE.FrontSide} />
  </T.Mesh>
{/each}

<!-- Small reference book propped on upper shelf -->
<T.Mesh position={[2.0, shelfYPositions[3] + 0.06, -6.7]} rotation.z={0.15}>
  <T.BoxGeometry args={[0.15, 0.12, 0.2]} />
  <T.MeshStandardMaterial color="#4a2a1a" roughness={0.85} />
</T.Mesh>
<!-- Book pages (edge visible) -->
<T.Mesh position={[2.07, shelfYPositions[3] + 0.06, -6.7]} rotation.z={0.15}>
  <T.BoxGeometry args={[0.01, 0.10, 0.18]} />
  <T.MeshStandardMaterial color="#e8dcc8" roughness={0.9} />
</T.Mesh>
