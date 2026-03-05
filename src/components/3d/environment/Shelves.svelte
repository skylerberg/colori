<script lang="ts">
  import { T } from '@threlte/core';
  import { createWoodTexture, getCachedTexture } from '../shaders/proceduralTextures';

  // Rich dark wood for shelf frames - matching room palette
  const shelfWoodTexture = getCachedTexture('shelf-wood', () =>
    createWoodTexture(256, 256, {
      baseColor: [58, 42, 26],
      darkColor: [77, 58, 40],
      ringScale: 10,
      seed: 88,
    })
  );

  // Shelf unit dimensions
  const unitWidth = 3.2;
  const unitHeight = 3.4;
  const unitDepth = 0.55;
  const sideThickness = 0.06;
  const shelfThickness = 0.08;

  // Colors
  const darkWood = '#3a2a1a';
  const accentWood = '#4d3a28';
  const trimWood = '#2a1f15';

  // Shelf Y positions (4 shelves)
  const shelfYPositions = [0.0, 0.8, 1.6, 2.4];

  // Base Y for both units
  const baseY = -1.2;
</script>

<!-- ===== LEFT SHELF UNIT (left wall, facing right) ===== -->
<T.Group position={[-7.5, 0, -3.5]} rotation.y={Math.PI / 2}>

  <!-- Base/Footer -->
  <T.Mesh position={[0, baseY + 0.06, 0]} castShadow>
    <T.BoxGeometry args={[unitWidth + 0.16, 0.12, unitDepth + 0.08]} />
    <T.MeshStandardMaterial color={trimWood} roughness={0.75} />
  </T.Mesh>

  <!-- Side panels -->
  {#each [-1, 1] as side}
    <T.Mesh position={[side * (unitWidth / 2 + sideThickness / 2), baseY + unitHeight / 2 + 0.12, 0]} castShadow>
      <T.BoxGeometry args={[sideThickness, unitHeight, unitDepth]} />
      <T.MeshStandardMaterial map={shelfWoodTexture} roughness={0.8} />
    </T.Mesh>
  {/each}

  <!-- Back panel -->
  <T.Mesh position={[0, baseY + unitHeight / 2 + 0.12, -unitDepth / 2 + 0.02]}>
    <T.BoxGeometry args={[unitWidth, unitHeight, 0.04]} />
    <T.MeshStandardMaterial color={darkWood} roughness={0.85} />
  </T.Mesh>

  <!-- Crown molding -->
  <T.Mesh position={[0, baseY + unitHeight + 0.12 + 0.05, 0.04]} castShadow>
    <T.BoxGeometry args={[unitWidth + 0.24, 0.10, unitDepth + 0.16]} />
    <T.MeshStandardMaterial color={trimWood} roughness={0.65} />
  </T.Mesh>

  <!-- Shelf planks -->
  {#each shelfYPositions as sy}
    <T.Mesh position={[0, baseY + 0.12 + sy, 0]} castShadow>
      <T.BoxGeometry args={[unitWidth, shelfThickness, unitDepth - 0.04]} />
      <T.MeshStandardMaterial map={shelfWoodTexture} roughness={0.8} />
    </T.Mesh>
  {/each}

</T.Group>

<!-- ===== RIGHT SHELF UNIT (right wall, facing left) ===== -->
<T.Group position={[7.5, 0, -3.0]} rotation.y={-Math.PI / 2}>

  <!-- Base/Footer -->
  <T.Mesh position={[0, baseY + 0.06, 0]} castShadow>
    <T.BoxGeometry args={[unitWidth + 0.16, 0.12, unitDepth + 0.08]} />
    <T.MeshStandardMaterial color={trimWood} roughness={0.75} />
  </T.Mesh>

  <!-- Side panels -->
  {#each [-1, 1] as side}
    <T.Mesh position={[side * (unitWidth / 2 + sideThickness / 2), baseY + unitHeight / 2 + 0.12, 0]} castShadow>
      <T.BoxGeometry args={[sideThickness, unitHeight, unitDepth]} />
      <T.MeshStandardMaterial map={shelfWoodTexture} roughness={0.8} />
    </T.Mesh>
  {/each}

  <!-- Back panel -->
  <T.Mesh position={[0, baseY + unitHeight / 2 + 0.12, -unitDepth / 2 + 0.02]}>
    <T.BoxGeometry args={[unitWidth, unitHeight, 0.04]} />
    <T.MeshStandardMaterial color={darkWood} roughness={0.85} />
  </T.Mesh>

  <!-- Crown molding -->
  <T.Mesh position={[0, baseY + unitHeight + 0.12 + 0.05, 0.04]} castShadow>
    <T.BoxGeometry args={[unitWidth + 0.24, 0.10, unitDepth + 0.16]} />
    <T.MeshStandardMaterial color={trimWood} roughness={0.65} />
  </T.Mesh>

  <!-- Center divider -->
  <T.Mesh position={[0, baseY + unitHeight / 2 + 0.12, 0]}>
    <T.BoxGeometry args={[0.05, unitHeight, unitDepth - 0.06]} />
    <T.MeshStandardMaterial map={shelfWoodTexture} roughness={0.8} />
  </T.Mesh>

  <!-- Shelf planks -->
  {#each shelfYPositions as sy}
    <T.Mesh position={[0, baseY + 0.12 + sy, 0]} castShadow>
      <T.BoxGeometry args={[unitWidth, shelfThickness, unitDepth - 0.04]} />
      <T.MeshStandardMaterial map={shelfWoodTexture} roughness={0.8} />
    </T.Mesh>
  {/each}

</T.Group>
