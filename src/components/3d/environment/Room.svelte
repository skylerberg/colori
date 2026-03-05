<script lang="ts">
  import { T } from '@threlte/core';
  import * as THREE from 'three';
  import {
    createStoneTexture,
    createFlagstoneTexture,
    createRoughnessMap,
    getCachedTexture,
  } from '../shaders/proceduralTextures';

  // Procedural textures for walls and floor
  const wallTexture = getCachedTexture('room-wall', () =>
    createStoneTexture(512, 512, {
      baseColor: [98, 86, 74],
      accentColor: [72, 62, 52],
      roughnessLevel: 0.5,
      seed: 77,
    })
  );
  const wallRoughness = getCachedTexture('room-wall-rough', () =>
    createRoughnessMap(256, 256, { baseRoughness: 0.88, variation: 0.15, seed: 78 })
  );
  const floorTexture = getCachedTexture('room-floor', () =>
    createFlagstoneTexture(512, 512, {
      stoneColor: [80, 72, 64],
      mortarColor: [50, 44, 38],
      seed: 99,
    })
  );
  const floorRoughness = getCachedTexture('room-floor-rough', () =>
    createRoughnessMap(256, 256, { baseRoughness: 0.95, variation: 0.1, seed: 100 })
  );

  // Set wall texture repeat for proper scale
  wallTexture.repeat.set(2, 1);
  floorTexture.repeat.set(2, 2);
  wallRoughness.repeat.set(2, 1);
  floorRoughness.repeat.set(2, 2);

  // Ceiling texture: dark wood
  const ceilingTexture = getCachedTexture('room-ceiling', () =>
    createStoneTexture(256, 256, {
      baseColor: [42, 31, 24],
      accentColor: [30, 22, 16],
      roughnessLevel: 0.3,
      seed: 200,
    })
  );
  ceilingTexture.repeat.set(3, 2);
</script>

<!-- Floor -->
<T.Mesh rotation.x={-Math.PI / 2} position.y={-1.2} receiveShadow>
  <T.PlaneGeometry args={[16, 14]} />
  <T.MeshStandardMaterial
    map={floorTexture}
    roughnessMap={floorRoughness}
    roughness={0.95}
  />
</T.Mesh>

<!-- Back Wall -->
<T.Mesh position={[0, 1.8, -7]}>
  <T.PlaneGeometry args={[16, 6]} />
  <T.MeshStandardMaterial
    map={wallTexture}
    roughnessMap={wallRoughness}
    roughness={0.9}
  />
</T.Mesh>

<!-- Left Wall -->
<T.Mesh position={[-8, 1.8, 0]} rotation.y={Math.PI / 2}>
  <T.PlaneGeometry args={[14, 6]} />
  <T.MeshStandardMaterial
    map={wallTexture}
    roughnessMap={wallRoughness}
    roughness={0.9}
  />
</T.Mesh>

<!-- Right Wall -->
<T.Mesh position={[8, 1.8, 0]} rotation.y={-Math.PI / 2}>
  <T.PlaneGeometry args={[14, 6]} />
  <T.MeshStandardMaterial
    map={wallTexture}
    roughnessMap={wallRoughness}
    roughness={0.9}
  />
</T.Mesh>

<!-- Ceiling -->
<T.Mesh position.y={4.8} rotation.x={Math.PI / 2}>
  <T.PlaneGeometry args={[16, 14]} />
  <T.MeshStandardMaterial map={ceilingTexture} roughness={0.85} />
</T.Mesh>

<!-- Ceiling Beams -->
{#each [-6, -3, 0, 3, 6] as x}
  <T.Mesh position={[x, 4.6, 0]} castShadow>
    <T.BoxGeometry args={[0.3, 0.4, 14]} />
    <T.MeshStandardMaterial color="#2a1f18" roughness={0.8} />
  </T.Mesh>
{/each}

<!-- Cross Beams -->
{#each [-4, 0, 4] as z}
  <T.Mesh position={[0, 4.55, z]} castShadow>
    <T.BoxGeometry args={[16, 0.3, 0.25]} />
    <T.MeshStandardMaterial color="#2a1f18" roughness={0.8} />
  </T.Mesh>
{/each}

<!-- Baseboard trim along walls -->
{#each [[0, -0.95, -6.95, 16, 0.15, 0.1], [-7.95, -0.95, 0, 0.1, 0.15, 14], [7.95, -0.95, 0, 0.1, 0.15, 14]] as [x, y, z, w, h, d]}
  <T.Mesh position={[x, y, z]}>
    <T.BoxGeometry args={[w, h, d]} />
    <T.MeshStandardMaterial color="#2a1f15" roughness={0.8} />
  </T.Mesh>
{/each}
