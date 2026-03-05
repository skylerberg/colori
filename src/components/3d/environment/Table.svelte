<script lang="ts">
  import { T } from '@threlte/core';
  import {
    createWoodTexture,
    createFabricTexture,
    createRoughnessMap,
    getCachedTexture,
  } from '../shaders/proceduralTextures';

  // Procedural wood grain for table top and legs
  const woodTexture = getCachedTexture('table-wood', () =>
    createWoodTexture(512, 512, {
      baseColor: [72, 50, 30],
      darkColor: [42, 28, 14],
      ringScale: 16,
      seed: 42,
    })
  );
  const woodRoughness = getCachedTexture('table-wood-rough', () =>
    createRoughnessMap(256, 256, { baseRoughness: 0.72, variation: 0.2, seed: 43 })
  );

  // Dark wood for edge trim (aged/stained)
  const trimWoodTexture = getCachedTexture('table-trim', () =>
    createWoodTexture(256, 256, {
      baseColor: [48, 34, 22],
      darkColor: [28, 18, 10],
      ringScale: 12,
      seed: 50,
    })
  );

  // Green felt inlay
  const feltTexture = getCachedTexture('table-felt', () =>
    createFabricTexture(256, 256, {
      baseColor: [38, 54, 38],
      seed: 55,
    })
  );
  const feltRoughness = getCachedTexture('table-felt-rough', () =>
    createRoughnessMap(128, 128, { baseRoughness: 0.95, variation: 0.05, seed: 56 })
  );
</script>

<!-- Table Top -->
<T.Mesh position.y={0} castShadow receiveShadow>
  <T.BoxGeometry args={[6, 0.12, 4]} />
  <T.MeshStandardMaterial
    map={woodTexture}
    roughnessMap={woodRoughness}
    roughness={0.7}
    metalness={0.05}
  />
</T.Mesh>

<!-- Felt Inlay -->
<T.Mesh position={[0, 0.065, 0]} rotation.x={-Math.PI / 2}>
  <T.PlaneGeometry args={[4.5, 3]} />
  <T.MeshStandardMaterial
    map={feltTexture}
    roughnessMap={feltRoughness}
    roughness={0.95}
  />
</T.Mesh>

<!-- Felt border inset (subtle recessed edge) -->
<T.Mesh position={[0, 0.062, 0]} rotation.x={-Math.PI / 2}>
  <T.RingGeometry args={[2.65, 2.72, 4]} />
  <T.MeshStandardMaterial color="#2a1f15" roughness={0.85} />
</T.Mesh>

<!-- Table Legs - slightly tapered with wear -->
{#each [[-2.7, 1.7], [2.7, 1.7], [-2.7, -1.7], [2.7, -1.7]] as [x, z]}
  <T.Mesh position={[x, -0.6, z]} castShadow>
    <T.CylinderGeometry args={[0.06, 0.08, 1.2, 8]} />
    <T.MeshStandardMaterial
      map={woodTexture}
      roughness={0.75}
      metalness={0.03}
    />
  </T.Mesh>
  <!-- Leg foot cap -->
  <T.Mesh position={[x, -1.18, z]}>
    <T.CylinderGeometry args={[0.09, 0.09, 0.04, 8]} />
    <T.MeshStandardMaterial color="#2a1f15" roughness={0.8} />
  </T.Mesh>
{/each}

<!-- Table apron (under-edge frame) -->
{#each [[0, -0.1, 2, 6, 0.14, 0.06], [0, -0.1, -2, 6, 0.14, 0.06]] as [x, y, z, w, h, d]}
  <T.Mesh position={[x, y, z]} castShadow>
    <T.BoxGeometry args={[w, h, d]} />
    <T.MeshStandardMaterial
      map={trimWoodTexture}
      roughness={0.78}
    />
  </T.Mesh>
{/each}
{#each [[-3, -0.1, 0, 0.06, 0.14, 4], [3, -0.1, 0, 0.06, 0.14, 4]] as [x, y, z, w, h, d]}
  <T.Mesh position={[x, y, z]} castShadow>
    <T.BoxGeometry args={[w, h, d]} />
    <T.MeshStandardMaterial
      map={trimWoodTexture}
      roughness={0.78}
    />
  </T.Mesh>
{/each}

<!-- Edge Trim (decorative) -->
{#each [[0, 2, 6.04, 0.08], [0, -2, 6.04, 0.08]] as [x, z, w, d]}
  <T.Mesh position={[x, 0.02, z]} castShadow>
    <T.BoxGeometry args={[w, 0.08, d]} />
    <T.MeshStandardMaterial
      map={trimWoodTexture}
      roughness={0.75}
    />
  </T.Mesh>
{/each}
{#each [[-3, 0, 0.08, 4.04], [3, 0, 0.08, 4.04]] as [x, z, w, d]}
  <T.Mesh position={[x, 0.02, z]} castShadow>
    <T.BoxGeometry args={[w, 0.08, d]} />
    <T.MeshStandardMaterial
      map={trimWoodTexture}
      roughness={0.75}
    />
  </T.Mesh>
{/each}
