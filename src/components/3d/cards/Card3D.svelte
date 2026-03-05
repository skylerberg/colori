<script lang="ts">
  import { T } from '@threlte/core';
  import * as THREE from 'three';
  import { spring } from 'svelte/motion';
  import type { Card, BuyerCard } from '../../../data/types';
  import { getCardTexture, getBuyerTexture, getCardBackTexture, loadCardTexture, loadBuyerTexture } from '../textures/TextureManager';

  let { card, buyerCard, position = [0, 0, 0], rotation = [0, 0, 0], faceUp = true, interactive = false, selected = false, highlighted = false, scale: cardScale = 1, onclick }: {
    card?: Card;
    buyerCard?: BuyerCard;
    position?: [number, number, number];
    rotation?: [number, number, number];
    faceUp?: boolean;
    interactive?: boolean;
    selected?: boolean;
    highlighted?: boolean;
    scale?: number;
    onclick?: () => void;
  } = $props();

  // Card dimensions: 5:7 ratio
  const CARD_WIDTH = 0.5;
  const CARD_HEIGHT = 0.7;
  const CARD_DEPTH = 0.005;
  const CORNER_RADIUS = 0.03;
  const CORNER_SEGMENTS = 6;

  let hovered = $state(false);

  // Spring-animated hover offset
  const hoverY = spring(0, { stiffness: 0.15, damping: 0.7 });
  const hoverScale = spring(1, { stiffness: 0.15, damping: 0.7 });

  $effect(() => {
    if (selected) {
      hoverY.set(0.25);
      hoverScale.set(1.2);
    } else if (hovered && interactive) {
      hoverY.set(0.15);
      hoverScale.set(1.1);
    } else {
      hoverY.set(0);
      hoverScale.set(1);
    }
  });

  // Get textures — use $state so async loads trigger re-render
  let frontTexture = $state<THREE.Texture | null>(null);
  let frontMaterialRef = $state<THREE.MeshStandardMaterial | undefined>();
  let backTexture = $derived(getCardBackTexture());

  $effect(() => {
    if (card) {
      frontTexture = getCardTexture(card);
      loadCardTexture(card).then(tex => { frontTexture = tex; });
    } else if (buyerCard) {
      frontTexture = getBuyerTexture(buyerCard);
      loadBuyerTexture(buyerCard).then(tex => { frontTexture = tex; });
    } else {
      frontTexture = null;
    }
  });

  // Force shader recompile whenever texture changes (Threlte doesn't do this)
  $effect(() => {
    const _tex = frontTexture;
    if (frontMaterialRef) frontMaterialRef.needsUpdate = true;
  });

  // Emissive for selection
  let emissiveColor = $derived(selected ? '#2a6bcf' : highlighted ? '#d4a017' : '#000000');
  let emissiveIntensity = $derived(selected ? 0.3 : highlighted ? 0.2 : 0);

  // Rounded rectangle shape
  function createRoundedRectShape(w: number, h: number, r: number): THREE.Shape {
    const shape = new THREE.Shape();
    const hw = w / 2;
    const hh = h / 2;
    shape.moveTo(-hw + r, -hh);
    shape.lineTo(hw - r, -hh);
    shape.quadraticCurveTo(hw, -hh, hw, -hh + r);
    shape.lineTo(hw, hh - r);
    shape.quadraticCurveTo(hw, hh, hw - r, hh);
    shape.lineTo(-hw + r, hh);
    shape.quadraticCurveTo(-hw, hh, -hw, hh - r);
    shape.lineTo(-hw, -hh + r);
    shape.quadraticCurveTo(-hw, -hh, -hw + r, -hh);
    return shape;
  }

  // Create rounded card geometry (extruded for thickness)
  let cardGeometry = $derived.by(() => {
    const shape = createRoundedRectShape(CARD_WIDTH, CARD_HEIGHT, CORNER_RADIUS);
    const geom = new THREE.ExtrudeGeometry(shape, {
      depth: CARD_DEPTH,
      bevelEnabled: false,
      curveSegments: CORNER_SEGMENTS,
    });
    // Center the geometry on z-axis
    geom.translate(0, 0, -CARD_DEPTH / 2);
    return geom;
  });

  // Normalize ShapeGeometry UVs from vertex coords to 0-1 range
  function normalizeUVs(geom: THREE.ShapeGeometry, width: number, height: number) {
    const uvAttr = geom.attributes.uv;
    for (let i = 0; i < uvAttr.count; i++) {
      // Vertices go from -width/2..width/2, -height/2..height/2
      // Map to 0..1
      uvAttr.setX(i, uvAttr.getX(i) / width + 0.5);
      uvAttr.setY(i, uvAttr.getY(i) / height + 0.5);
    }
    uvAttr.needsUpdate = true;
  }

  // Front face plane geometry (for art texture, slightly inset to prevent z-fighting)
  const FRONT_W = CARD_WIDTH - 0.01;
  const FRONT_H = CARD_HEIGHT - 0.01;
  let frontGeometry = $derived.by(() => {
    const shape = createRoundedRectShape(FRONT_W, FRONT_H, CORNER_RADIUS - 0.005);
    const geom = new THREE.ShapeGeometry(shape, CORNER_SEGMENTS);
    normalizeUVs(geom, FRONT_W, FRONT_H);
    return geom;
  });

  // Back face plane geometry
  let backGeometry = $derived.by(() => {
    const shape = createRoundedRectShape(FRONT_W, FRONT_H, CORNER_RADIUS - 0.005);
    const geom = new THREE.ShapeGeometry(shape, CORNER_SEGMENTS);
    normalizeUVs(geom, FRONT_W, FRONT_H);
    return geom;
  });

  // Shadow plane (slightly larger, under the card)
  let shadowGeometry = $derived.by(() => {
    const shape = createRoundedRectShape(CARD_WIDTH + 0.02, CARD_HEIGHT + 0.02, CORNER_RADIUS + 0.01);
    return new THREE.ShapeGeometry(shape, CORNER_SEGMENTS);
  });

  function handlePointerEnter() {
    if (interactive) {
      hovered = true;
      document.body.style.cursor = 'pointer';
    }
  }

  function handlePointerLeave() {
    hovered = false;
    document.body.style.cursor = 'default';
  }

  function handleClick() {
    if (interactive && onclick) {
      onclick();
    }
  }
