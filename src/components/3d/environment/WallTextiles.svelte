<script lang="ts">
  import { T } from '@threlte/core';
  import * as THREE from 'three';

  // Game's 12-color palette (rich saturated versions)
  const palette = {
    red: '#c41e3a',
    yellow: '#daa520',
    blue: '#1e3a8a',
    orange: '#d2691e',
    green: '#228b22',
    purple: '#6b2fa0',
    vermilion: '#e34234',
    amber: '#ffbf00',
    chartreuse: '#7fff00',
    teal: '#008080',
    indigo: '#4b0082',
    magenta: '#c20078',
  };

  // --- Textile definitions ---
  // Each textile: position [x,y,z], rotation [rx,ry,rz], width, length, color, type
  interface Textile {
    pos: [number, number, number];
    rot: [number, number, number];
    width: number;
    length: number;
    color: string;
    hasFringe: boolean;
  }

  const textiles: Textile[] = [
    // RIGHT WALL (x=7.95, facing -X) - Front section (z > -0.5)
    { pos: [7.95, 3.2, 0.5], rot: [0, -Math.PI / 2, 0], width: 1.2, length: 2.8, color: palette.red, hasFringe: true },
    { pos: [7.95, 3.4, 2.2], rot: [0, -Math.PI / 2, 0], width: 0.9, length: 2.4, color: palette.blue, hasFringe: true },
    { pos: [7.95, 3.0, 3.8], rot: [0, -Math.PI / 2, 0], width: 1.0, length: 2.6, color: palette.amber, hasFringe: true },
    // RIGHT WALL - Back section (z < -5.0)
    { pos: [7.95, 3.3, -5.8], rot: [0, -Math.PI / 2, 0], width: 1.1, length: 2.5, color: palette.purple, hasFringe: true },
    { pos: [7.95, 3.1, -6.8], rot: [0, -Math.PI / 2, 0], width: 0.8, length: 2.0, color: palette.teal, hasFringe: false },
    // LEFT WALL (x=-7.95, facing +X) - Back corner (z < -5.5)
    { pos: [-7.95, 3.2, -6.0], rot: [0, Math.PI / 2, 0], width: 1.0, length: 2.4, color: palette.vermilion, hasFringe: true },
    { pos: [-7.95, 3.4, -6.9], rot: [0, Math.PI / 2, 0], width: 0.7, length: 1.8, color: palette.green, hasFringe: false },
    // BACK WALL PILLARS (z=-6.95, facing +Z) - narrow sashes on pillars
    { pos: [-4.8, 3.0, -6.95], rot: [0, 0, 0], width: 0.35, length: 1.4, color: palette.magenta, hasFringe: false },
    { pos: [-1.6, 3.2, -6.95], rot: [0, 0, 0], width: 0.3, length: 1.2, color: palette.orange, hasFringe: false },
    { pos: [1.6, 3.1, -6.95], rot: [0, 0, 0], width: 0.3, length: 1.3, color: palette.indigo, hasFringe: false },
    { pos: [4.8, 2.9, -6.95], rot: [0, 0, 0], width: 0.35, length: 1.5, color: palette.chartreuse, hasFringe: false },
  ];

  // Seeded pseudo-random for consistent drape variation
  function seededRandom(seed: number): number {
    const x = Math.sin(seed * 127.1 + 311.7) * 43758.5453;
    return x - Math.floor(x);
  }

  // Create draped cloth geometries with vertex displacement
  const clothGeometries: THREE.PlaneGeometry[] = textiles.map((t, i) => {
    const geo = new THREE.PlaneGeometry(t.width, t.length, 8, 14);
    const positions = geo.attributes.position;

    for (let vi = 0; vi < positions.count; vi++) {
      const x = positions.getX(vi);
      const y = positions.getY(vi);
      const halfW = t.width / 2;
      const halfH = t.length / 2;

      const normalizedY = (y + halfH) / t.length; // 0 at top, 1 at bottom
      const normalizedX = x / halfW; // -1 to 1

      // Cloth billows forward, more at bottom
      const forwardDrape = normalizedY * normalizedY * 0.1;
      // Side-to-side belly curve
      const sideCurve = Math.sin(normalizedX * Math.PI) * 0.03 * normalizedY;
      // Vertical wrinkles unique per textile
      const wrinkle = Math.sin(normalizedX * 6 + i * 2.3) * 0.012 * (1 - normalizedY * 0.4);
      // Horizontal creases
      const crease = Math.sin(normalizedY * 10 + i * 1.7) * 0.006;
      // Slight random bulge variation per textile
      const bulge = seededRandom(i * 17 + vi * 3) * 0.004 * normalizedY;

      positions.setZ(vi, forwardDrape + sideCurve + wrinkle + crease + bulge);
    }

    geo.computeVertexNormals();
    return geo;
  });

  // Iron bracket color
  const ironColor = '#2a2a2a';
  // Gold fringe color
  const fringeColor = '#8b7335';
</script>

<!-- Wall textiles -->
{#each textiles as textile, i}
  <!-- Iron mounting bracket -->
  <T.Mesh
    position={[textile.pos[0], textile.pos[1] + textile.length / 2 - 0.02, textile.pos[2]]}
    rotation.y={textile.rot[1]}
  >
    <T.BoxGeometry args={[textile.width * 0.6, 0.04, 0.04]} />
    <T.MeshStandardMaterial color={ironColor} roughness={0.6} metalness={0.6} />
  </T.Mesh>

  <!-- Two bracket end caps -->
  {#each [-1, 1] as side}
    <T.Mesh
      position={[
        textile.pos[0] + (textile.rot[1] === 0 ? side * textile.width * 0.3 : 0),
        textile.pos[1] + textile.length / 2 + 0.01,
        textile.pos[2] + (textile.rot[1] !== 0 ? side * textile.width * 0.3 * (textile.rot[1] > 0 ? 1 : -1) : 0)
      ]}
    >
      <T.SphereGeometry args={[0.03, 6, 6]} />
      <T.MeshStandardMaterial color={ironColor} roughness={0.6} metalness={0.6} />
    </T.Mesh>
  {/each}

  <!-- Draped fabric -->
  <T.Mesh
    position={textile.pos}
    rotation.x={textile.rot[0]}
    rotation.y={textile.rot[1]}
    rotation.z={textile.rot[2]}
    geometry={clothGeometries[i]}
  >
    <T.MeshStandardMaterial
      color={textile.color}
      roughness={0.85}
      side={THREE.DoubleSide}
    />
  </T.Mesh>

  <!-- Gold fringe at bottom of larger tapestries -->
  {#if textile.hasFringe}
    <T.Mesh
      position={[
        textile.pos[0] + (textile.rot[1] !== 0 ? (textile.rot[1] > 0 ? 0.06 : -0.06) : 0),
        textile.pos[1] - textile.length / 2 + 0.01,
        textile.pos[2] + (textile.rot[1] === 0 ? 0.06 : 0)
      ]}
      rotation.y={textile.rot[1]}
    >
      <T.BoxGeometry args={[textile.width * 0.95, 0.04, 0.02]} />
      <T.MeshStandardMaterial color={fringeColor} roughness={0.7} metalness={0.3} />
    </T.Mesh>
  {/if}
{/each}
