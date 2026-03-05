<script lang="ts">
  import { T, useTask } from '@threlte/core';
  import * as THREE from 'three';

  const fabricColors = ['#8b2020', '#2a4a7a', '#c4a040', '#5a2a5a'];
  const fabricPositions = [-2, -0.5, 1, 2.5];

  let time = 0;
  let swayAngles: number[] = $state(fabricPositions.map(() => 0));

  useTask((delta) => {
    time += delta;
    swayAngles = fabricPositions.map((_, i) =>
      Math.sin(time * 0.8 + i * 1.7) * 0.03
    );
  });
</script>

<!-- Rope hooks -->
{#each [-3.5, 3.5] as z}
  <T.Mesh position={[-7.5, 3.5, z]}>
    <T.SphereGeometry args={[0.03, 6, 6]} />
    <T.MeshStandardMaterial color="#5a4a3a" roughness={0.8} metalness={0.3} />
  </T.Mesh>
{/each}

<!-- Rope -->
<T.Mesh position={[-7.5, 3.5, 0]} rotation.x={Math.PI / 2}>
  <T.CylinderGeometry args={[0.01, 0.01, 7, 6]} />
  <T.MeshStandardMaterial color="#8b7355" roughness={0.9} />
</T.Mesh>

<!-- Hanging fabrics -->
{#each fabricPositions as z, i}
  <T.Mesh
    position={[-7.45, 2.7, z]}
    rotation.z={swayAngles[i]}
  >
    <T.PlaneGeometry args={[0.7, 1.5]} />
    <T.MeshStandardMaterial
      color={fabricColors[i]}
      roughness={0.85}
      side={THREE.DoubleSide}
    />
  </T.Mesh>
{/each}
