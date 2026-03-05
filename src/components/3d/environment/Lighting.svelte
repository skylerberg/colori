<script lang="ts">
  import { T, useTask } from '@threlte/core';

  // Candle flicker state (decorative only in daylight mode)
  let tableCandle1Intensity = $state(0.4);
  let tableCandle2Intensity = $state(0.35);
  let time = 0;

  useTask((delta) => {
    time += delta;
    // Three layered sine waves for organic flicker - subtle in daylight
    const flicker1 = Math.sin(time * 8 + 0) * 0.12 + Math.sin(time * 13 + 0.7) * 0.08 + Math.sin(time * 21 + 1.3) * 0.05;
    const flicker2 = Math.sin(time * 8 + 2.1) * 0.12 + Math.sin(time * 13 + 3.5) * 0.08 + Math.sin(time * 21 + 0.4) * 0.05;
    tableCandle1Intensity = 0.4 * (1.0 + flicker1);
    tableCandle2Intensity = 0.35 * (1.0 + flicker2);
  });
</script>

<!-- Ambient Fill (bright warm daylight) -->
<T.AmbientLight color="#fff5e6" intensity={1.6} />

<!-- Hemisphere Light (natural sky/ground bounce) -->
<T.HemisphereLight
  args={["#87CEEB", "#d4a574", 0.8]}
/>

<!-- Primary Directional Sunlight (from back wall windows) -->
<T.DirectionalLight
  position={[2, 6, -6]}
  color="#fff8f0"
  intensity={3.5}
  castShadow
  shadow.mapSize.width={1024}
  shadow.mapSize.height={1024}
  shadow.camera.left={-8}
  shadow.camera.right={8}
  shadow.camera.top={6}
  shadow.camera.bottom={-6}
  shadow.camera.near={0.5}
  shadow.camera.far={20}
  shadow.bias={-0.0001}
/>

<!-- Secondary Directional Light (angled from upper left for depth) -->
<T.DirectionalLight
  position={[-4, 5, -3]}
  color="#fff0e0"
  intensity={1.8}
/>

<!-- Table Candle 1 (decorative, reduced intensity) -->
<T.PointLight
  position={[-1.2, 0.5, 0.5]}
  color="#ff9944"
  intensity={tableCandle1Intensity}
  distance={8}
/>

<!-- Table Candle 2 (decorative, reduced intensity) -->
<T.PointLight
  position={[1.2, 0.5, -0.8]}
  color="#ff8833"
  intensity={tableCandle2Intensity}
  distance={8}
/>

<!-- Window SpotLights - strong sunlight streaming from tall back wall windows -->
<T.SpotLight
  position={[-4.5, 5, -6.5]}
  target.position={[-4.5, 0, -1]}
  color="#fff8f0"
  intensity={2.8}
  angle={Math.PI / 4}
  penumbra={0.7}
/>
<T.SpotLight
  position={[0, 5, -6.5]}
  target.position={[0, 0, -1]}
  color="#fff8f0"
  intensity={2.8}
  angle={Math.PI / 4}
  penumbra={0.7}
/>
<T.SpotLight
  position={[4.5, 5, -6.5]}
  target.position={[4.5, 0, -1]}
  color="#fff8f0"
  intensity={2.8}
  angle={Math.PI / 4}
  penumbra={0.7}
/>

<!-- Left side fill (simulates light bouncing off left wall) -->
<T.PointLight position={[-6, 3, -2]} color="#ffe8d0" intensity={1.0} distance={14} />

<!-- Right side fill (simulates light bouncing off right wall) -->
<T.PointLight position={[6, 3, -2]} color="#ffe8d0" intensity={1.0} distance={14} />

<!-- Card Rim Light (functional readability) -->
<T.SpotLight
  position={[0, 3, 3]}
  target.position={[0, 0, 1.5]}
  color="#ffd8a0"
  intensity={0.4}
  angle={Math.PI / 4}
  penumbra={0.5}
/>
