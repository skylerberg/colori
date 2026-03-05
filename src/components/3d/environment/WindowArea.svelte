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
      // Distance-based density falloff from back wall center
      float density = smoothstep(0.0, 1.0, 1.0 - length(vWorldPosition.xz - vec2(0.0, -6.0)) * 0.15);

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

      float opacity = density * dustNoise * heightFade * edgeFade * 0.03;

      // Bright daylight color
      vec3 color = mix(vec3(0.95, 0.93, 0.88), vec3(0.90, 0.88, 0.82), vUv.y);
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

  // Procedural Mediterranean seascape backdrop
  function createSeascapeTexture(): THREE.CanvasTexture {
    const width = 1024;
    const height = 512;
    const canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext('2d')!;

    // --- Sky ---
    const skyGrad = ctx.createLinearGradient(0, 0, 0, height * 0.55);
    skyGrad.addColorStop(0, '#4a8ec2');
    skyGrad.addColorStop(0.3, '#6baed6');
    skyGrad.addColorStop(0.6, '#a8d4e6');
    skyGrad.addColorStop(0.85, '#e8d8b8');
    skyGrad.addColorStop(1.0, '#f0dfc0');
    ctx.fillStyle = skyGrad;
    ctx.fillRect(0, 0, width, height * 0.55);

    // --- Clouds ---
    ctx.globalAlpha = 0.35;
    const drawCloud = (cx: number, cy: number, w: number, h: number) => {
      const cloudGrad = ctx.createRadialGradient(cx, cy, 0, cx, cy, w);
      cloudGrad.addColorStop(0, '#ffffff');
      cloudGrad.addColorStop(0.4, 'rgba(255, 252, 245, 0.6)');
      cloudGrad.addColorStop(1, 'rgba(255, 250, 240, 0)');
      ctx.fillStyle = cloudGrad;
      ctx.beginPath();
      ctx.ellipse(cx, cy, w, h, 0, 0, Math.PI * 2);
      ctx.fill();
    };

    drawCloud(180, 60, 120, 30);
    drawCloud(260, 50, 80, 20);
    drawCloud(500, 80, 150, 25);
    drawCloud(620, 55, 90, 22);
    drawCloud(820, 70, 110, 28);
    drawCloud(380, 100, 70, 18);
    drawCloud(750, 95, 100, 20);

    ctx.globalAlpha = 1.0;

    // --- Distant hazy coastline ---
    ctx.fillStyle = '#b8c4a8';
    ctx.globalAlpha = 0.25;
    ctx.beginPath();
    ctx.moveTo(width * 0.65, height * 0.50);
    ctx.quadraticCurveTo(width * 0.72, height * 0.42, width * 0.80, height * 0.46);
    ctx.quadraticCurveTo(width * 0.88, height * 0.40, width * 0.94, height * 0.44);
    ctx.quadraticCurveTo(width, height * 0.42, width, height * 0.48);
    ctx.lineTo(width, height * 0.55);
    ctx.lineTo(width * 0.65, height * 0.55);
    ctx.closePath();
    ctx.fill();

    ctx.fillStyle = '#8da080';
    ctx.globalAlpha = 0.15;
    ctx.beginPath();
    ctx.moveTo(width * 0.70, height * 0.52);
    ctx.quadraticCurveTo(width * 0.78, height * 0.46, width * 0.85, height * 0.48);
    ctx.quadraticCurveTo(width * 0.92, height * 0.44, width, height * 0.47);
    ctx.lineTo(width, height * 0.55);
    ctx.lineTo(width * 0.70, height * 0.55);
    ctx.closePath();
    ctx.fill();

    ctx.globalAlpha = 1.0;

    // --- Sea ---
    const horizonY = height * 0.55;

    const seaGrad = ctx.createLinearGradient(0, horizonY, 0, height);
    seaGrad.addColorStop(0, '#5ab0b8');
    seaGrad.addColorStop(0.15, '#3a98a8');
    seaGrad.addColorStop(0.4, '#2a7a90');
    seaGrad.addColorStop(0.7, '#1e6878');
    seaGrad.addColorStop(1.0, '#165060');
    ctx.fillStyle = seaGrad;
    ctx.fillRect(0, horizonY, width, height - horizonY);

    // Sunlight reflection
    ctx.globalAlpha = 0.12;
    const sunPathGrad = ctx.createRadialGradient(
      width * 0.35, horizonY + 10, 0,
      width * 0.35, horizonY + 60, 200
    );
    sunPathGrad.addColorStop(0, '#fff8e0');
    sunPathGrad.addColorStop(0.5, 'rgba(255, 240, 200, 0.3)');
    sunPathGrad.addColorStop(1, 'rgba(255, 240, 200, 0)');
    ctx.fillStyle = sunPathGrad;
    ctx.fillRect(0, horizonY, width, height - horizonY);
    ctx.globalAlpha = 1.0;

    // Wave lines
    ctx.globalAlpha = 0.06;
    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 1;
    for (let y = horizonY + 5; y < height; y += 6) {
      const waveAmplitude = 1.5 + (y - horizonY) * 0.008;
      const waveFreq = 0.02 - (y - horizonY) * 0.00003;
      const offset = (y - horizonY) * 0.5;
      ctx.beginPath();
      for (let x = 0; x < width; x += 3) {
        const wy = y + Math.sin(x * waveFreq + offset) * waveAmplitude;
        if (x === 0) ctx.moveTo(x, wy);
        else ctx.lineTo(x, wy);
      }
      ctx.stroke();
    }
    ctx.globalAlpha = 1.0;

    // Horizon glow
    ctx.globalAlpha = 0.2;
    const horizonGlowGrad = ctx.createLinearGradient(0, horizonY - 4, 0, horizonY + 8);
    horizonGlowGrad.addColorStop(0, 'rgba(240, 220, 180, 0)');
    horizonGlowGrad.addColorStop(0.4, 'rgba(240, 220, 180, 0.4)');
    horizonGlowGrad.addColorStop(0.6, 'rgba(240, 220, 180, 0.3)');
    horizonGlowGrad.addColorStop(1, 'rgba(240, 220, 180, 0)');
    ctx.fillStyle = horizonGlowGrad;
    ctx.fillRect(0, horizonY - 4, width, 12);
    ctx.globalAlpha = 1.0;

    // --- Small distant sailboat ---
    ctx.globalAlpha = 0.3;
    const boatX = width * 0.25;
    const boatY = horizonY + 15;

    ctx.fillStyle = '#3a2a1a';
    ctx.beginPath();
    ctx.moveTo(boatX - 8, boatY);
    ctx.lineTo(boatX + 8, boatY);
    ctx.lineTo(boatX + 5, boatY + 3);
    ctx.lineTo(boatX - 5, boatY + 3);
    ctx.closePath();
    ctx.fill();

    ctx.fillStyle = '#f0e8d8';
    ctx.beginPath();
    ctx.moveTo(boatX, boatY - 14);
    ctx.lineTo(boatX + 7, boatY);
    ctx.lineTo(boatX, boatY);
    ctx.closePath();
    ctx.fill();

    ctx.globalAlpha = 1.0;

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.needsUpdate = true;
    return texture;
  }

  const seascapeTexture = createSeascapeTexture();

  // Window x positions matching Room.svelte layout (5 windows)
  const windowWidth = 2.6;
  const pillarWidth = 0.6;
  const windowCount = 5;
  const edgeWidth = (16 - windowCount * windowWidth - (windowCount - 1) * pillarWidth) / 2;
  const windowXPositions: number[] = [];
  for (let i = 0; i < windowCount; i++) {
    windowXPositions.push(-8 + edgeWidth + windowWidth / 2 + i * (windowWidth + pillarWidth));
  }
</script>

<!-- Mediterranean seascape backdrop behind the windows - full floor-to-ceiling height -->
<T.Mesh position={[0, 1.8, -7.5]}>
  <T.PlaneGeometry args={[18, 10]} />
  <T.MeshBasicMaterial
    map={seascapeTexture}
    toneMapped={false}
  />
</T.Mesh>

<!-- Subtle glass panes in each window opening -->
{#each windowXPositions as wx}
  <T.Mesh position={[wx, 1.5, -6.98]}>
    <T.PlaneGeometry args={[windowWidth, 5.4]} />
    <T.MeshPhysicalMaterial
      color="#c8e8f0"
      transparent
      opacity={0.12}
      roughness={0.05}
      metalness={0.1}
      transmission={0.9}
      thickness={0.1}
      side={THREE.DoubleSide}
    />
  </T.Mesh>
{/each}

<!-- Light shaft volumes for each of the 5 windows -->
{#each windowXPositions as wx, i}
  <T.Mesh
    position={[wx, 1.8, -3.8]}
    rotation={[-0.28, (i - 2) * 0.03, 0]}
    material={lightShaftMaterial}
  >
    <T.PlaneGeometry args={[2.8, 6]} />
  </T.Mesh>
{/each}
