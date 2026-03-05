<script lang="ts">
  import { T } from '@threlte/core';
  import * as THREE from 'three';
</script>

<!-- Floor -->
<T.Mesh rotation.x={-Math.PI / 2} position.y={-1.2} receiveShadow>
  <T.PlaneGeometry args={[16, 14]} />
  <T.MeshStandardMaterial color="#5a5048" roughness={0.95} />
</T.Mesh>

<!-- Back Wall -->
<T.Mesh position={[0, 1.8, -7]}>
  <T.PlaneGeometry args={[16, 6]} />
  <T.MeshStandardMaterial color="#5a5048" roughness={0.9} />
</T.Mesh>

<!-- Left Wall -->
<T.Mesh position={[-8, 1.8, 0]} rotation.y={Math.PI / 2}>
  <T.PlaneGeometry args={[14, 6]} />
  <T.MeshStandardMaterial color="#5a5048" roughness={0.9} />
</T.Mesh>

<!-- Right Wall -->
<T.Mesh position={[8, 1.8, 0]} rotation.y={-Math.PI / 2}>
  <T.PlaneGeometry args={[14, 6]} />
  <T.MeshStandardMaterial color="#5a5048" roughness={0.9} />
</T.Mesh>

<!-- Ceiling -->
<T.Mesh position.y={4.8} rotation.x={Math.PI / 2}>
  <T.PlaneGeometry args={[16, 14]} />
  <T.MeshStandardMaterial color="#2a1f18" roughness={0.85} />
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
