<script lang="ts">
  import { T } from '@threlte/core';
  import { Text } from '@threlte/extras';
  import type { Color } from '../../../data/types';
  import { colorToHex } from '../../../data/colors';

  let { colors, onColorClick, position = [0, 0, 0], label = '' }: {
    colors: Color[];
    onColorClick: (color: Color) => void;
    position?: [number, number, number];
    label?: string;
  } = $props();

  let hoveredIndex = $state<number | null>(null);
</script>

<T.Group position={position}>
  <!-- Header label -->
  {#if label}
    <Text
      text={label}
      position={[0, 0.25, 0]}
      fontSize={0.06}
      color="#4a3a2a"
      anchorX="center"
      anchorY="middle"
      outlineWidth={0.003}
      outlineColor="#ffffff"
      fontWeight="bold"
    />
  {/if}

  <!-- Colored spheres in an arc -->
  {#each colors as color, i}
    {@const angle = ((i - (colors.length - 1) / 2) * 0.3)}
    {@const x = Math.sin(angle) * 0.4}
    {@const z = -Math.cos(angle) * 0.4 + 0.4}
    {@const isHovered = hoveredIndex === i}

    <T.Group position={[x, 0, z]}>
      <!-- Sphere -->
      <T.Mesh
        onclick={() => onColorClick(color)}
        onpointerenter={() => { hoveredIndex = i; document.body.style.cursor = 'pointer'; }}
        onpointerleave={() => { hoveredIndex = null; document.body.style.cursor = 'auto'; }}
      >
        <T.SphereGeometry args={[0.06, 16, 16]} />
        <T.MeshStandardMaterial
          color={colorToHex(color)}
          roughness={0.3}
          metalness={0.3}
          emissive={colorToHex(color)}
          emissiveIntensity={isHovered ? 0.6 : 0.15}
        />
      </T.Mesh>
      <!-- Color name label above -->
      <Text
        text={color}
        position={[0, 0.1, 0]}
        fontSize={0.035}
        color={isHovered ? '#2a1a0a' : '#4a3a2a'}
        anchorX="center"
        anchorY="middle"
        outlineWidth={0.002}
        outlineColor="#000000"
        fontWeight="bold"
      />
    </T.Group>
  {/each}
</T.Group>
