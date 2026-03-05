<script lang="ts">
  import { T } from '@threlte/core';
</script>

<!-- Table Top -->
<T.Mesh position.y={0} castShadow receiveShadow>
  <T.BoxGeometry args={[6, 0.12, 4]} />
  <T.MeshStandardMaterial color="#3a2a1a" roughness={0.7} metalness={0.05} />
</T.Mesh>

<!-- Felt Inlay -->
<T.Mesh position={[0, 0.065, 0]} rotation.x={-Math.PI / 2}>
  <T.PlaneGeometry args={[4.5, 3]} />
  <T.MeshStandardMaterial color="#2a3a2a" roughness={0.95} />
</T.Mesh>

<!-- Table Legs -->
{#each [[-2.7, 1.7], [2.7, 1.7], [-2.7, -1.7], [2.7, -1.7]] as [x, z]}
  <T.Mesh position={[x, -0.6, z]} castShadow>
    <T.BoxGeometry args={[0.15, 1.2, 0.15]} />
    <T.MeshStandardMaterial color="#3a2a1a" roughness={0.7} />
  </T.Mesh>
{/each}

<!-- Edge Trim -->
{#each [[0, 2, 6.04, 0.08], [0, -2, 6.04, 0.08]] as [x, z, w, d]}
  <T.Mesh position={[x, 0.02, z]} castShadow>
    <T.BoxGeometry args={[w, 0.08, d]} />
    <T.MeshStandardMaterial color="#2a1f15" roughness={0.75} />
  </T.Mesh>
{/each}
{#each [[-3, 0, 0.08, 4.04], [3, 0, 0.08, 4.04]] as [x, z, w, d]}
  <T.Mesh position={[x, 0.02, z]} castShadow>
    <T.BoxGeometry args={[w, 0.08, d]} />
    <T.MeshStandardMaterial color="#2a1f15" roughness={0.75} />
  </T.Mesh>
{/each}
