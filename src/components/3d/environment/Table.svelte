<script lang="ts">
  import { T } from '@threlte/core';
  import * as THREE from 'three';
  import {
    createWoodTexture,
    createRoughnessMap,
    getCachedTexture,
  } from '../shaders/proceduralTextures';
  import { PENTAGON_CIRCUMRADIUS, PENTAGON_SIDES } from '../pentagonLayout';

  // Procedural wood grain for table top and legs - lightened for bright room
  const woodTexture = getCachedTexture('table-wood', () =>
    createWoodTexture(512, 512, {
      baseColor: [105, 78, 52],
      darkColor: [72, 52, 34],
      ringScale: 16,
      seed: 42,
    })
  );
  const woodRoughness = getCachedTexture('table-wood-rough', () =>
    createRoughnessMap(256, 256, { baseRoughness: 0.72, variation: 0.2, seed: 43 })
  );

  // Slightly lighter trim wood for edge trim
  const trimWoodTexture = getCachedTexture('table-trim', () =>
    createWoodTexture(256, 256, {
      baseColor: [72, 55, 38],
      darkColor: [48, 34, 22],
      ringScale: 12,
      seed: 50,
    })
  );

  // Pentagon geometry parameters from shared constants
  const CIRCUMRADIUS = PENTAGON_CIRCUMRADIUS;
  const SIDES = PENTAGON_SIDES;
  const TABLE_THICKNESS = 0.12;

  // Pentagon vertices (starting from top, going clockwise)
  // Oriented so one edge faces positive z (camera/player side)
  function pentagonVertices(radius: number): [number, number][] {
    const verts: [number, number][] = [];
    for (let i = 0; i < SIDES; i++) {
      const angle = (2 * Math.PI * i) / SIDES - Math.PI / 2;
      verts.push([radius * Math.cos(angle), radius * Math.sin(angle)]);
    }
    return verts;
  }

  const vertices = pentagonVertices(CIRCUMRADIUS);

  // Create pentagon shape for ExtrudeGeometry
  function createPentagonShape(radius: number): THREE.Shape {
    const shape = new THREE.Shape();
    const verts = pentagonVertices(radius);
    shape.moveTo(verts[0][0], verts[0][1]);
    for (let i = 1; i < verts.length; i++) {
      shape.lineTo(verts[i][0], verts[i][1]);
    }
    shape.closePath();
    return shape;
  }

  const pentagonShape = createPentagonShape(CIRCUMRADIUS);
  const tableTopGeometry = new THREE.ExtrudeGeometry(pentagonShape, {
    depth: TABLE_THICKNESS,
    bevelEnabled: false,
  });
  // ExtrudeGeometry extrudes along z, but we want the table horizontal.
  // Rotate the geometry so it lies flat (extrusion goes along y).
  tableTopGeometry.rotateX(-Math.PI / 2);
  // Top surface is now at y=0, bottom at y=-TABLE_THICKNESS

  // Create pentagon border shape (ring) by punching inner hole from outer
  const borderShape = createPentagonShape(CIRCUMRADIUS * 0.96);
  const holePath = new THREE.Path();
  const innerVerts = pentagonVertices(CIRCUMRADIUS * 0.93);
  holePath.moveTo(innerVerts[0][0], innerVerts[0][1]);
  for (let i = 1; i < innerVerts.length; i++) {
    holePath.lineTo(innerVerts[i][0], innerVerts[i][1]);
  }
  holePath.closePath();
  borderShape.holes.push(holePath);
  const borderGeometry = new THREE.ExtrudeGeometry(borderShape, {
    depth: 0.005,
    bevelEnabled: false,
  });
  borderGeometry.rotateX(-Math.PI / 2);

  // Apron: extruded pentagon ring hanging below the table top
  // Outer matches table edge, inner is slightly smaller
  const apronOuter = createPentagonShape(CIRCUMRADIUS);
  const apronHole = new THREE.Path();
  const apronInnerVerts = pentagonVertices(CIRCUMRADIUS - 0.06);
  apronHole.moveTo(apronInnerVerts[0][0], apronInnerVerts[0][1]);
  for (let i = 1; i < apronInnerVerts.length; i++) {
    apronHole.lineTo(apronInnerVerts[i][0], apronInnerVerts[i][1]);
  }
  apronHole.closePath();
  apronOuter.holes.push(apronHole);
  const apronGeometry = new THREE.ExtrudeGeometry(apronOuter, {
    depth: 0.14,
    bevelEnabled: false,
  });
  apronGeometry.rotateX(-Math.PI / 2);
  // Shift apron down so it hangs below the table top
  apronGeometry.translate(0, -TABLE_THICKNESS, 0);

  // Edge trim: a thin raised lip along the table edge
  const trimOuterShape = createPentagonShape(CIRCUMRADIUS + 0.02);
  const trimHolePath = new THREE.Path();
  const trimInnerVerts = pentagonVertices(CIRCUMRADIUS - 0.02);
  trimHolePath.moveTo(trimInnerVerts[0][0], trimInnerVerts[0][1]);
  for (let i = 1; i < trimInnerVerts.length; i++) {
    trimHolePath.lineTo(trimInnerVerts[i][0], trimInnerVerts[i][1]);
  }
  trimHolePath.closePath();
  trimOuterShape.holes.push(trimHolePath);
  const trimGeometry = new THREE.ExtrudeGeometry(trimOuterShape, {
    depth: 0.03,
    bevelEnabled: false,
  });
  trimGeometry.rotateX(-Math.PI / 2);

  // Leg positions at each vertex
  const legPositions: [number, number][] = vertices.map(([x, z]) => [x * 0.9, z * 0.9]);
</script>

<!-- Table Top (pentagon) -->
<T.Mesh position.y={0} castShadow receiveShadow geometry={tableTopGeometry}>
  <T.MeshStandardMaterial
    map={woodTexture}
    roughnessMap={woodRoughness}
    roughness={0.7}
    metalness={0.05}
  />
</T.Mesh>

<!-- Felt border inset (pentagon ring) -->
<T.Mesh position.y={0.065} geometry={borderGeometry}>
  <T.MeshStandardMaterial color="#2a1f15" roughness={0.85} />
</T.Mesh>

<!-- Table Legs at each vertex -->
{#each legPositions as [x, z]}
  <T.Mesh position={[x, -0.6, z]} castShadow>
    <T.CylinderGeometry args={[0.06, 0.08, 1.2, 8]} />
    <T.MeshStandardMaterial
      map={woodTexture}
      roughness={0.75}
      metalness={0.03}
    />
  </T.Mesh>
  <!-- Leg foot cap -->
  <T.Mesh position={[x, -1.18, z]}>
    <T.CylinderGeometry args={[0.09, 0.09, 0.04, 8]} />
    <T.MeshStandardMaterial color="#2a1f15" roughness={0.8} />
  </T.Mesh>
{/each}

<!-- Table apron (continuous pentagon ring below tabletop) -->
<T.Mesh position.y={0} castShadow geometry={apronGeometry}>
  <T.MeshStandardMaterial
    map={trimWoodTexture}
    roughness={0.78}
  />
</T.Mesh>

<!-- Edge Trim (thin raised lip along table edge) -->
<T.Mesh position.y={0} castShadow geometry={trimGeometry}>
  <T.MeshStandardMaterial
    map={trimWoodTexture}
    roughness={0.75}
  />
</T.Mesh>
