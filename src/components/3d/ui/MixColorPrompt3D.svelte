<script lang="ts">
  import { T } from '@threlte/core';
  import { Text } from '@threlte/extras';
  import type { Color, Choice } from '../../../data/types';
  import { canMix, mixResult, ALL_COLORS } from '../../../data/colors';

  let { colorWheel, remaining, onAction, playerAreaRef }: {
    colorWheel: Record<Color, number>;
    remaining: number;
    onAction: (choice: Choice) => void;
    playerAreaRef?: { setMixMode: (active: boolean, selected: Color[], simWheel: Record<Color, number>, onClick: (c: Color) => void) => void };
  } = $props();

  let plannedMixes: [Color, Color][] = $state([]);
  let selectedMixColors: Color[] = $state([]);

  let simulatedWheel: Record<Color, number> = $state(
    Object.fromEntries(ALL_COLORS.map(c => [c, 0])) as Record<Color, number>
  );

  // Sync simulatedWheel when colorWheel changes
  $effect(() => {
    simulatedWheel = { ...colorWheel };
    plannedMixes = [];
    selectedMixColors = [];
  });

  let mixRemaining = $derived(remaining - plannedMixes.length);

  function handleMixColorClick(color: Color) {
    if (selectedMixColors.length === 0) {
      selectedMixColors = [color];
    } else if (selectedMixColors.length === 1) {
      const first = selectedMixColors[0];
      if (first === color) {
        selectedMixColors = [];
      } else if (canMix(first, color) && simulatedWheel[first] > 0 && simulatedWheel[color] > 0) {
        const result = mixResult(first, color);
        const newWheel = { ...simulatedWheel };
        newWheel[first]--;
        newWheel[color]--;
        newWheel[result]++;
        simulatedWheel = newWheel;
        plannedMixes = [...plannedMixes, [first, color]];
        selectedMixColors = [];

        // Auto-submit if all mixes used
        if (plannedMixes.length === remaining) {
          onAction({ type: 'mixAll', mixes: plannedMixes });
        }
      } else {
        selectedMixColors = [color];
      }
    }
  }

  function handleSkipMix() {
    onAction({ type: 'mixAll', mixes: plannedMixes });
  }

  function handleUndoMix() {
    if (plannedMixes.length === 0) return;
    const newWheel = { ...colorWheel };
    const newMixes = plannedMixes.slice(0, -1);
    for (const [a, b] of newMixes) {
      const result = mixResult(a, b);
      newWheel[a]--;
      newWheel[b]--;
      newWheel[result]++;
    }
    simulatedWheel = newWheel;
    plannedMixes = newMixes;
    selectedMixColors = [];
  }

  // Button hover states
  let submitHovered = $state(false);
  let undoHovered = $state(false);
</script>

<!-- Header text -->
<Text
  text={`Mix Colors (${mixRemaining} remaining)`}
  position={[0, 1.5, 2.0]}
  fontSize={0.07}
  color="#4a3a2a"
  anchorX="center"
  anchorY="middle"
  outlineWidth={0.003}
  outlineColor="#ffffff"
  fontWeight="bold"
/>

<!-- Hint text -->
<Text
  text="Click two colors on the wheel to mix"
  position={[0, 1.38, 2.0]}
  fontSize={0.04}
  color="#7a6a5a"
  anchorX="center"
  anchorY="middle"
/>

<!-- Submit / Skip button -->
<T.Group position={[0.4, 1.2, 2.4]}>
  <T.Mesh
    onclick={handleSkipMix}
    onpointerenter={() => { submitHovered = true; document.body.style.cursor = 'pointer'; }}
    onpointerleave={() => { submitHovered = false; document.body.style.cursor = 'auto'; }}
  >
    <T.BoxGeometry args={[0.4, 0.1, 0.12]} />
    <T.MeshStandardMaterial
      color={submitHovered ? '#3cb525' : '#2a6b14'}
      roughness={0.5}
      emissive={submitHovered ? '#3cb525' : '#000000'}
      emissiveIntensity={submitHovered ? 0.3 : 0}
    />
  </T.Mesh>
  <Text
    text={plannedMixes.length > 0 ? 'Submit' : 'Skip'}
    position={[0, 0.06, 0]}
    fontSize={0.04}
    color="#ffffff"
    anchorX="center"
    anchorY="middle"
    fontWeight="bold"
  />
</T.Group>

<!-- Undo button (only if mixes planned) -->
{#if plannedMixes.length > 0}
  <T.Group position={[-0.4, 1.2, 2.4]}>
    <T.Mesh
      onclick={handleUndoMix}
      onpointerenter={() => { undoHovered = true; document.body.style.cursor = 'pointer'; }}
      onpointerleave={() => { undoHovered = false; document.body.style.cursor = 'auto'; }}
    >
      <T.BoxGeometry args={[0.3, 0.1, 0.12]} />
      <T.MeshStandardMaterial
        color={undoHovered ? '#888' : '#555'}
        roughness={0.5}
        emissive={undoHovered ? '#888' : '#000000'}
        emissiveIntensity={undoHovered ? 0.2 : 0}
      />
    </T.Mesh>
    <Text
      text="Undo"
      position={[0, 0.06, 0]}
      fontSize={0.04}
      color="#ffffff"
      anchorX="center"
      anchorY="middle"
      fontWeight="bold"
    />
  </T.Group>
{/if}
