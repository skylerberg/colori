<script lang="ts">
  import { T } from '@threlte/core';
  import { Text } from '@threlte/extras';
  import type { Color } from '../../../data/types';
  import { colorToHex } from '../../../data/colors';

  let { color, count, position = [0, 0, 0] }: {
    color: Color;
    count: number;
    position?: [number, number, number];
  } = $props();

  const PRIMARY: Color[] = ['Red', 'Yellow', 'Blue'];
  const SECONDARY: Color[] = ['Orange', 'Green', 'Purple'];

  type Tier = 'primary' | 'secondary' | 'tertiary';

  function getTier(c: Color): Tier {
    if (PRIMARY.includes(c)) return 'primary';
    if (SECONDARY.includes(c)) return 'secondary';
    return 'tertiary';
  }

  let tier = $derived(getTier(color));
  let hex = $derived(colorToHex(color));
  let isEmpty = $derived(count === 0);

  const WOOD_COLOR = '#b8956a';
  const CORK_COLOR = '#8b6914';
</script>

<T.Group position={position}>
  {#if isEmpty}
    <!-- Empty: clear glass bottle -->
    <T.Group>
      <!-- Bottle body -->
      <T.Mesh position={[0, 0.03, 0]} castShadow>
        <T.CylinderGeometry args={[0.018, 0.02, 0.05, 12]} />
        <T.MeshPhysicalMaterial
          color="#d4e8f0"
          roughness={0.1}
          metalness={0.2}
          opacity={0.25}
          transparent={true}
        />
      </T.Mesh>
      <!-- Bottle neck -->
      <T.Mesh position={[0, 0.065, 0]} castShadow>
        <T.CylinderGeometry args={[0.008, 0.012, 0.02, 8]} />
        <T.MeshPhysicalMaterial
          color="#d4e8f0"
          roughness={0.1}
          metalness={0.2}
          opacity={0.25}
          transparent={true}
        />
      </T.Mesh>
      <!-- Cork stopper -->
      <T.Mesh position={[0, 0.08, 0]}>
        <T.CylinderGeometry args={[0.009, 0.008, 0.012, 8]} />
        <T.MeshStandardMaterial color={CORK_COLOR} roughness={0.9} metalness={0.0} />
      </T.Mesh>
    </T.Group>

  {:else if tier === 'primary'}
    <!-- Primary: Wooden bowl with colored powder mound -->
    <T.Group>
      <!-- Bowl (wooden hemisphere, open top) -->
      <T.Mesh position={[0, 0.0, 0]} rotation={[0, 0, 0]} castShadow>
        <T.SphereGeometry args={[0.035, 16, 12, 0, Math.PI * 2, 0, Math.PI / 2]} />
        <T.MeshStandardMaterial color={WOOD_COLOR} roughness={0.7} metalness={0.05} side={2} />
      </T.Mesh>
      <!-- Bowl rim ring -->
      <T.Mesh position={[0, 0.0, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <T.RingGeometry args={[0.033, 0.038, 16]} />
        <T.MeshStandardMaterial color="#a07850" roughness={0.6} metalness={0.05} />
      </T.Mesh>
      <!-- Powder mound inside bowl -->
      <T.Mesh position={[0, 0.005, 0]} castShadow>
        <T.SphereGeometry args={[0.028, 12, 8, 0, Math.PI * 2, 0, Math.PI / 2]} />
        <T.MeshStandardMaterial
          color={hex}
          roughness={0.9}
          metalness={0.0}
          emissive={hex}
          emissiveIntensity={0.1}
        />
      </T.Mesh>
    </T.Group>

  {:else if tier === 'secondary'}
    <!-- Secondary: Colored stone/clay block -->
    <T.Group>
      <T.Mesh position={[0, 0.025, 0]} castShadow>
        <T.BoxGeometry args={[0.045, 0.045, 0.045]} />
        <T.MeshStandardMaterial
          color={hex}
          roughness={0.7}
          metalness={0.05}
          emissive={hex}
          emissiveIntensity={0.1}
        />
      </T.Mesh>
    </T.Group>

  {:else}
    <!-- Tertiary: Glass bottle with colored liquid and cork -->
    <T.Group>
      <!-- Bottle body (glass) -->
      <T.Mesh position={[0, 0.03, 0]} castShadow>
        <T.CylinderGeometry args={[0.018, 0.02, 0.05, 12]} />
        <T.MeshPhysicalMaterial
          color={hex}
          roughness={0.1}
          metalness={0.3}
          opacity={0.3}
          transparent={true}
        />
      </T.Mesh>
      <!-- Liquid inside -->
      <T.Mesh position={[0, 0.025, 0]}>
        <T.CylinderGeometry args={[0.015, 0.017, 0.04, 12]} />
        <T.MeshStandardMaterial
          color={hex}
          roughness={0.3}
          metalness={0.1}
          opacity={0.9}
          transparent={true}
          emissive={hex}
          emissiveIntensity={0.15}
        />
      </T.Mesh>
      <!-- Bottle neck (glass) -->
      <T.Mesh position={[0, 0.065, 0]} castShadow>
        <T.CylinderGeometry args={[0.008, 0.012, 0.02, 8]} />
        <T.MeshPhysicalMaterial
          color={hex}
          roughness={0.1}
          metalness={0.3}
          opacity={0.3}
          transparent={true}
        />
      </T.Mesh>
      <!-- Cork stopper -->
      <T.Mesh position={[0, 0.08, 0]}>
        <T.CylinderGeometry args={[0.009, 0.008, 0.012, 8]} />
        <T.MeshStandardMaterial color={CORK_COLOR} roughness={0.9} metalness={0.0} />
      </T.Mesh>
    </T.Group>
  {/if}

  <!-- Count label floating above (only when count > 0) -->
  {#if count > 0}
    <Text
      text={String(count)}
      position={[0, tier === 'tertiary' || isEmpty ? 0.1 : (tier === 'primary' ? 0.06 : 0.07), 0]}
      fontSize={0.025}
      color={hex}
      anchorX="center"
      anchorY="middle"
      outlineWidth={0.002}
      outlineColor="#000000"
      fontWeight="bold"
    />
  {/if}
</T.Group>
