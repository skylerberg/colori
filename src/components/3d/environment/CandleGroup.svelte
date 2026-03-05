<script lang="ts">
  import { T, useTask, useThrelte } from '@threlte/core';
  import * as THREE from 'three';

  interface CandleConfig {
    position: [number, number, number];
    height: number;
    phase: number;
    hasDrips: boolean;
  }

  const candles: CandleConfig[] = [
    { position: [-1.2, 0.07, 0.5], height: 0.15, phase: 0, hasDrips: true },
    { position: [1.2, 0.07, -0.8], height: 0.12, phase: 2.1, hasDrips: true },
    { position: [-6, 2.0, -5], height: 0.1, phase: 1.3, hasDrips: false },
    { position: [5, 2.0, -4], height: 0.1, phase: 3.7, hasDrips: false },
    { position: [-4, 2.0, 3], height: 0.1, phase: 0.9, hasDrips: false },
    { position: [6, 2.0, 2], height: 0.1, phase: 4.2, hasDrips: false },
  ];

  // Wax drip positions for candles that have drips (seeded for consistency)
  const dripData: { offset: [number, number]; length: number; angle: number }[][] = [
    // Candle 0 drips
    [
      { offset: [0.03, 0], length: 0.06, angle: 0.2 },
      { offset: [-0.02, 0.02], length: 0.045, angle: -0.15 },
      { offset: [0.01, -0.03], length: 0.055, angle: 0.3 },
    ],
    // Candle 1 drips
    [
      { offset: [-0.025, 0.01], length: 0.05, angle: -0.25 },
      { offset: [0.02, -0.02], length: 0.04, angle: 0.1 },
    ],
  ];

  // Flame shader material
  const flameVertexShader = `
    varying vec2 vUv;
    void main() {
      vUv = uv;
      gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
    }
  `;

  const flameFragmentShader = `
    uniform float time;
    uniform float phase;
    varying vec2 vUv;
    void main() {
      float flicker = sin(time * 8.0 + phase + vUv.x * 3.0) * 0.1
                    + sin(time * 13.0 + phase * 1.3) * 0.05
                    + 0.85;
      float shape = smoothstep(0.0, 0.4, vUv.y) * smoothstep(1.0, 0.25, vUv.y);
      float edge = smoothstep(0.5, 0.0, abs(vUv.x - 0.5));
      float intensity = shape * edge * flicker;
      // Inner core is bright yellow, outer is orange
      vec3 inner = vec3(1.0, 0.95, 0.6);
      vec3 outer = vec3(1.0, 0.35, 0.05);
      vec3 color = mix(outer, inner, smoothstep(0.0, 0.7, vUv.y) * edge);
      gl_FragColor = vec4(color * intensity * 2.5, intensity);
    }
  `;

  let time = 0;
  let flameMaterials: THREE.ShaderMaterial[] = [];
  let flameRefs: (THREE.Mesh | undefined)[] = $state(candles.map(() => undefined));

  const { camera } = useThrelte();

  // Create shader materials for each candle
  for (const candle of candles) {
    flameMaterials.push(new THREE.ShaderMaterial({
      uniforms: {
        time: { value: 0 },
        phase: { value: candle.phase },
      },
      vertexShader: flameVertexShader,
      fragmentShader: flameFragmentShader,
      transparent: true,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
      side: THREE.DoubleSide,
    }));
  }

  useTask((delta) => {
    time += delta;
    for (const mat of flameMaterials) {
      mat.uniforms.time.value = time;
    }
    // Billboard: make flames always face camera
    const cam = camera.current;
    if (cam) {
      for (const mesh of flameRefs) {
        if (mesh) {
          mesh.quaternion.copy(cam.quaternion);
        }
      }
    }
  });
</script>

{#each candles as candle, i}
  <!-- Candle holder (brass) -->
  <T.Mesh position={[candle.position[0], candle.position[1], candle.position[2]]} castShadow>
    <T.CylinderGeometry args={[0.05, 0.06, 0.03, 8]} />
    <T.MeshStandardMaterial color="#b87333" roughness={0.3} metalness={0.8} />
  </T.Mesh>
  <!-- Holder rim -->
  <T.Mesh position={[candle.position[0], candle.position[1] + 0.015, candle.position[2]]}>
    <T.TorusGeometry args={[0.052, 0.005, 6, 12]} />
    <T.MeshStandardMaterial color="#c4863e" roughness={0.25} metalness={0.85} />
  </T.Mesh>

  <!-- Candle body -->
  <T.Mesh position={[candle.position[0], candle.position[1] + 0.015 + candle.height / 2, candle.position[2]]} castShadow>
    <T.CylinderGeometry args={[0.03, 0.035, candle.height, 8]} />
    <T.MeshStandardMaterial color="#f5efe0" roughness={0.9} />
  </T.Mesh>

  <!-- Wax drips on candles that have them -->
  {#if candle.hasDrips && dripData[i]}
    {#each dripData[i] as drip}
      <T.Mesh
        position={[
          candle.position[0] + drip.offset[0],
          candle.position[1] + 0.015 + candle.height * 0.6,
          candle.position[2] + drip.offset[1],
        ]}
        rotation.z={drip.angle}
      >
        <T.CylinderGeometry args={[0.006, 0.003, drip.length, 5]} />
        <T.MeshStandardMaterial color="#f0e8d0" roughness={0.85} />
      </T.Mesh>
      <!-- Drip bead at bottom -->
      <T.Mesh
        position={[
          candle.position[0] + drip.offset[0] + Math.sin(drip.angle) * drip.length * 0.4,
          candle.position[1] + 0.015 + candle.height * 0.6 - drip.length / 2,
          candle.position[2] + drip.offset[1],
        ]}
      >
        <T.SphereGeometry args={[0.007, 5, 5]} />
        <T.MeshStandardMaterial color="#ede5cc" roughness={0.8} />
      </T.Mesh>
    {/each}
  {/if}

  <!-- Wick -->
  <T.Mesh position={[candle.position[0], candle.position[1] + 0.015 + candle.height + 0.01, candle.position[2]]}>
    <T.CylinderGeometry args={[0.003, 0.003, 0.02, 4]} />
    <T.MeshStandardMaterial color="#1a1a1a" roughness={0.9} />
  </T.Mesh>

  <!-- Flame (billboarded plane) -->
  <T.Mesh
    bind:ref={flameRefs[i]}
    position={[candle.position[0], candle.position[1] + 0.015 + candle.height + 0.05, candle.position[2]]}
    material={flameMaterials[i]}
  >
    <T.PlaneGeometry args={[0.06, 0.1]} />
  </T.Mesh>

  <!-- Flame glow (soft additive sphere) -->
  <T.Mesh position={[candle.position[0], candle.position[1] + 0.015 + candle.height + 0.04, candle.position[2]]}>
    <T.SphereGeometry args={[0.04, 8, 8]} />
    <T.MeshBasicMaterial
      color="#ff8833"
      transparent
      opacity={0.08}
      blending={THREE.AdditiveBlending}
      depthWrite={false}
    />
  </T.Mesh>
{/each}
