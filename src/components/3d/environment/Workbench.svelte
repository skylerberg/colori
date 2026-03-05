<script lang="ts">
  import { T } from '@threlte/core';
  import { createWoodTexture, getCachedTexture } from '../shaders/proceduralTextures';

  const benchWoodTexture = getCachedTexture('bench-wood', () =>
    createWoodTexture(256, 256, {
      baseColor: [58, 42, 26],
      darkColor: [35, 25, 15],
      ringScale: 14,
      seed: 60,
    })
  );
</script>

<!-- Small workbench table -->
<T.Mesh position={[-5, -0.3, -3]} castShadow receiveShadow>
  <T.BoxGeometry args={[1.5, 0.06, 0.8]} />
  <T.MeshStandardMaterial map={benchWoodTexture} roughness={0.75} />
</T.Mesh>

<!-- Workbench legs -->
{#each [[-5.6, -3.3], [-5.6, -2.7], [-4.4, -3.3], [-4.4, -2.7]] as [x, z]}
  <T.Mesh position={[x, -0.93, z]}>
    <T.BoxGeometry args={[0.08, 1.2, 0.08]} />
    <T.MeshStandardMaterial map={benchWoodTexture} roughness={0.75} />
  </T.Mesh>
{/each}

<!-- Workbench stretcher (cross brace between legs) -->
<T.Mesh position={[-5, -1.1, -3]}>
  <T.BoxGeometry args={[1.1, 0.04, 0.04]} />
  <T.MeshStandardMaterial map={benchWoodTexture} roughness={0.8} />
</T.Mesh>

<!-- Copper pot nearby -->
<T.Mesh position={[-5, -0.83, -4]} castShadow>
  <T.CylinderGeometry args={[0.18, 0.16, 0.25, 12]} />
  <T.MeshStandardMaterial color="#c4982a" roughness={0.3} metalness={0.8} emissive="#ff8040" emissiveIntensity={0.02} />
</T.Mesh>
