<script lang="ts">
  import { T } from '@threlte/core';
  import { Text } from '@threlte/extras';
  import type { Color, Choice } from '../../../data/types';
  import { TERTIARIES } from '../../../data/cards';
  import ColorPicker3D from './ColorPicker3D.svelte';

  let { colorWheel, onAction }: {
    colorWheel: Record<Color, number>;
    onAction: (choice: Choice) => void;
  } = $props();

  let selectedLoseColor = $state<Color | null>(null);

  // Reset when colorWheel changes
  $effect(() => {
    const _cw = colorWheel;
    selectedLoseColor = null;
  });

  let ownedTertiaries = $derived(TERTIARIES.filter(c => colorWheel[c] > 0));
  let gainOptions = $derived(TERTIARIES.filter(c => c !== selectedLoseColor));

  // Back button hover state
  let backHovered = $state(false);
</script>

{#if selectedLoseColor === null}
  <!-- Step 1: Choose tertiary to lose -->
  <ColorPicker3D
    colors={ownedTertiaries}
    onColorClick={(color) => { selectedLoseColor = color; }}
    position={[0, 1.2, 2.0]}
    label="Choose a tertiary to lose"
  />
{:else}
  <!-- Step 2: Choose tertiary to gain -->
  <ColorPicker3D
    colors={gainOptions}
    onColorClick={(color) => onAction({ type: 'swapTertiary', loseColor: selectedLoseColor!, gainColor: color })}
    position={[0, 1.2, 2.0]}
    label={`Losing ${selectedLoseColor} — choose to gain`}
  />

  <!-- Back button -->
  <T.Group position={[-0.5, 1.2, 2.4]}>
    <T.Mesh
      onclick={() => { selectedLoseColor = null; }}
      onpointerenter={() => { backHovered = true; document.body.style.cursor = 'pointer'; }}
      onpointerleave={() => { backHovered = false; document.body.style.cursor = 'auto'; }}
    >
      <T.BoxGeometry args={[0.25, 0.08, 0.1]} />
      <T.MeshStandardMaterial
        color={backHovered ? '#666' : '#444'}
        roughness={0.5}
        emissive={backHovered ? '#666' : '#000000'}
        emissiveIntensity={backHovered ? 0.2 : 0}
      />
    </T.Mesh>
    <Text
      text="Back"
      position={[0, 0.05, 0]}
      fontSize={0.04}
      color="#ffffff"
      anchorX="center"
      anchorY="middle"
      fontWeight="bold"
    />
  </T.Group>
{/if}
