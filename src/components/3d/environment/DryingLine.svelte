<script lang="ts">
  import { T } from '@threlte/core';
  import * as THREE from 'three';

  const fabricColors = ['#a01525', '#152050', '#d4a520', '#5a1575', '#157070', '#8b4513', '#1a3a1a', '#8b0a2a'];
  const fabricPositions = [-3.2, -2.0, -0.7, 0.5, 1.7, 2.8, -1.4, 3.5];
  const fabricWidths = [0.72, 0.84, 0.68, 0.96, 0.6, 0.78, 0.55, 0.65];
  const fabricLengths = [1.6, 1.8, 1.5, 1.92, 1.44, 1.68, 1.3, 1.55];

  // Static sway angles for slight visual variety (no per-frame animation)
  const swayAngles: number[] = fabricPositions.map((_, i) =>
    Math.sin(i * 1.7) * 0.03 + Math.sin(i * 0.9) * 0.015
  );

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

</script>

<!-- Rope hooks (iron) -->
{#each [-3.8, 3.8] as z}
  <T.Mesh position={[-7.5, 3.5, z]}>
    <T.SphereGeometry args={[0.04, 6, 6]} />
    <T.MeshStandardMaterial color="#2a2a2a" roughness={0.7} metalness={0.5} />
  </T.Mesh>
  <!-- Hook bracket -->
  <T.Mesh position={[-7.8, 3.5, z]}>
    <T.BoxGeometry args={[0.06, 0.02, 0.02]} />
    <T.MeshStandardMaterial color="#2a2a2a" roughness={0.7} metalness={0.5} />
  </T.Mesh>
{/each}

<!-- Rope -->
<T.Mesh position={[-7.5, 3.5, 0]} rotation.x={Math.PI / 2}>
  <T.CylinderGeometry args={[0.012, 0.012, 7.6, 6]} />
  <T.MeshStandardMaterial color="#3a2a1a" roughness={0.9} />
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
    <T.MeshStandardMaterial color="#3a3028" roughness={0.85} />
  </T.Mesh>
{/each}
