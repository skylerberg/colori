<script lang="ts">
  import { T, useTask } from '@threlte/core';
  import * as THREE from 'three';

  // Candle flicker state
  let tableCandle1Intensity = $state(1.2);
  let tableCandle2Intensity = $state(1.0);
  let time = 0;

  useTask((delta) => {
    time += delta;
    // Three layered sine waves for organic flicker
    const flicker1 = Math.sin(time * 8 + 0) * 0.05 + Math.sin(time * 13 + 0.7) * 0.03 + Math.sin(time * 21 + 1.3) * 0.02;
    const flicker2 = Math.sin(time * 8 + 2.1) * 0.05 + Math.sin(time * 13 + 3.5) * 0.03 + Math.sin(time * 21 + 0.4) * 0.02;
    tableCandle1Intensity = 1.2 * (1.0 + flicker1);
    tableCandle2Intensity = 1.0 * (1.0 + flicker2);
  });
</script>

<!-- Ambient Fill -->
<T.AmbientLight color="#3a2a1a" intensity={0.15} />

<!-- Window Daylight (cool) -->
<T.DirectionalLight
  position={[6, 4, -2]}
  color="#c8d4e8"
  intensity={0.6}
  castShadow
  shadow.mapSize.width={2048}
  shadow.mapSize.height={2048}
  shadow.camera.left={-8}
  shadow.camera.right={8}
  shadow.camera.top={6}
  shadow.camera.bottom={-6}
  shadow.camera.near={0.5}
  shadow.camera.far={20}
  shadow.bias={-0.0001}
/>

<!-- Table Candle 1 (main) -->
<T.PointLight
  position={[-1.2, 0.5, 0.5]}
  color="#ff9944"
  intensity={tableCandle1Intensity}
  castShadow
  shadow.mapSize.width={512}
  shadow.mapSize.height={512}
/>

<!-- Table Candle 2 -->
<T.PointLight
  position={[1.2, 0.5, -0.8]}
  color="#ff8833"
  intensity={tableCandle2Intensity}
  castShadow
  shadow.mapSize.width={512}
  shadow.mapSize.height={512}
/>

<!-- Wall Candles (ambient fill, no shadows) -->
<T.PointLight position={[-6, 3, -5]} color="#ff7722" intensity={0.6} />
<T.PointLight position={[5, 3, -4]} color="#ff7722" intensity={0.5} />
<T.PointLight position={[-4, 3, 3]} color="#ff8833" intensity={0.4} />
<T.PointLight position={[6, 3, 2]} color="#ff8833" intensity={0.3} />

<!-- Card Rim Light (readability) -->
<T.SpotLight
  position={[0, 3, 3]}
  target.position={[0, 0, 1.5]}
  color="#ffe8cc"
  intensity={0.4}
  angle={Math.PI / 4}
  penumbra={0.5}
/>

<!-- Shelf Highlight -->
<T.PointLight position={[0, 3.5, -6.5]} color="#ff9955" intensity={0.3} />
