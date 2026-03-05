<script lang="ts">
  import { T, useTask } from '@threlte/core';
  import * as THREE from 'three';

  const CANDLE_POSITIONS: [number, number, number][] = [
    [-1.2, 0.35, 0.5],
    [1.2, 0.3, -0.8],
    [-6, 2.25, -5],
    [5, 2.25, -4],
  ];

  const PARTICLES_PER_CANDLE = 5;
  const COUNT = CANDLE_POSITIONS.length * PARTICLES_PER_CANDLE;

  interface SmokeParticle {
    x: number; y: number; z: number;
    spawnX: number; spawnY: number; spawnZ: number;
    vy: number;
    scale: number;
    phase: number;
    maxRise: number;
  }

  const particles: SmokeParticle[] = [];
  for (const [cx, cy, cz] of CANDLE_POSITIONS) {
    for (let j = 0; j < PARTICLES_PER_CANDLE; j++) {
      const startY = cy + Math.random() * 0.5;
      particles.push({
        x: cx, y: startY, z: cz,
        spawnX: cx, spawnY: cy, spawnZ: cz,
        vy: 0.015 + Math.random() * 0.01,
        scale: 0.03 + Math.random() * 0.05,
        phase: Math.random() * Math.PI * 2,
        maxRise: 1.2 + Math.random() * 0.5,
      });
    }
  }

  let meshRef: THREE.InstancedMesh | undefined = $state();
  const dummy = new THREE.Object3D();
  let time = 0;

  useTask((delta) => {
    if (!meshRef) return;
    time += delta;

    for (let i = 0; i < COUNT; i++) {
      const p = particles[i];
      p.y += p.vy * delta;
      p.x = p.spawnX + Math.sin(time + p.phase) * 0.02;
      p.z = p.spawnZ + Math.cos(time * 0.8 + p.phase) * 0.015;

      // Respawn when risen too far
      if (p.y > p.spawnY + p.maxRise) {
        p.y = p.spawnY;
      }

      const progress = (p.y - p.spawnY) / p.maxRise;
      const fadeScale = p.scale * (1 - progress * 0.5);

      dummy.position.set(p.x, p.y, p.z);
      dummy.scale.setScalar(fadeScale);
      dummy.rotation.set(time * 0.3 + p.phase, time * 0.2, 0);
      dummy.updateMatrix();
      meshRef.setMatrixAt(i, dummy.matrix);
    }
    meshRef.instanceMatrix.needsUpdate = true;
  });
</script>

<T.InstancedMesh bind:ref={meshRef} args={[undefined, undefined, COUNT]}>
  <T.PlaneGeometry args={[1, 1]} />
  <T.MeshBasicMaterial
    color="#888888"
    transparent
    opacity={0.06}
    depthWrite={false}
    side={THREE.DoubleSide}
  />
</T.InstancedMesh>
