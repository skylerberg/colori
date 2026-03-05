<script lang="ts">
  import { T, useTask } from '@threlte/core';
  import * as THREE from 'three';

  const COUNT = 80;
  const BOUNDS = { minX: 5, maxX: 8, minY: -0.5, maxY: 4, minZ: -4, maxZ: 0 };

  interface Particle {
    x: number; y: number; z: number;
    vx: number; vy: number; vz: number;
    scale: number;
    phase: number;
  }

  const particles: Particle[] = Array.from({ length: COUNT }, () => ({
    x: BOUNDS.minX + Math.random() * (BOUNDS.maxX - BOUNDS.minX),
    y: BOUNDS.minY + Math.random() * (BOUNDS.maxY - BOUNDS.minY),
    z: BOUNDS.minZ + Math.random() * (BOUNDS.maxZ - BOUNDS.minZ),
    vx: (Math.random() - 0.5) * 0.002,
    vy: (Math.random() - 0.5) * 0.003 + 0.001,
    vz: (Math.random() - 0.5) * 0.002,
    scale: 0.015 + Math.random() * 0.025,
    phase: Math.random() * Math.PI * 2,
  }));

  let meshRef: THREE.InstancedMesh | undefined = $state();
  const dummy = new THREE.Object3D();
  let time = 0;

  useTask((delta) => {
    if (!meshRef) return;
    time += delta;

    for (let i = 0; i < COUNT; i++) {
      const p = particles[i];
      // Brownian + sine drift
      p.vx += (Math.random() - 0.5) * 0.0001;
      p.vy += (Math.random() - 0.5) * 0.0001;
      p.vz += (Math.random() - 0.5) * 0.0001;
      p.vx *= 0.99;
      p.vy *= 0.99;
      p.vz *= 0.99;

      p.x += p.vx + Math.sin(time * 0.3 + p.phase) * 0.001;
      p.y += p.vy;
      p.z += p.vz + Math.cos(time * 0.25 + p.phase) * 0.0008;

      // Respawn if out of bounds
      if (p.x < BOUNDS.minX || p.x > BOUNDS.maxX ||
          p.y < BOUNDS.minY || p.y > BOUNDS.maxY ||
          p.z < BOUNDS.minZ || p.z > BOUNDS.maxZ) {
        p.x = BOUNDS.minX + Math.random() * (BOUNDS.maxX - BOUNDS.minX);
        p.y = BOUNDS.minY + Math.random() * (BOUNDS.maxY - BOUNDS.minY);
        p.z = BOUNDS.minZ + Math.random() * (BOUNDS.maxZ - BOUNDS.minZ);
      }

      dummy.position.set(p.x, p.y, p.z);
      dummy.scale.setScalar(p.scale);
      dummy.updateMatrix();
      meshRef.setMatrixAt(i, dummy.matrix);
    }
    meshRef.instanceMatrix.needsUpdate = true;
  });
</script>

<T.InstancedMesh bind:ref={meshRef} args={[undefined, undefined, COUNT]}>
  <T.SphereGeometry args={[1, 4, 4]} />
  <T.MeshBasicMaterial
    color="#ffffee"
    transparent
    opacity={0.3}
    blending={THREE.AdditiveBlending}
    depthWrite={false}
  />
</T.InstancedMesh>
