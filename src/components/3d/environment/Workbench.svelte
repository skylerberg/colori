<script lang="ts">
  import { T } from '@threlte/core';
</script>

<!-- Small workbench table -->
<T.Mesh position={[-5, -0.3, -3]} castShadow receiveShadow>
  <T.BoxGeometry args={[1.5, 0.06, 0.8]} />
  <T.MeshStandardMaterial color="#4a3a28" roughness={0.75} />
</T.Mesh>

<!-- Workbench legs -->
{#each [[-5.6, -3.3], [-5.6, -2.7], [-4.4, -3.3], [-4.4, -2.7]] as [x, z]}
  <T.Mesh position={[x, -0.93, z]}>
    <T.BoxGeometry args={[0.08, 1.2, 0.08]} />
    <T.MeshStandardMaterial color="#4a3a28" roughness={0.75} />
  </T.Mesh>
{/each}

<!-- Mortar -->
<T.Mesh position={[-5.3, -0.17, -3]} castShadow>
  <T.CylinderGeometry args={[0.1, 0.13, 0.12, 8]} />
  <T.MeshStandardMaterial color="#7a7a7a" roughness={0.95} />
</T.Mesh>

<!-- Pestle -->
<T.Mesh position={[-5.25, -0.08, -2.95]} rotation.z={0.4} castShadow>
  <T.CylinderGeometry args={[0.015, 0.025, 0.18, 6]} />
  <T.MeshStandardMaterial color="#8a8a8a" roughness={0.9} />
</T.Mesh>

<!-- Brass Scale -->
<T.Group position={[-4.6, -0.27, -3]}>
  <!-- Base -->
  <T.Mesh castShadow>
    <T.CylinderGeometry args={[0.08, 0.1, 0.02, 8]} />
    <T.MeshStandardMaterial color="#b87333" roughness={0.3} metalness={0.7} />
  </T.Mesh>
  <!-- Upright -->
  <T.Mesh position.y={0.12} castShadow>
    <T.CylinderGeometry args={[0.01, 0.01, 0.22, 6]} />
    <T.MeshStandardMaterial color="#b87333" roughness={0.3} metalness={0.7} />
  </T.Mesh>
  <!-- Beam -->
  <T.Mesh position.y={0.23} castShadow>
    <T.BoxGeometry args={[0.25, 0.01, 0.01]} />
    <T.MeshStandardMaterial color="#b87333" roughness={0.3} metalness={0.7} />
  </T.Mesh>
  <!-- Left pan -->
  <T.Mesh position={[-0.11, 0.18, 0]}>
    <T.CylinderGeometry args={[0.04, 0.04, 0.005, 8]} />
    <T.MeshStandardMaterial color="#b87333" roughness={0.3} metalness={0.7} />
  </T.Mesh>
  <!-- Right pan -->
  <T.Mesh position={[0.11, 0.18, 0]}>
    <T.CylinderGeometry args={[0.04, 0.04, 0.005, 8]} />
    <T.MeshStandardMaterial color="#b87333" roughness={0.3} metalness={0.7} />
  </T.Mesh>
</T.Group>

<!-- Small cloth draped on edge -->
<T.Mesh position={[-5.5, -0.25, -2.6]} rotation={[0.2, 0.3, 0.1]}>
  <T.PlaneGeometry args={[0.3, 0.25]} />
  <T.MeshStandardMaterial color="#8b7355" roughness={0.9} side={2} />
</T.Mesh>

<!-- Copper pot nearby -->
<T.Mesh position={[-5, -0.83, -4]} castShadow>
  <T.CylinderGeometry args={[0.18, 0.16, 0.25, 12]} />
  <T.MeshStandardMaterial color="#b87333" roughness={0.35} metalness={0.75} />
</T.Mesh>
