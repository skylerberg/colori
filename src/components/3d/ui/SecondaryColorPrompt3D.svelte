<script lang="ts">
  import type { Choice } from '../../../data/types';
  import { SECONDARIES } from '../../../data/cards';
  import ColorPicker3D from './ColorPicker3D.svelte';

  let { onAction }: {
    onAction: (choice: Choice) => void;
  } = $props();
</script>

<ColorPicker3D
  colors={SECONDARIES}
  onColorClick={(color) => onAction({ type: 'gainSecondary', color })}
  position={[0, 1.2, 2.0]}
  label="Choose a secondary color"
/>
