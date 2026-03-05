<script lang="ts">
  import { T, useTask } from '@threlte/core';
  import * as THREE from 'three';

  const COUNT = 40;
  const BOUNDS = { minX: -7, maxX: 7, minY: 0, maxY: 4, minZ: -6, maxZ: 4 };

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
    vx: (Math.random() - 0.5) * 0.001,
    vy: (Math.random() - 0.5) * 0.001,
    vz: (Math.random() - 0.5) * 0.001,
    scale: 0.01 + Math.random() * 0.02,
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
      p.vx += (Math.random() - 0.5) * 0.00005;
      p.vy += (Math.random() - 0.5) * 0.00005;
      p.vz += (Math.random() - 0.5) * 0.00005;
      p.vx *= 0.995;
      p.vy *= 0.995;
      p.vz *= 0.995;

      p.x += p.vx + Math.sin(time * 0.2 + p.phase) * 0.0005;
      p.y += p.vy;
      p.z += p.vz;

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
    color="#ccbbaa"
    transparent
    opacity={0.1}
    blending={THREE.AdditiveBlending}
    depthWrite={false}
  />
</T.InstancedMesh>
