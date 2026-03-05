<script lang="ts">
  import { T } from '@threlte/core';
  import {
    createWoodTexture,
    createRoughnessMap,
    getCachedTexture,
  } from '../shaders/proceduralTextures';

  // Rich dark wood floor - aged workshop planks
  const floorTexture = getCachedTexture('room-floor-wood', () =>
    createWoodTexture(512, 512, {
      baseColor: [50, 35, 20],
      darkColor: [35, 24, 14],
      ringScale: 25,
      seed: 99,
    })
  );
  const floorRoughness = getCachedTexture('room-floor-rough', () =>
    createRoughnessMap(256, 256, { baseRoughness: 0.85, variation: 0.15, seed: 100 })
  );

  floorTexture.repeat.set(3, 3);
  floorRoughness.repeat.set(3, 3);

  // Ceiling texture: warm exposed wood
  const ceilingTexture = getCachedTexture('room-ceiling-wood', () =>
    createWoodTexture(256, 256, {
      baseColor: [72, 52, 35],
      darkColor: [55, 38, 24],
      ringScale: 12,
      seed: 200,
    })
  );
  ceilingTexture.repeat.set(3, 2);

  // Venetian plaster wall color - warm cream/ivory
  const plasterColor = '#d4c4a8';
  // Stone frame color - warm grey
  const stoneColor = '#a09080';
  // Baseboard trim
  const trimColor = '#8a7a6a';

  // 5 majestic floor-to-ceiling arched windows along the back wall
  // Room is 16 units wide (-8 to +8), windows span most of it
  // Each window: 2.6 wide, pillars between: 0.6 wide
  // Layout: |edge(1.1)|win|pil(0.6)|win|pil(0.6)|win|pil(0.6)|win|pil(0.6)|win|edge(1.1)|
  const windowWidth = 2.6;
  const pillarWidth = 0.6;
  const windowCount = 5;
  // Total windows+pillars: 5*2.6 + 4*0.6 = 13.0 + 2.4 = 15.4, edges = (16-15.4)/2 = 0.3 each side... let's recalculate
  // Actually: 5*windowWidth + 4*pillarWidth + 2*edgeWidth = 16
  // 5*2.6 + 4*0.6 + 2*edge = 16 -> 13 + 2.4 + 2*edge = 16 -> edge = 0.3
  const edgeWidth = (16 - windowCount * windowWidth - (windowCount - 1) * pillarWidth) / 2;

  // Window positions (x centers)
  const windowPositions: number[] = [];
  for (let i = 0; i < windowCount; i++) {
    const x = -8 + edgeWidth + windowWidth / 2 + i * (windowWidth + pillarWidth);
    windowPositions.push(x);
  }

  // Floor-to-ceiling windows
  const floorY = -1.2;
  const ceilingY = 4.8;
  const windowBottomY = floorY; // Floor-to-ceiling
  const windowTopY = 4.2; // Near ceiling, leave room for arch crown
  const windowHeight = windowTopY - windowBottomY; // 5.4
  const windowCenterY = (windowBottomY + windowTopY) / 2; // 1.5

  // Arch parameters
  const archRadius = windowWidth / 2; // Semicircular arch radius = half window width
  const archTopY = windowTopY + archRadius * 0.4; // Arch crown extends a bit above rectangular top

  const frameThickness = 0.14;
  const frameDepth = 0.2;

  // Pillar positions (between windows)
  const pillarPositions: number[] = [];
  for (let i = 0; i < windowCount - 1; i++) {
    pillarPositions.push((windowPositions[i] + windowPositions[i + 1]) / 2);
  }
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

<!-- Back Wall - Venetian plaster with window cutouts -->
<!-- Far left edge segment -->
<T.Mesh position={[-8 + edgeWidth / 2, (floorY + ceilingY) / 2, -7]}>
  <T.PlaneGeometry args={[edgeWidth, ceilingY - floorY]} />
  <T.MeshStandardMaterial color={plasterColor} roughness={0.92} />
</T.Mesh>
<!-- Far right edge segment -->
<T.Mesh position={[8 - edgeWidth / 2, (floorY + ceilingY) / 2, -7]}>
  <T.PlaneGeometry args={[edgeWidth, ceilingY - floorY]} />
  <T.MeshStandardMaterial color={plasterColor} roughness={0.92} />
</T.Mesh>

