<script lang="ts">
  import { T, useTask } from '@threlte/core';
  import * as THREE from 'three';

  interface CandleConfig {
    position: [number, number, number];
    height: number;
    phase: number;
  }

  const candles: CandleConfig[] = [
    { position: [-1.2, 0.07, 0.5], height: 0.15, phase: 0 },
    { position: [1.2, 0.07, -0.8], height: 0.12, phase: 2.1 },
    { position: [-6, 2.0, -5], height: 0.1, phase: 1.3 },
    { position: [5, 2.0, -4], height: 0.1, phase: 3.7 },
    { position: [-4, 2.0, 3], height: 0.1, phase: 0.9 },
    { position: [6, 2.0, 2], height: 0.1, phase: 4.2 },
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
    varying vec2 vUv;
    void main() {
      float flicker = sin(time * 8.0 + vUv.x * 3.0) * 0.1 + 0.9;
      float shape = smoothstep(0.0, 0.5, vUv.y) * smoothstep(1.0, 0.3, vUv.y);
      float edge = smoothstep(0.5, 0.0, abs(vUv.x - 0.5));
      float intensity = shape * edge * flicker;
      vec3 color = mix(vec3(1.0, 0.3, 0.0), vec3(1.0, 0.9, 0.4), vUv.y);
      gl_FragColor = vec4(color * intensity * 2.0, intensity);
    }
  `;

  let time = 0;
  let flameMaterials: THREE.ShaderMaterial[] = [];

  // Create shader materials for each candle
  for (const candle of candles) {
    flameMaterials.push(new THREE.ShaderMaterial({
      uniforms: { time: { value: 0 } },
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
  });
</script>

{#each candles as candle, i}
  <!-- Candle holder (brass) -->
  <T.Mesh position={[candle.position[0], candle.position[1], candle.position[2]]} castShadow>
    <T.CylinderGeometry args={[0.05, 0.06, 0.03, 8]} />
    <T.MeshStandardMaterial color="#b87333" roughness={0.3} metalness={0.8} />
  </T.Mesh>

  <!-- Candle body -->
  <T.Mesh position={[candle.position[0], candle.position[1] + 0.015 + candle.height / 2, candle.position[2]]} castShadow>
    <T.CylinderGeometry args={[0.03, 0.035, candle.height, 8]} />
    <T.MeshStandardMaterial color="#f5efe0" roughness={0.9} />
  </T.Mesh>

  <!-- Flame (billboard plane) -->
  <T.Mesh
    position={[candle.position[0], candle.position[1] + 0.015 + candle.height + 0.05, candle.position[2]]}
    material={flameMaterials[i]}
  >
    <T.PlaneGeometry args={[0.06, 0.1]} />
  </T.Mesh>
{/each}
