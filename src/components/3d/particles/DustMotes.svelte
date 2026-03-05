<script lang="ts">
  import { T, useTask } from '@threlte/core';
  import * as THREE from 'three';

  const COUNT = 60;
  // Reposition bounds near back wall where windows now are
  const BOUNDS = { minX: -6, maxX: 6, minY: -0.5, maxY: 5, minZ: -7, maxZ: 1 };

  interface Particle {
    x: number; y: number; z: number;
    vx: number; vy: number; vz: number;
    scale: number;
    phase: number;
  }

  // Bias z-position toward back wall (z: -7 to -4) where windows are
  function biasedZ(): number {
    if (Math.random() < 0.65) {
      // Concentrate near window area (back wall)
      return -7 + Math.random() * 3;
    }
    return BOUNDS.minZ + Math.random() * (BOUNDS.maxZ - BOUNDS.minZ);
  }

  const particles: Particle[] = Array.from({ length: COUNT }, () => ({
    x: BOUNDS.minX + Math.random() * (BOUNDS.maxX - BOUNDS.minX),
    y: BOUNDS.minY + Math.random() * (BOUNDS.maxY - BOUNDS.minY),
    z: biasedZ(),
    vx: (Math.random() - 0.5) * 0.002,
    vy: (Math.random() - 0.5) * 0.003 + 0.001,
    vz: (Math.random() - 0.5) * 0.002,
    scale: 0.02 + Math.random() * 0.035,
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

      // Respawn if out of bounds — bias toward back wall
      if (p.x < BOUNDS.minX || p.x > BOUNDS.maxX ||
          p.y < BOUNDS.minY || p.y > BOUNDS.maxY ||
          p.z < BOUNDS.minZ || p.z > BOUNDS.maxZ) {
        p.x = BOUNDS.minX + Math.random() * (BOUNDS.maxX - BOUNDS.minX);
        p.y = BOUNDS.minY + Math.random() * (BOUNDS.maxY - BOUNDS.minY);
        p.z = biasedZ();
      }

      dummy.position.set(p.x, p.y, p.z);
      dummy.scale.setScalar(p.scale);
      dummy.updateMatrix();
      meshRef.setMatrixAt(i, dummy.matrix);
    }
    meshRef.instanceMatrix.needsUpdate = true;
  });
</script>

<T.InstancedMesh bind:ref={meshRef} args={[undefined, undefined, COUNT]} frustumCulled={false}>
  <T.SphereGeometry args={[1, 4, 4]} />
  <T.MeshBasicMaterial
    color="#ffcc66"
    transparent
    opacity={0.5}
    blending={THREE.AdditiveBlending}
    depthWrite={false}
  />
</T.InstancedMesh>
