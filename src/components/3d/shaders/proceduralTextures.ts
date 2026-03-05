import * as THREE from 'three';

// Simple seeded pseudo-random number generator
function seededRandom(seed: number): () => number {
  let s = seed;
  return () => {
    s = (s * 16807 + 0) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

// 2D value noise
function valueNoise(x: number, y: number, scale: number, seed: number): number {
  const rand = seededRandom(seed);
  const sx = x * scale;
  const sy = y * scale;
  const ix = Math.floor(sx);
  const iy = Math.floor(sy);
  const fx = sx - ix;
  const fy = sy - iy;

  // Smoothstep
  const ux = fx * fx * (3 - 2 * fx);
  const uy = fy * fy * (3 - 2 * fy);

  // Hash corners
  const hash = (i: number, j: number) => {
    const h = (i * 374761393 + j * 668265263 + seed) & 0x7fffffff;
    return ((h * h * h) & 0x7fffffff) / 2147483647;
  };

  const n00 = hash(ix, iy);
  const n10 = hash(ix + 1, iy);
  const n01 = hash(ix, iy + 1);
  const n11 = hash(ix + 1, iy + 1);

  return n00 * (1 - ux) * (1 - uy) + n10 * ux * (1 - uy) + n01 * (1 - ux) * uy + n11 * ux * uy;
}

// Fractal Brownian motion
function fbm(x: number, y: number, octaves: number, scale: number, seed: number): number {
  let value = 0;
  let amplitude = 1;
  let frequency = scale;
  let total = 0;
  for (let i = 0; i < octaves; i++) {
    value += valueNoise(x, y, frequency, seed + i * 137) * amplitude;
    total += amplitude;
    amplitude *= 0.5;
    frequency *= 2;
  }
  return value / total;
}

/**
 * Generate a wood grain CanvasTexture.
 * Simulates growth rings with knots and color variation.
 */
export function createWoodTexture(
  width = 512,
  height = 512,
  options: {
    baseColor?: [number, number, number];
    darkColor?: [number, number, number];
    ringScale?: number;
    seed?: number;
  } = {}
): THREE.CanvasTexture {
  const {
    baseColor = [82, 56, 34],   // warm brown
    darkColor = [48, 32, 18],    // dark grain
    ringScale = 20,
    seed = 42,
  } = options;

  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d')!;
  const imageData = ctx.createImageData(width, height);
  const data = imageData.data;

  for (let py = 0; py < height; py++) {
    for (let px = 0; px < width; px++) {
      const u = px / width;
      const v = py / height;

      // Warp coordinates for organic grain
      const warpX = fbm(u, v, 3, 4, seed + 100) * 0.15;
      const warpY = fbm(u, v, 3, 4, seed + 200) * 0.15;

      // Ring pattern along one axis
      const ringVal = (v + warpY) * ringScale + Math.sin((u + warpX) * 15) * 0.5;
      const ring = (Math.sin(ringVal * Math.PI * 2) * 0.5 + 0.5);

      // Fine grain noise
      const grain = fbm(u, v, 4, 30, seed + 300) * 0.3;

      // Knot detail
      const knotX = 0.3 + fbm(0.5, 0.5, 2, 1, seed + 400) * 0.4;
      const knotY = 0.5;
      const knotDist = Math.sqrt((u - knotX) ** 2 + (v - knotY) ** 2);
      const knotRing = Math.sin(knotDist * 60) * 0.5 + 0.5;
      const knotInfluence = Math.max(0, 1 - knotDist * 8) * 0.4;

      const t = Math.min(1, Math.max(0, ring * 0.6 + grain + knotRing * knotInfluence));

      const idx = (py * width + px) * 4;
      data[idx] = baseColor[0] + (darkColor[0] - baseColor[0]) * t;
      data[idx + 1] = baseColor[1] + (darkColor[1] - baseColor[1]) * t;
      data[idx + 2] = baseColor[2] + (darkColor[2] - baseColor[2]) * t;
      data[idx + 3] = 255;
    }
  }

  ctx.putImageData(imageData, 0, 0);
  const texture = new THREE.CanvasTexture(canvas);
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

/**
 * Generate a stone/plaster wall texture.
 * Rough plaster with subtle color variation and cracks.
 */
export function createStoneTexture(
  width = 512,
  height = 512,
  options: {
    baseColor?: [number, number, number];
    accentColor?: [number, number, number];
    roughnessLevel?: number;
    seed?: number;
  } = {}
): THREE.CanvasTexture {
  const {
    baseColor = [105, 92, 78],   // warm stone
    accentColor = [78, 68, 58],  // darker patches
    roughnessLevel = 0.6,
    seed = 77,
  } = options;

  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d')!;
  const imageData = ctx.createImageData(width, height);
  const data = imageData.data;

  for (let py = 0; py < height; py++) {
    for (let px = 0; px < width; px++) {
      const u = px / width;
      const v = py / height;

      // Large-scale variation (plaster patches)
      const large = fbm(u, v, 3, 3, seed);

      // Medium-scale roughness
      const medium = fbm(u, v, 4, 8, seed + 50) * roughnessLevel;

      // Fine grain
      const fine = fbm(u, v, 3, 20, seed + 100) * 0.15;

      // Crack lines (directional noise thresholded)
      const crackNoise = fbm(u, v, 5, 12, seed + 200);
      const crack = crackNoise > 0.72 ? (crackNoise - 0.72) * 4 : 0;

      const t = Math.min(1, Math.max(0, large * 0.4 + medium + fine - crack * 0.3));

      const idx = (py * width + px) * 4;
      data[idx] = baseColor[0] + (accentColor[0] - baseColor[0]) * t;
      data[idx + 1] = baseColor[1] + (accentColor[1] - baseColor[1]) * t;
      data[idx + 2] = baseColor[2] + (accentColor[2] - baseColor[2]) * t;
      data[idx + 3] = 255;
    }
  }

  ctx.putImageData(imageData, 0, 0);
  const texture = new THREE.CanvasTexture(canvas);
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

/**
 * Generate a flagstone floor texture.
 * Irregular stone tiles with mortar lines.
 */
export function createFlagstoneTexture(
  width = 512,
  height = 512,
  options: {
    stoneColor?: [number, number, number];
    mortarColor?: [number, number, number];
    seed?: number;
  } = {}
): THREE.CanvasTexture {
  const {
    stoneColor = [90, 80, 72],
    mortarColor = [55, 48, 42],
    seed = 99,
  } = options;

  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d')!;
  const imageData = ctx.createImageData(width, height);
  const data = imageData.data;

  // Generate Voronoi cell centers for flagstone pattern
  const cellCount = 18;
  const rand = seededRandom(seed);
  const centers: [number, number][] = [];
  for (let i = 0; i < cellCount; i++) {
    centers.push([rand(), rand()]);
  }

  for (let py = 0; py < height; py++) {
    for (let px = 0; px < width; px++) {
      const u = px / width;
      const v = py / height;

      // Find two closest Voronoi centers (wrapping for tileability)
      let d1 = Infinity;
      let d2 = Infinity;
      let closestIdx = 0;

      for (let i = 0; i < cellCount; i++) {
        // Check wrapped distances for tileability
        for (let ox = -1; ox <= 1; ox++) {
          for (let oy = -1; oy <= 1; oy++) {
            const dx = u - (centers[i][0] + ox);
            const dy = v - (centers[i][1] + oy);
            const d = dx * dx + dy * dy;
            if (d < d1) {
              d2 = d1;
              d1 = d;
              closestIdx = i;
            } else if (d < d2) {
              d2 = d;
            }
          }
        }
      }

      // Edge detection: mortar lines where d1 ~= d2
      const edge = Math.sqrt(d2) - Math.sqrt(d1);
      const isMortar = edge < 0.012;

      // Per-stone color variation
      const stoneVariation = ((closestIdx * 7919 + seed) % 100) / 100;
      const noise = fbm(u, v, 3, 15, seed + closestIdx * 13) * 0.2;

      const idx = (py * width + px) * 4;

      if (isMortar) {
        const mortarNoise = fbm(u, v, 2, 30, seed + 500) * 0.15;
        data[idx] = mortarColor[0] + mortarNoise * 20;
        data[idx + 1] = mortarColor[1] + mortarNoise * 15;
        data[idx + 2] = mortarColor[2] + mortarNoise * 10;
      } else {
        const variation = stoneVariation * 0.25 + noise;
        data[idx] = stoneColor[0] + (variation - 0.2) * 40;
        data[idx + 1] = stoneColor[1] + (variation - 0.2) * 35;
        data[idx + 2] = stoneColor[2] + (variation - 0.2) * 30;
      }
      data[idx + 3] = 255;
    }
  }

  ctx.putImageData(imageData, 0, 0);
  const texture = new THREE.CanvasTexture(canvas);
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

/**
 * Generate a fabric/felt texture.
 * Woven fibers with subtle cross-hatch pattern.
 */
export function createFabricTexture(
  width = 256,
  height = 256,
  options: {
    baseColor?: [number, number, number];
    seed?: number;
  } = {}
): THREE.CanvasTexture {
  const {
    baseColor = [42, 58, 42],  // dark green felt
    seed = 55,
  } = options;

  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d')!;
  const imageData = ctx.createImageData(width, height);
  const data = imageData.data;

  for (let py = 0; py < height; py++) {
    for (let px = 0; px < width; px++) {
      const u = px / width;
      const v = py / height;

      // Cross-hatch weave pattern
      const weaveX = Math.sin(u * width * 0.8) * 0.5 + 0.5;
      const weaveY = Math.sin(v * height * 0.8) * 0.5 + 0.5;
      const weave = (weaveX + weaveY) * 0.5;

      // Fiber noise
      const fiber = fbm(u, v, 4, 40, seed) * 0.25;

      // Subtle pile variation
      const pile = fbm(u, v, 2, 6, seed + 300) * 0.1;

      const t = weave * 0.15 + fiber + pile;

      const idx = (py * width + px) * 4;
      data[idx] = Math.min(255, Math.max(0, baseColor[0] + t * 30 - 10));
      data[idx + 1] = Math.min(255, Math.max(0, baseColor[1] + t * 35 - 12));
      data[idx + 2] = Math.min(255, Math.max(0, baseColor[2] + t * 25 - 8));
      data[idx + 3] = 255;
    }
  }

  ctx.putImageData(imageData, 0, 0);
  const texture = new THREE.CanvasTexture(canvas);
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

/**
 * Generate a roughness map from a base texture pattern.
 * Higher roughness in cracks/mortar, lower on polished surfaces.
 */
export function createRoughnessMap(
  width = 256,
  height = 256,
  options: {
    baseRoughness?: number;
    variation?: number;
    seed?: number;
  } = {}
): THREE.CanvasTexture {
  const {
    baseRoughness = 0.8,
    variation = 0.2,
    seed = 123,
  } = options;

  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d')!;
  const imageData = ctx.createImageData(width, height);
  const data = imageData.data;

  for (let py = 0; py < height; py++) {
    for (let px = 0; px < width; px++) {
      const u = px / width;
      const v = py / height;
      const noise = fbm(u, v, 3, 10, seed);
      const roughness = Math.min(1, Math.max(0, baseRoughness + (noise - 0.5) * variation * 2));
      const val = Math.floor(roughness * 255);
      const idx = (py * width + px) * 4;
      data[idx] = val;
      data[idx + 1] = val;
      data[idx + 2] = val;
      data[idx + 3] = 255;
    }
  }

  ctx.putImageData(imageData, 0, 0);
  const texture = new THREE.CanvasTexture(canvas);
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  return texture;
}

// Texture cache to avoid regenerating
const textureCache = new Map<string, THREE.CanvasTexture>();

export function getCachedTexture(key: string, generator: () => THREE.CanvasTexture): THREE.CanvasTexture {
  let tex = textureCache.get(key);
  if (!tex) {
    tex = generator();
    textureCache.set(key, tex);
  }
  return tex;
}

export function disposeProceduralTextures(): void {
  for (const tex of textureCache.values()) {
    tex.dispose();
  }
  textureCache.clear();
}
