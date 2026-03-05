<script lang="ts">
  import { T } from '@threlte/core';
  import { colorToHex } from '../../../data/colors';
  import type { Color } from '../../../data/types';

  const pigmentColors: Color[] = ['Red', 'Vermilion', 'Orange', 'Amber', 'Yellow', 'Chartreuse', 'Green', 'Teal', 'Blue', 'Indigo', 'Purple', 'Magenta'];

  const shelfYPositions = [1.5, 2.2, 2.9, 3.6];
</script>

<!-- Shelf frame uprights -->
{#each [-2.3, 2.3] as x}
  <T.Mesh position={[x, 2.5, -6.85]} castShadow>
    <T.BoxGeometry args={[0.08, 4, 0.08]} />
    <T.MeshStandardMaterial color="#2a1f18" roughness={0.8} />
  </T.Mesh>
{/each}

<!-- Shelf planks -->
{#each shelfYPositions as y}
  <T.Mesh position={[0, y, -6.82]} castShadow>
    <T.BoxGeometry args={[4.8, 0.06, 0.5]} />
    <T.MeshStandardMaterial color="#3a2a1a" roughness={0.8} />
  </T.Mesh>
{/each}

<!-- Pigment jars on shelves -->
{#each pigmentColors as color, i}
  {@const shelfIndex = Math.floor(i / 4)}
  {@const posInShelf = (i % 4) - 1.5}
  {@const y = shelfYPositions[shelfIndex] + 0.03 + 0.125}
  <T.Mesh position={[posInShelf * 1.0, y, -6.7]} castShadow>
    <T.CylinderGeometry args={[0.1, 0.1, 0.2, 8]} />
    <T.MeshStandardMaterial color={colorToHex(color)} roughness={0.6} metalness={0.05} />
  </T.Mesh>
  <!-- Jar lid -->
  <T.Mesh position={[posInShelf * 1.0, y + 0.11, -6.7]}>
    <T.CylinderGeometry args={[0.08, 0.11, 0.02, 8]} />
    <T.MeshStandardMaterial color="#5a4a3a" roughness={0.7} />
  </T.Mesh>
{/each}

<!-- Dye bottles (glass-like, on lower shelf) -->
{#each [[-1.8, 'Red'], [-1.2, 'Blue'], [-0.6, 'Yellow'], [0.6, 'Green'], [1.2, 'Purple']] as [x, color]}
  {@const y = shelfYPositions[0] + 0.03 + 0.15}
  <T.Mesh position={[x as number, y, -6.65]}>
    <T.CylinderGeometry args={[0.06, 0.08, 0.25, 8]} />
    <T.MeshStandardMaterial
      color={colorToHex(color as Color)}
      roughness={0.1}
      metalness={0.1}
      transparent
      opacity={0.7}
    />
  </T.Mesh>
  <!-- Bottle neck -->
  <T.Mesh position={[x as number, y + 0.15, -6.65]}>
    <T.CylinderGeometry args={[0.025, 0.04, 0.06, 8]} />
    <T.MeshStandardMaterial
      color={colorToHex(color as Color)}
      roughness={0.1}
      transparent
      opacity={0.6}
    />
  </T.Mesh>
{/each}

<!-- Fabric rolls on bottom shelf -->
{#each [[-1.5, '#8b2020'], [0, '#2a4a7a'], [1.5, '#c4a040']] as [x, color]}
  <T.Mesh position={[x as number, shelfYPositions[0] + 0.03 + 0.12, -6.55]} rotation.z={Math.PI / 2}>
    <T.CylinderGeometry args={[0.12, 0.12, 0.4, 12]} />
    <T.MeshStandardMaterial color={color as string} roughness={0.85} />
  </T.Mesh>
{/each}
