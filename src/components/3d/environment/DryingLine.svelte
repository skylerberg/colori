<script lang="ts">
  import { T, useTask } from '@threlte/core';
  import * as THREE from 'three';

  const fabricColors = ['#8b2020', '#2a4a7a', '#c4a040', '#5a2a5a', '#2a5a5a'];
  const fabricPositions = [-2.5, -1, 0.5, 2, 3.2];
  const fabricWidths = [0.7, 0.6, 0.8, 0.5, 0.65];
  const fabricLengths = [1.5, 1.3, 1.6, 1.2, 1.4];

  let time = 0;
  let swayAngles: number[] = $state(fabricPositions.map(() => 0));

  // Create draped cloth geometries (deformed planes)
  const clothGeometries: THREE.PlaneGeometry[] = fabricPositions.map((_, i) => {
    const geo = new THREE.PlaneGeometry(fabricWidths[i], fabricLengths[i], 8, 12);
    const positions = geo.attributes.position;

    for (let vi = 0; vi < positions.count; vi++) {
      const x = positions.getX(vi);
      const y = positions.getY(vi);
      const halfW = fabricWidths[i] / 2;
      const halfH = fabricLengths[i] / 2;

      // Gentle drape curve - more pronounced in center and at bottom
      const normalizedY = (y + halfH) / fabricLengths[i]; // 0 at top, 1 at bottom
      const normalizedX = x / halfW; // -1 to 1

      // Cloth hangs forward slightly, more at bottom
      const forwardDrape = normalizedY * normalizedY * 0.08;
      // Slight side-to-side curve
      const sideCurve = Math.sin(normalizedX * Math.PI) * 0.02 * normalizedY;
      // Wrinkle detail
      const wrinkle = Math.sin(normalizedX * 5 + i * 2) * 0.01 * (1 - normalizedY * 0.5);
      const verticalWrinkle = Math.sin(normalizedY * 8 + i * 3) * 0.005;

      positions.setZ(vi, forwardDrape + sideCurve + wrinkle + verticalWrinkle);
    }

    geo.computeVertexNormals();
    return geo;
  });

  useTask((delta) => {
    time += delta;
    swayAngles = fabricPositions.map((_, i) =>
      Math.sin(time * 0.8 + i * 1.7) * 0.03 + Math.sin(time * 1.3 + i * 0.9) * 0.015
    );
  });
</script>

<!-- Rope hooks (iron) -->
{#each [-3.8, 3.8] as z}
  <T.Mesh position={[-7.5, 3.5, z]}>
    <T.SphereGeometry args={[0.04, 6, 6]} />
    <T.MeshStandardMaterial color="#3a3a3a" roughness={0.7} metalness={0.5} />
  </T.Mesh>
  <!-- Hook bracket -->
  <T.Mesh position={[-7.8, 3.5, z]}>
    <T.BoxGeometry args={[0.06, 0.02, 0.02]} />
    <T.MeshStandardMaterial color="#3a3a3a" roughness={0.7} metalness={0.5} />
  </T.Mesh>
{/each}

<!-- Rope -->
<T.Mesh position={[-7.5, 3.5, 0]} rotation.x={Math.PI / 2}>
  <T.CylinderGeometry args={[0.012, 0.012, 7.6, 6]} />
  <T.MeshStandardMaterial color="#8b7355" roughness={0.9} />
</T.Mesh>

<!-- Hanging fabrics with cloth drape -->
{#each fabricPositions as z, i}
  <T.Mesh
    position={[-7.45, 2.7, z]}
    rotation.z={swayAngles[i]}
    geometry={clothGeometries[i]}
  >
    <T.MeshStandardMaterial
      color={fabricColors[i]}
      roughness={0.85}
      side={THREE.DoubleSide}
    />
  </T.Mesh>

  <!-- Clothespin/clip at top of each fabric -->
  <T.Mesh position={[-7.45, 3.45, z]}>
    <T.BoxGeometry args={[0.04, 0.06, 0.02]} />
    <T.MeshStandardMaterial color="#6a5a40" roughness={0.8} />
  </T.Mesh>
{/each}