<!-- Pillar wall segments between windows -->
{#each pillarPositions as px}
  <T.Mesh position={[px, (floorY + ceilingY) / 2, -7]}>
    <T.PlaneGeometry args={[pillarWidth, ceilingY - floorY]} />
    <T.MeshStandardMaterial color={plasterColor} roughness={0.92} />
  </T.Mesh>
{/each}

<!-- Wall strips above each window (between window top and ceiling) -->
{#each windowPositions as wx}
  <T.Mesh position={[wx, windowTopY + (ceilingY - windowTopY) / 2, -7]}>
    <T.PlaneGeometry args={[windowWidth, ceilingY - windowTopY]} />
    <T.MeshStandardMaterial color={plasterColor} roughness={0.92} />
  </T.Mesh>
{/each}

<!-- Window Frames - ornate stone/marble style -->
{#each windowPositions as wx}
  <!-- Frame - left vertical -->
  <T.Mesh position={[wx - windowWidth / 2, windowCenterY, -6.95]}>
    <T.BoxGeometry args={[frameThickness, windowHeight, frameDepth]} />
    <T.MeshStandardMaterial color={stoneColor} roughness={0.8} metalness={0.05} />
  </T.Mesh>
  <!-- Frame - right vertical -->
  <T.Mesh position={[wx + windowWidth / 2, windowCenterY, -6.95]}>
    <T.BoxGeometry args={[frameThickness, windowHeight, frameDepth]} />
    <T.MeshStandardMaterial color={stoneColor} roughness={0.8} metalness={0.05} />
  </T.Mesh>
  <!-- Frame - top horizontal -->
  <T.Mesh position={[wx, windowTopY, -6.95]}>
    <T.BoxGeometry args={[windowWidth + frameThickness, frameThickness, frameDepth]} />
    <T.MeshStandardMaterial color={stoneColor} roughness={0.8} metalness={0.05} />
  </T.Mesh>

  <!-- Arch segments above the rectangular window opening -->
  <!-- Approximate semicircular arch with 7 angled segments -->
  {#each Array.from({ length: 7 }, (_, i) => {
    const startAngle = Math.PI * (i / 7);
    const endAngle = Math.PI * ((i + 1) / 7);
    const midAngle = (startAngle + endAngle) / 2;
    const segX = wx + Math.cos(midAngle) * archRadius * 0.95;
    const segY = windowTopY + Math.sin(midAngle) * archRadius * 0.45;
    const segAngle = midAngle - Math.PI / 2;
    const segWidth = archRadius * Math.PI / 7 * 1.05;
    return { segX, segY, segAngle, segWidth };
  }) as seg}
    <T.Mesh
      position={[seg.segX, seg.segY, -6.94]}
      rotation.z={seg.segAngle}
    >
      <T.BoxGeometry args={[seg.segWidth, frameThickness * 1.2, frameDepth]} />
      <T.MeshStandardMaterial color={stoneColor} roughness={0.8} metalness={0.05} />
    </T.Mesh>
  {/each}

  <!-- Decorative keystone at arch crown -->
  <T.Mesh position={[wx, windowTopY + archRadius * 0.45, -6.92]}>
    <T.BoxGeometry args={[0.2, 0.35, frameDepth + 0.04]} />
    <T.MeshStandardMaterial color="#8a7a68" roughness={0.75} metalness={0.08} />
  </T.Mesh>

  <!-- Window sill at the bottom -->
  <T.Mesh position={[wx, floorY + 0.02, -6.85]}>
    <T.BoxGeometry args={[windowWidth + 0.3, 0.08, 0.35]} />
    <T.MeshStandardMaterial color={stoneColor} roughness={0.75} metalness={0.05} />
  </T.Mesh>

  <!-- Window cross bars - horizontal (two bars dividing window into thirds) -->
  <T.Mesh position={[wx, windowCenterY + windowHeight / 6, -6.95]}>
    <T.BoxGeometry args={[windowWidth, 0.05, frameDepth * 0.7]} />
    <T.MeshStandardMaterial color={stoneColor} roughness={0.8} />
  </T.Mesh>
  <T.Mesh position={[wx, windowCenterY - windowHeight / 6, -6.95]}>
    <T.BoxGeometry args={[windowWidth, 0.05, frameDepth * 0.7]} />
    <T.MeshStandardMaterial color={stoneColor} roughness={0.8} />
  </T.Mesh>
  <!-- Window cross bar - vertical center -->
  <T.Mesh position={[wx, windowCenterY, -6.95]}>
    <T.BoxGeometry args={[0.05, windowHeight, frameDepth * 0.7]} />
    <T.MeshStandardMaterial color={stoneColor} roughness={0.8} />
  </T.Mesh>
{/each}

<!-- Left Wall - Venetian plaster -->
<T.Mesh position={[-8, (floorY + ceilingY) / 2, 0]} rotation.y={Math.PI / 2}>
  <T.PlaneGeometry args={[14, ceilingY - floorY]} />
  <T.MeshStandardMaterial color={plasterColor} roughness={0.92} />
</T.Mesh>

<!-- Right Wall - Venetian plaster -->
<T.Mesh position={[8, (floorY + ceilingY) / 2, 0]} rotation.y={-Math.PI / 2}>
  <T.PlaneGeometry args={[14, ceilingY - floorY]} />
  <T.MeshStandardMaterial color={plasterColor} roughness={0.92} />
</T.Mesh>

<!-- Ceiling - keep as warm exposed wood -->
<T.Mesh position.y={ceilingY} rotation.x={Math.PI / 2}>
  <T.PlaneGeometry args={[16, 14]} />
  <T.MeshStandardMaterial map={ceilingTexture} roughness={0.85} />
</T.Mesh>

<!-- Ceiling Beams -->
{#each [-6, -3, 0, 3, 6] as x}
  <T.Mesh position={[x, ceilingY - 0.2, 0]} castShadow>
    <T.BoxGeometry args={[0.3, 0.4, 14]} />
    <T.MeshStandardMaterial color="#5a3f2a" roughness={0.75} />
  </T.Mesh>
{/each}

<!-- Cross Beams -->
{#each [-4, 0, 4] as z}
  <T.Mesh position={[0, ceilingY - 0.25, z]} castShadow>
    <T.BoxGeometry args={[16, 0.3, 0.25]} />
    <T.MeshStandardMaterial color="#5a3f2a" roughness={0.75} />
  </T.Mesh>
{/each}

<!-- Baseboard trim along walls -->
{#each [[0, floorY + 0.075, -6.95, 16, 0.15, 0.1], [-7.95, floorY + 0.075, 0, 0.1, 0.15, 14], [7.95, floorY + 0.075, 0, 0.1, 0.15, 14]] as [x, y, z, w, h, d]}
  <T.Mesh position={[x, y, z]}>
    <T.BoxGeometry args={[w, h, d]} />
    <T.MeshStandardMaterial color={trimColor} roughness={0.7} />
  </T.Mesh>
{/each}
