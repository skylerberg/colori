<script lang="ts">
  import { T, useThrelte } from '@threlte/core';
  import * as THREE from 'three';

  interface CandleConfig {
    position: [number, number, number];
    height: number;
  }

  const candles: CandleConfig[] = [
    { position: [-1.2, 0.07, 0.5], height: 0.15 },
    { position: [1.2, 0.07, -0.8], height: 0.12 },
  ];
</script>

{#each candles as candle}
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

  <!-- Flame (simple emissive) -->
  <T.Mesh position={[candle.position[0], candle.position[1] + 0.015 + candle.height + 0.04, candle.position[2]]}>
    <T.SphereGeometry args={[0.025, 6, 6]} />
    <T.MeshBasicMaterial
      color="#ffaa33"
      transparent
      opacity={0.85}
      blending={THREE.AdditiveBlending}
      depthWrite={false}
    />
  </T.Mesh>
{/each}
