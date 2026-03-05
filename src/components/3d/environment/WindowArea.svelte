<script lang="ts">
  import { T, useTask } from '@threlte/core';
  import * as THREE from 'three';

  // Volumetric light shaft shader
  const lightShaftVertexShader = `
    varying vec3 vWorldPosition;
    void main() {
      vec4 worldPos = modelMatrix * vec4(position, 1.0);
      vWorldPosition = worldPos.xyz;
      gl_Position = projectionMatrix * viewMatrix * worldPos;
    }
  `;

  const lightShaftFragmentShader = `
    uniform float time;
    varying vec3 vWorldPosition;
    void main() {
      float density = smoothstep(0.0, 1.0, 1.0 - length(vWorldPosition.xz - vec2(6.0, -2.0)) * 0.2);
      float noise = sin(vWorldPosition.y * 3.0 + time * 0.5) * 0.1 + 0.9;
      float heightFade = smoothstep(-1.0, 4.0, vWorldPosition.y);
      float opacity = density * noise * heightFade * 0.06;
      gl_FragColor = vec4(0.95, 0.9, 0.8, opacity);
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

  <!-- Glass panes (semi-transparent) -->
  <T.Mesh position={[0, 0, -0.01]}>
    <T.PlaneGeometry args={[1.2, 1.9]} />
    <T.MeshStandardMaterial color="#dde4f0" transparent opacity={0.15} roughness={0.1} />
  </T.Mesh>
</T.Group>

<!-- Light shaft volume -->
<T.Mesh position={[5, 1.5, -2]} rotation={[0.3, -0.2, 0]} material={lightShaftMaterial}>
  <T.PlaneGeometry args={[4, 5]} />
</T.Mesh>
