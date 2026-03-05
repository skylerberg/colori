<script lang="ts">
  import { T } from '@threlte/core';
  import * as THREE from 'three';
  import { createWoodTexture, getCachedTexture } from '../shaders/proceduralTextures';

  const benchWoodTexture = getCachedTexture('bench-wood', () =>
    createWoodTexture(256, 256, {
      baseColor: [80, 62, 42],
      darkColor: [50, 36, 22],
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

<!-- Mortar -->
<T.Mesh position={[-5.3, -0.17, -3]} castShadow>
  <T.CylinderGeometry args={[0.1, 0.13, 0.12, 8]} />
  <T.MeshStandardMaterial color="#7a7a7a" roughness={0.95} />
</T.Mesh>
<!-- Mortar interior (visible concave top) -->
<T.Mesh position={[-5.3, -0.11, -3]} rotation.x={-Math.PI / 2}>
  <T.CircleGeometry args={[0.09, 8]} />
  <T.MeshStandardMaterial color="#606060" roughness={0.9} />
</T.Mesh>
<!-- Ground pigment residue in mortar -->
<T.Mesh position={[-5.3, -0.13, -3]} rotation.x={-Math.PI / 2}>
  <T.CircleGeometry args={[0.06, 8]} />
  <T.MeshStandardMaterial color="#8b2020" roughness={0.95} />
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
  <!-- Chain links (simplified as thin cylinders) -->
  {#each [-0.11, 0.11] as xOff}
    <T.Mesh position={[xOff, 0.21, 0]}>
      <T.CylinderGeometry args={[0.003, 0.003, 0.05, 4]} />
      <T.MeshStandardMaterial color="#c4863e" roughness={0.3} metalness={0.7} />
    </T.Mesh>
  {/each}
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

<!-- Small bowl with pigment powder -->
<T.Group position={[-5.5, -0.27, -2.8]}>
  <T.Mesh castShadow>
    <T.CylinderGeometry args={[0.06, 0.08, 0.04, 8]} />
    <T.MeshStandardMaterial color="#6a5a48" roughness={0.85} />
  </T.Mesh>
  <!-- Powder inside -->
  <T.Mesh position.y={0.015} rotation.x={-Math.PI / 2}>
    <T.CircleGeometry args={[0.055, 8]} />
    <T.MeshStandardMaterial color="#2a4a7a" roughness={0.95} />
  </T.Mesh>
</T.Group>

<!-- Second small bowl -->
<T.Group position={[-4.5, -0.27, -2.75]}>
  <T.Mesh castShadow>
    <T.CylinderGeometry args={[0.05, 0.07, 0.035, 8]} />
    <T.MeshStandardMaterial color="#7a6a55" roughness={0.85} />
  </T.Mesh>
  <!-- Powder inside -->
  <T.Mesh position.y={0.012} rotation.x={-Math.PI / 2}>
    <T.CircleGeometry args={[0.045, 8]} />
    <T.MeshStandardMaterial color="#c4a040" roughness={0.95} />
  </T.Mesh>
</T.Group>

<!-- Parchment paper (curled slightly) -->
<T.Group position={[-5.1, -0.26, -2.55]}>
  <T.Mesh rotation={[-Math.PI / 2, 0, 0.3]}>
    <T.PlaneGeometry args={[0.22, 0.3]} />
    <T.MeshStandardMaterial color="#e8dcc0" roughness={0.85} side={THREE.DoubleSide} />
  </T.Mesh>
  <!-- Curled corner -->
  <T.Mesh position={[0.08, 0.005, 0.12]} rotation={[-1.2, 0, 0.3]}>
    <T.PlaneGeometry args={[0.06, 0.06]} />
    <T.MeshStandardMaterial color="#ddd0b5" roughness={0.85} side={THREE.DoubleSide} />
  </T.Mesh>
</T.Group>

<!-- Scattered pigment powder (small colored spots on bench surface) -->
{#each [[-5.4, '#8b2020'], [-5.1, '#2a4a7a'], [-4.8, '#c4a040']] as [x, color]}
  <T.Mesh position={[x as number, -0.268, -3.05]} rotation.x={-Math.PI / 2}>
    <T.CircleGeometry args={[0.03, 6]} />
    <T.MeshStandardMaterial color={color as string} roughness={0.95} transparent opacity={0.7} />
  </T.Mesh>
{/each}

<!-- Small cloth draped on edge -->
<T.Mesh position={[-5.5, -0.25, -2.6]} rotation={[0.2, 0.3, 0.1]}>
  <T.PlaneGeometry args={[0.3, 0.25]} />
  <T.MeshStandardMaterial color="#8b7355" roughness={0.9} side={THREE.DoubleSide} />
</T.Mesh>

<!-- Copper pot nearby -->
<T.Mesh position={[-5, -0.83, -4]} castShadow>
  <T.CylinderGeometry args={[0.18, 0.16, 0.25, 12]} />
  <T.MeshStandardMaterial color="#b87333" roughness={0.35} metalness={0.75} />
</T.Mesh>
<!-- Pot handle -->
<T.Mesh position={[-5.19, -0.7, -4]} rotation.z={Math.PI / 2}>
  <T.TorusGeometry args={[0.06, 0.01, 4, 8, Math.PI]} />
  <T.MeshStandardMaterial color="#a06828" roughness={0.4} metalness={0.7} />
</T.Mesh>

<!-- Water bucket on floor -->
<T.Group position={[-4.2, -1.05, -4.5]}>
  <T.Mesh castShadow>
    <T.CylinderGeometry args={[0.14, 0.12, 0.28, 10, 1, true]} />
    <T.MeshStandardMaterial color="#5a4a38" roughness={0.85} side={THREE.DoubleSide} />
  </T.Mesh>
  <!-- Bucket bottom -->
  <T.Mesh position.y={-0.14} rotation.x={-Math.PI / 2}>
    <T.CircleGeometry args={[0.12, 10]} />
    <T.MeshStandardMaterial color="#4a3a28" roughness={0.9} />
  </T.Mesh>
  <!-- Water surface -->
  <T.Mesh position.y={0.05} rotation.x={-Math.PI / 2}>
    <T.CircleGeometry args={[0.13, 10]} />
    <T.MeshStandardMaterial color="#3a4a5a" roughness={0.1} metalness={0.2} />
  </T.Mesh>
  <!-- Bucket metal band -->
  <T.Mesh position.y={0.08}>
    <T.TorusGeometry args={[0.14, 0.006, 4, 10]} />
    <T.MeshStandardMaterial color="#5a5a5a" roughness={0.5} metalness={0.6} />
  </T.Mesh>
</T.Group>
