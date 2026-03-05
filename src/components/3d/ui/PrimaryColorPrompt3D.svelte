<script lang="ts">
  import type { Choice } from '../../../data/types';
  import { PRIMARIES } from '../../../data/cards';
  import ColorPicker3D from './ColorPicker3D.svelte';

  let { onAction }: {
    onAction: (choice: Choice) => void;
  } = $props();
</script>

<ColorPicker3D
  colors={PRIMARIES}
  onColorClick={(color) => onAction({ type: 'gainPrimary', color })}
  position={[0, 1.2, 2.0]}
  label="Choose a primary color"
/>
