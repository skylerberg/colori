<script lang="ts">
  import { T, useTask } from '@threlte/core';
  import * as THREE from 'three';

  // Volumetric light shaft shader with animated dust density
  const lightShaftVertexShader = `
    varying vec3 vWorldPosition;
    varying vec2 vUv;
    void main() {
      vUv = uv;
      vec4 worldPos = modelMatrix * vec4(position, 1.0);
      vWorldPosition = worldPos.xyz;
      gl_Position = projectionMatrix * viewMatrix * worldPos;
    }
  `;

  const lightShaftFragmentShader = `
    uniform float time;
    varying vec3 vWorldPosition;
    varying vec2 vUv;
    void main() {
      // Distance-based density falloff from window center
      float density = smoothstep(0.0, 1.0, 1.0 - length(vWorldPosition.xz - vec2(6.0, -2.0)) * 0.2);

      // Animated dust density variation (slow-moving bands)
      float dust1 = sin(vWorldPosition.y * 2.5 + time * 0.3 + vWorldPosition.x * 0.5) * 0.15 + 0.85;
      float dust2 = sin(vWorldPosition.y * 4.0 - time * 0.2 + vWorldPosition.z * 0.8) * 0.08;
      float dust3 = sin(vWorldPosition.y * 1.5 + time * 0.15) * 0.05;
      float dustNoise = dust1 + dust2 + dust3;

      // Height-based fade (stronger near ceiling, fades toward floor)
      float heightFade = smoothstep(-1.0, 4.0, vWorldPosition.y);

      // UV-based edge fade for smooth edges
      float edgeFade = smoothstep(0.0, 0.15, vUv.x) * smoothstep(1.0, 0.85, vUv.x)
                     * smoothstep(0.0, 0.1, vUv.y) * smoothstep(1.0, 0.9, vUv.y);

      float opacity = density * dustNoise * heightFade * edgeFade * 0.07;

      // Warm sunlight color with slight gradient
      vec3 color = mix(vec3(1.0, 0.92, 0.8), vec3(0.95, 0.85, 0.7), vUv.y);
      gl_FragColor = vec4(color, opacity);
    }
  `;

  const lightShaftMaterial = new THREE.ShaderMaterial({
    uniforms: { time: { value: 0 } },
    vertexShader: lightShaftVertexShader,
    fragmentShader: lightShaftFragmentShader,
    transparent: true,
    blending: THREE.AdditiveBlending,
    depthWrite: false,
    side: THREE.DoubleSide,
  });

  let time = 0;
  useTask((delta) => {
    time += delta;
    lightShaftMaterial.uniforms.time.value = time;
  });
</script>

<!-- Window frame -->
<T.Group position={[7.9, 2.5, -2]}>
  <!-- Frame outer -->
  {#each [[-0.65, 0, 0.06, 2.0], [0.65, 0, 0.06, 2.0]] as [x, y, w, h]}
    <T.Mesh position={[x, y, 0]} castShadow>
      <T.BoxGeometry args={[w, h, 0.08]} />
      <T.MeshStandardMaterial color="#2a1f18" roughness={0.8} />
    </T.Mesh>
  {/each}
  {#each [[0, 1.0, 1.3, 0.06], [0, -1.0, 1.3, 0.06]] as [x, y, w, h]}
    <T.Mesh position={[x, y, 0]} castShadow>
      <T.BoxGeometry args={[w, h, 0.08]} />
      <T.MeshStandardMaterial color="#2a1f18" roughness={0.8} />
    </T.Mesh>
  {/each}
  <!-- Mullion (center cross) -->
  <T.Mesh position={[0, 0, 0]}>
    <T.BoxGeometry args={[1.3, 0.04, 0.06]} />
    <T.MeshStandardMaterial color="#2a1f18" roughness={0.8} />
  </T.Mesh>
  <T.Mesh position={[0, 0, 0]}>
    <T.BoxGeometry args={[0.04, 2.0, 0.06]} />
    <T.MeshStandardMaterial color="#2a1f18" roughness={0.8} />
  </T.Mesh>

  <!-- Window sill -->
  <T.Mesh position={[0, -1.05, 0.08]}>
    <T.BoxGeometry args={[1.4, 0.06, 0.2]} />
    <T.MeshStandardMaterial color="#3a2a1a" roughness={0.8} />
  </T.Mesh>

  <!-- Glass panes (semi-transparent with slight imperfection) -->
  {#each [[-0.33, 0.5], [0.33, 0.5], [-0.33, -0.5], [0.33, -0.5]] as [px, py]}
    <T.Mesh position={[px, py, -0.01]}>
      <T.PlaneGeometry args={[0.58, 0.9]} />
      <T.MeshPhysicalMaterial
        color="#d8e0ec"
        transparent
        opacity={0.12}
        roughness={0.15}
        transmission={0.4}
        thickness={0.1}
      />
    </T.Mesh>
  {/each}

  <!-- Exterior daylight glow (behind window) -->
  <T.Mesh position={[0, 0, -0.15]}>
    <T.PlaneGeometry args={[1.5, 2.2]} />
    <T.MeshBasicMaterial
      color="#e8dcc8"
      transparent
      opacity={0.25}
    />
  </T.Mesh>
  <!-- Bright center glow -->
  <T.Mesh position={[0, 0.2, -0.16]}>
    <T.PlaneGeometry args={[0.8, 1.0]} />
    <T.MeshBasicMaterial
      color="#fff5e0"
      transparent
      opacity={0.15}
      blending={THREE.AdditiveBlending}
      depthWrite={false}
    />
  </T.Mesh>
</T.Group>

<!-- Light shaft volume (angled from window) -->
<T.Mesh position={[5, 1.5, -2]} rotation={[0.3, -0.2, 0]} material={lightShaftMaterial}>
  <T.PlaneGeometry args={[4, 5]} />
</T.Mesh>

<!-- Secondary light shaft (slightly offset for depth) -->
<T.Mesh position={[5.5, 1.8, -1.5]} rotation={[0.25, -0.15, 0.05]} material={lightShaftMaterial}>
  <T.PlaneGeometry args={[2, 4]} />
</T.Mesh>
