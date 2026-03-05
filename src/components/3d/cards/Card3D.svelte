<script lang="ts">
  import { T, useTask } from '@threlte/core';
  import * as THREE from 'three';
  import { spring } from 'svelte/motion';
  import type { Card, BuyerCard } from '../../../data/types';
  import { getCardTexture, getBuyerTexture } from '../textures/TextureManager';

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

  // Get texture
  let frontTexture = $derived(
    card ? getCardTexture(card) :
    buyerCard ? getBuyerTexture(buyerCard) :
    null
  );

  // Emissive for selection
  let emissiveColor = $derived(selected ? '#2a6bcf' : highlighted ? '#d4a017' : '#000000');
  let emissiveIntensity = $derived(selected ? 0.3 : highlighted ? 0.2 : 0);

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
  <!-- Card front face -->
  <T.Mesh
    position.z={CARD_DEPTH / 2 + 0.001}
    onpointerenter={handlePointerEnter}
    onpointerleave={handlePointerLeave}
    onclick={handleClick}
    castShadow
  >
    <T.PlaneGeometry args={[CARD_WIDTH, CARD_HEIGHT]} />
    {#if frontTexture && faceUp}
      <T.MeshStandardMaterial
        map={frontTexture}
        roughness={0.7}
        metalness={0.05}
        emissive={emissiveColor}
        emissiveIntensity={emissiveIntensity}
      />
    {:else}
      <T.MeshStandardMaterial
        color="#3a2a1a"
        roughness={0.7}
        emissive={emissiveColor}
        emissiveIntensity={emissiveIntensity}
      />
    {/if}
  </T.Mesh>

  <!-- Card back face -->
  <T.Mesh position.z={-CARD_DEPTH / 2 - 0.001} rotation.y={Math.PI}>
    <T.PlaneGeometry args={[CARD_WIDTH, CARD_HEIGHT]} />
    <T.MeshStandardMaterial color="#2a1f18" roughness={0.8} />
  </T.Mesh>

  <!-- Card edge (thin box for thickness) -->
  <T.Mesh castShadow>
    <T.BoxGeometry args={[CARD_WIDTH, CARD_HEIGHT, CARD_DEPTH]} />
    <T.MeshStandardMaterial color="#fffef7" roughness={0.9} />
  </T.Mesh>
</T.Group>