</script>

<T.Group
  position={[position[0], position[1] + $hoverY, position[2]]}
  rotation={[rotation[0], rotation[1], rotation[2]]}
  scale={cardScale * $hoverScale}
>
  <!-- Card body (rounded extruded shape for edges) -->
  <T.Mesh
    geometry={cardGeometry}
    castShadow
  >
    <T.MeshStandardMaterial color="#fffef0" roughness={0.85} metalness={0.02} />
  </T.Mesh>

  <!-- Card front face (textured art) -->
  <T.Mesh
    geometry={frontGeometry}
    position.z={CARD_DEPTH / 2 + 0.001}
    onpointerenter={handlePointerEnter}
    onpointerleave={handlePointerLeave}
    onclick={handleClick}
  >
    <T.MeshStandardMaterial
      bind:ref={frontMaterialRef}
      map={faceUp ? frontTexture : null}
      color={frontTexture && faceUp ? '#ffffff' : '#3a2a1a'}
      roughness={0.7}
      metalness={0.05}
      emissive={emissiveColor}
      emissiveIntensity={emissiveIntensity}
    />
  </T.Mesh>

  <!-- Card back face (Renaissance pattern) -->
  <T.Mesh
    geometry={backGeometry}
    position.z={-CARD_DEPTH / 2 - 0.001}
    rotation.y={Math.PI}
  >
    {#if backTexture}
      <T.MeshStandardMaterial
        map={backTexture}
        roughness={0.8}
        metalness={0.05}
      />
    {:else}
      <T.MeshStandardMaterial color="#2a1f18" roughness={0.8} />
    {/if}
  </T.Mesh>

  <!-- Subtle shadow under card -->
  <T.Mesh
    geometry={shadowGeometry}
    position.z={-CARD_DEPTH / 2 - 0.003}
    rotation.y={Math.PI}
  >
    <T.MeshBasicMaterial color="#000000" transparent opacity={0.15} depthWrite={false} />
  </T.Mesh>
</T.Group>
