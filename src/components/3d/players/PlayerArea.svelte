<script lang="ts">
  import { T } from '@threlte/core';
  import type { PlayerState } from '../../../data/types';

  let { player, playerName, position = [0, 0.07, 1.5] }: {
    player: PlayerState;
    playerName: string;
    position?: [number, number, number];
  } = $props();
</script>

<!-- Player area marker (subtle) -->
<T.Group position={position}>
  <!-- Placeholder: player area will be enhanced with color wheel, materials display, etc. -->
</T.Group>
